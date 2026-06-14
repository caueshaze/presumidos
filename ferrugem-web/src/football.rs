//! Integração de resultados ao vivo via API pública da ESPN (scoreboard da
//! FIFA World Cup). Endpoint usado:
//! `.../sports/soccer/fifa.world/scoreboard?dates=YYYYMMDD&lang=pt&region=br`.
//!
//! - Poller em background (`spawn_poller`) que, a cada `poll_interval_secs`, só
//!   chama a API quando há jogo na janela.
//! - Comando CLI (`sync_fixtures`) que mapeia cada `jogo-NNN` local ao `id` do
//!   evento da ESPN, casando por par de seleções (nomes em PT, normalizados).
//!
//! Semântica de produto:
//! - O poller **finaliza automaticamente apenas a fase de grupos** (placar +
//!   `finished`). No mata-mata ele só exibe o placar ao vivo; o resultado oficial
//!   (classificado/pênaltis) é lançado pelo admin.
//! - Resultado de origem `manual` (admin) é soberano: nunca é sobrescrito.
//!
//! Detalhe de data: a ESPN agrupa o `dates=YYYYMMDD` por horário ET (EDT durante
//! a Copa, UTC−4). Por isso a data consultada é o kickoff convertido para ET.

#![cfg(feature = "server")]

use crate::config::settings;
use crate::error::ServerFnError;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Estruturas da resposta do scoreboard da ESPN (apenas o que usamos).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Scoreboard {
    #[serde(default)]
    events: Vec<Event>,
}

#[derive(Debug, Clone, Deserialize)]
struct Event {
    id: String,
    #[serde(default)]
    competitions: Vec<Competition>,
}

#[derive(Debug, Clone, Deserialize)]
struct Competition {
    status: Status,
    #[serde(default)]
    competitors: Vec<Competitor>,
}

#[derive(Debug, Clone, Deserialize)]
struct Status {
    #[serde(default, rename = "displayClock")]
    display_clock: String,
    #[serde(rename = "type")]
    type_: StatusType,
}

#[derive(Debug, Clone, Deserialize)]
struct StatusType {
    /// "pre" | "in" | "post".
    #[serde(default)]
    state: String,
    #[serde(default)]
    completed: bool,
    /// Ex.: "STATUS_FULL_TIME", "STATUS_FIRST_HALF".
    #[serde(default)]
    name: String,
    #[serde(default, rename = "shortDetail")]
    short_detail: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Competitor {
    #[serde(default, rename = "homeAway")]
    home_away: String,
    #[serde(default)]
    score: String,
    team: Team,
}

#[derive(Debug, Clone, Deserialize)]
struct Team {
    #[serde(default, rename = "displayName")]
    display_name: String,
}

fn parse_score(raw: &str) -> i64 {
    raw.trim().parse::<i64>().unwrap_or(0)
}

impl Event {
    fn competition(&self) -> Option<&Competition> {
        self.competitions.first()
    }
}

impl Competition {
    fn side(&self, which: &str) -> Option<&Competitor> {
        self.competitors.iter().find(|c| c.home_away == which)
    }
}

// ---------------------------------------------------------------------------
// Classificação de um evento da ESPN: o que fazer com ele no banco.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum GameApply {
    /// Em andamento — atualizar apenas o placar ao vivo.
    Live {
        home: i64,
        away: i64,
        status: String,
        elapsed: Option<i64>,
    },
    /// Fase de grupos encerrada — gravar o resultado oficial.
    FinalGroup {
        home: i64,
        away: i64,
        raw_status: String,
    },
    /// Mata-mata encerrado — não finaliza (admin lança). Limpa o placar ao vivo.
    FinishedKnockout,
    /// Não começou ou sem dados — ignorar.
    Skip,
}

/// Minuto corrido extraído do relógio da ESPN (ex.: "45'" -> 45).
fn live_elapsed(label: &str) -> Option<i64> {
    let digits: String = label.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<i64>().ok()
}

/// Decide, de forma pura e testável, o que fazer com um evento da ESPN.
fn classify_event(is_knockout: bool, event: &Event) -> GameApply {
    let Some(comp) = event.competition() else {
        return GameApply::Skip;
    };
    let (Some(home), Some(away)) = (comp.side("home"), comp.side("away")) else {
        return GameApply::Skip;
    };
    let home_score = parse_score(&home.score);
    let away_score = parse_score(&away.score);
    let state = comp.status.type_.state.as_str();
    let finished = state == "post" || comp.status.type_.completed;

    if state == "in" && !finished {
        let label = if !comp.status.display_clock.trim().is_empty() {
            comp.status.display_clock.trim().to_string()
        } else {
            comp.status.type_.short_detail.trim().to_string()
        };
        return GameApply::Live {
            home: home_score,
            away: away_score,
            elapsed: live_elapsed(&label),
            status: label,
        };
    }

    if !finished {
        return GameApply::Skip;
    }

    if is_knockout {
        // Sem placar do tempo normal / pênaltis confiáveis aqui: deixa p/ o admin.
        return GameApply::FinishedKnockout;
    }

    GameApply::FinalGroup {
        home: home_score,
        away: away_score,
        raw_status: comp.status.type_.name.clone(),
    }
}

// ---------------------------------------------------------------------------
// Cliente HTTP.
// ---------------------------------------------------------------------------

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Presumidos/1.0")
            .build()
            .expect("falha ao construir cliente HTTP")
    })
}

fn error_chain(err: &dyn std::error::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(inner) = source {
        msg.push_str(" -> ");
        msg.push_str(&inner.to_string());
        source = inner.source();
    }
    msg
}

const FETCH_ATTEMPTS: u32 = 3;

/// Busca o scoreboard da ESPN de uma data (formato YYYYMMDD, em ET).
async fn fetch_scoreboard(date: &str) -> Result<Vec<Event>, ServerFnError> {
    let url = settings().football.base_url.trim_end_matches('/').to_string();

    let mut last_err: Option<reqwest::Error> = None;
    for attempt in 1..=FETCH_ATTEMPTS {
        let send = client()
            .get(&url)
            .query(&[
                ("dates", date),
                ("limit", "100"),
                ("lang", "pt"),
                ("region", "br"),
            ])
            .send()
            .await;

        match send {
            Ok(resp) => {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .map_err(|e| crate::security::internal_error("football_fetch_body", e))?;
                if !status.is_success() {
                    return Err(crate::security::public_error(format!(
                        "ESPN respondeu {status}: {}",
                        body.chars().take(200).collect::<String>()
                    )));
                }
                let parsed: Scoreboard = serde_json::from_str(&body)
                    .map_err(|e| crate::security::internal_error("football_parse", e))?;
                return Ok(parsed.events);
            }
            Err(e) => {
                eprintln!(
                    "[football] tentativa {attempt}/{FETCH_ATTEMPTS} falhou ao buscar ESPN ({date}): {}",
                    error_chain(&e)
                );
                last_err = Some(e);
                if attempt < FETCH_ATTEMPTS {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    }

    Err(crate::security::internal_error(
        "football_fetch_send",
        last_err.expect("last_err preenchido após o loop"),
    ))
}

// ---------------------------------------------------------------------------
// Aplicação no banco.
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct PollCandidate {
    id: String,
    kickoff: String,
    external_fixture_id: Option<i64>,
    phase: Option<String>,
    result_source: Option<String>,
    home_score: Option<i64>,
    away_score: Option<i64>,
}

/// O que o poller fez com um jogo neste ciclo (para o log de heartbeat).
#[derive(Debug, Clone, Copy, PartialEq)]
enum ApplyOutcome {
    Finalized,
    Live,
    Noop,
}

async fn clear_live(db: &sqlx::SqlitePool, match_id: &str) -> Result<(), ServerFnError> {
    sqlx::query(
        "UPDATE matches SET live_home_score = NULL, live_away_score = NULL,
            live_status = NULL, live_elapsed = NULL, live_updated_at = NULL
         WHERE id = ?1",
    )
    .bind(match_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("football_clear_live", e))?;
    Ok(())
}

/// Aplica um evento da ESPN a um jogo local já mapeado.
async fn apply_event(
    db: &sqlx::SqlitePool,
    candidate: &PollCandidate,
    event: &Event,
) -> Result<ApplyOutcome, ServerFnError> {
    use crate::models::is_knockout;
    use serde_json::json;

    let is_ko = is_knockout(candidate.phase.as_deref());
    match classify_event(is_ko, event) {
        GameApply::Skip => Ok(ApplyOutcome::Noop),

        GameApply::FinishedKnockout => {
            clear_live(db, &candidate.id).await?;
            Ok(ApplyOutcome::Noop)
        }

        GameApply::Live {
            home,
            away,
            status,
            elapsed,
        } => {
            sqlx::query(
                "UPDATE matches SET
                    live_home_score = ?1, live_away_score = ?2,
                    live_status = ?3, live_elapsed = ?4,
                    live_updated_at = datetime('now')
                 WHERE id = ?5 AND finished = 0",
            )
            .bind(home)
            .bind(away)
            .bind(&status)
            .bind(elapsed)
            .bind(&candidate.id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("football_live_update", e))?;
            Ok(ApplyOutcome::Live)
        }

        GameApply::FinalGroup {
            home,
            away,
            raw_status,
        } => {
            // Resultado manual é soberano: não sobrescreve, só registra conflito.
            if candidate.result_source.as_deref() == Some("manual") {
                if candidate.home_score != Some(home) || candidate.away_score != Some(away) {
                    crate::security::append_audit_log(
                        db,
                        None,
                        "match_result_api_conflict",
                        "match",
                        Some(&candidate.id),
                        None,
                        json!({
                            "manual": { "home": candidate.home_score, "away": candidate.away_score },
                            "api": { "home": home, "away": away }
                        }),
                    )
                    .await?;
                }
                return Ok(ApplyOutcome::Noop);
            }

            let already = candidate.home_score == Some(home)
                && candidate.away_score == Some(away)
                && candidate.result_source.as_deref() == Some("api");

            sqlx::query(
                "UPDATE matches SET
                    home_score = ?1, away_score = ?2,
                    finished = 1,
                    result_source = 'api',
                    result_synced_at = datetime('now'),
                    result_external_raw_status = ?3,
                    live_home_score = NULL, live_away_score = NULL,
                    live_status = NULL, live_elapsed = NULL, live_updated_at = NULL
                 WHERE id = ?4",
            )
            .bind(home)
            .bind(away)
            .bind(&raw_status)
            .bind(&candidate.id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("football_final_update", e))?;

            if !already {
                let action = if candidate.result_source.as_deref() == Some("api") {
                    "match_result_api_corrected"
                } else {
                    "match_result_autofilled"
                };
                crate::security::append_audit_log(
                    db,
                    None,
                    action,
                    "match",
                    Some(&candidate.id),
                    None,
                    json!({ "home_score": home, "away_score": away }),
                )
                .await?;
            }
            Ok(if already {
                ApplyOutcome::Noop
            } else {
                ApplyOutcome::Finalized
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Datas: a ESPN agrupa por dia ET (EDT na Copa, UTC−4).
// ---------------------------------------------------------------------------

/// Data ET (YYYYMMDD) de um kickoff em UTC. EDT = UTC−4 durante toda a Copa.
fn et_date(kickoff: &str) -> Option<String> {
    let dt = chrono::DateTime::parse_from_rfc3339(kickoff).ok()?;
    let et = dt.with_timezone(&chrono::Utc) - chrono::Duration::hours(4);
    Some(et.format("%Y%m%d").to_string())
}

fn distinct_et_dates(kickoffs: impl Iterator<Item = String>) -> Vec<String> {
    let mut dates: Vec<String> = kickoffs.filter_map(|k| et_date(&k)).collect();
    dates.sort();
    dates.dedup();
    dates
}

// ---------------------------------------------------------------------------
// Poller em background.
// ---------------------------------------------------------------------------

/// Sobe a task de polling. Só deve ser chamada se a integração e o poller
/// estiverem habilitados (ver [crate::config::FootballConfig]).
pub fn spawn_poller() {
    use rand_core::{OsRng, RngCore};

    let interval_secs = settings().football.poll_interval_secs;
    tokio::spawn(async move {
        eprintln!(
            "[football] poller iniciado (intervalo {interval_secs}s + jitter 0–60s, fonte ESPN)"
        );
        loop {
            if let Err(e) = run_poll_cycle().await {
                eprintln!("[football] ciclo falhou: {e:?}");
            }
            // Jitter de 0–60s sobre o intervalo base: evita bater sempre cravado
            // no mesmo segundo (ex.: :00, :10, :20) e espalha as requisições.
            let jitter = u64::from(OsRng.next_u32() % 61);
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs + jitter)).await;
        }
    });
}

async fn load_candidates(db: &sqlx::SqlitePool) -> Result<Vec<PollCandidate>, ServerFnError> {
    sqlx::query_as::<_, PollCandidate>(
        "SELECT id, kickoff, external_fixture_id, phase, result_source, home_score, away_score
         FROM matches
         WHERE finished = 0
           AND external_fixture_id IS NOT NULL
           AND datetime(kickoff) BETWEEN datetime('now','-4 hours') AND datetime('now','+30 minutes')",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("football_load_candidates", e))
}

async fn run_poll_cycle() -> Result<(), ServerFnError> {
    let db = crate::db::pool();
    let candidates = load_candidates(db).await?;
    if candidates.is_empty() {
        eprintln!("[football] ciclo: nenhum jogo na janela");
        return Ok(());
    }

    // Busca os scoreboards das datas ET envolvidas (normalmente 1, às vezes 2).
    let dates = distinct_et_dates(candidates.iter().map(|c| c.kickoff.clone()));
    let mut by_id: HashMap<String, Event> = HashMap::new();
    for date in &dates {
        for ev in fetch_scoreboard(date).await? {
            by_id.insert(ev.id.clone(), ev);
        }
    }

    let (mut finalized, mut live) = (0u32, 0u32);
    for candidate in &candidates {
        let key = candidate.external_fixture_id.map(|id| id.to_string());
        if let Some(event) = key.as_deref().and_then(|k| by_id.get(k)) {
            match apply_event(db, candidate, event).await? {
                ApplyOutcome::Finalized => finalized += 1,
                ApplyOutcome::Live => live += 1,
                ApplyOutcome::Noop => {}
            }
        }
    }

    eprintln!(
        "[football] ciclo: {} jogo(s) na janela, {finalized} finalizado(s), {live} ao vivo",
        candidates.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Comando CLI: mapeamento jogo local <-> id do evento da ESPN.
// ---------------------------------------------------------------------------

pub enum SyncMode {
    DryRun,
    Apply,
    Override { match_id: String, fixture_id: i64 },
}

#[derive(sqlx::FromRow)]
struct LocalMatch {
    id: String,
    home_team: String,
    away_team: String,
    kickoff: String,
}

/// Remove acentos e normaliza para comparação de nomes de seleção.
fn normalize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.trim().to_lowercase().chars() {
        let mapped = match ch {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            '\'' | '.' => continue,
            other => other,
        };
        out.push(mapped);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Token canônico por seleção, cobrindo o nome local (PT) e o da ESPN (PT, com
/// algumas variações: Catar/Qatar, Holanda/Países Baixos, etc.).
fn canonical_team(name: &str) -> String {
    static TABLE: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    let table = TABLE.get_or_init(build_team_table);
    let norm = normalize_name(name);
    table.get(&norm).map(|c| c.to_string()).unwrap_or(norm)
}

fn build_team_table() -> HashMap<String, &'static str> {
    let entries: &[(&str, &[&str])] = &[
        ("south-africa", &["África do Sul", "South Africa"]),
        ("germany", &["Alemanha", "Germany"]),
        ("saudi-arabia", &["Arábia Saudita", "Saudi Arabia"]),
        ("algeria", &["Argélia", "Algeria"]),
        ("argentina", &["Argentina"]),
        ("australia", &["Austrália", "Australia"]),
        ("austria", &["Áustria", "Austria"]),
        ("belgium", &["Bélgica", "Belgium"]),
        ("bosnia", &["Bósnia e Herzegovina", "Bosnia and Herzegovina"]),
        ("brazil", &["Brasil", "Brazil"]),
        ("cape-verde", &["Cabo Verde", "Cape Verde"]),
        ("canada", &["Canadá", "Canada"]),
        ("colombia", &["Colômbia", "Colombia"]),
        ("south-korea", &["Coreia do Sul", "South Korea", "Korea Republic"]),
        ("ivory-coast", &["Costa do Marfim", "Ivory Coast", "Côte d'Ivoire"]),
        ("croatia", &["Croácia", "Croatia"]),
        ("curacao", &["Curaçao", "Curacao"]),
        ("egypt", &["Egito", "Egypt"]),
        ("ecuador", &["Equador", "Ecuador"]),
        ("scotland", &["Escócia", "Scotland"]),
        ("spain", &["Espanha", "Spain"]),
        ("usa", &["Estados Unidos", "USA", "United States"]),
        ("france", &["França", "France"]),
        ("ghana", &["Gana", "Ghana"]),
        ("haiti", &["Haiti"]),
        ("england", &["Inglaterra", "England"]),
        ("iran", &["Irã", "Iran"]),
        ("iraq", &["Iraque", "Iraq"]),
        ("japan", &["Japão", "Japan"]),
        ("jordan", &["Jordânia", "Jordan"]),
        ("morocco", &["Marrocos", "Morocco"]),
        ("mexico", &["México", "Mexico"]),
        ("norway", &["Noruega", "Norway"]),
        ("new-zealand", &["Nova Zelândia", "New Zealand"]),
        ("netherlands", &["Países Baixos", "Holanda", "Netherlands"]),
        ("panama", &["Panamá", "Panama"]),
        ("paraguay", &["Paraguai", "Paraguay"]),
        ("portugal", &["Portugal"]),
        ("qatar", &["Qatar", "Catar"]),
        ("dr-congo", &[
            "RD Congo",
            "DR Congo",
            "Congo DR",
            "República Democrática do Congo",
        ]),
        ("senegal", &["Senegal"]),
        ("sweden", &["Suécia", "Sweden"]),
        ("switzerland", &["Suíça", "Switzerland"]),
        ("czechia", &["Tchéquia", "República Tcheca", "Czech Republic", "Czechia"]),
        ("tunisia", &["Tunísia", "Tunisia"]),
        ("turkey", &["Turquia", "Turkey", "Türkiye"]),
        ("uruguay", &["Uruguai", "Uruguay"]),
        ("uzbekistan", &["Uzbequistão", "Uzbekistan"]),
    ];

    let mut table = HashMap::new();
    for (canon, names) in entries {
        for name in *names {
            table.insert(normalize_name(name), *canon);
        }
    }
    table
}

async fn set_mapping(
    db: &sqlx::SqlitePool,
    match_id: &str,
    fixture_id: i64,
) -> Result<bool, ServerFnError> {
    let result = sqlx::query("UPDATE matches SET external_fixture_id = ?1 WHERE id = ?2")
        .bind(fixture_id)
        .bind(match_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("football_set_mapping", e))?;

    if result.rows_affected() == 0 {
        return Ok(false);
    }

    crate::security::append_audit_log(
        db,
        None,
        "external_fixture_mapped",
        "match",
        Some(match_id),
        None,
        serde_json::json!({ "external_fixture_id": fixture_id }),
    )
    .await?;
    Ok(true)
}

/// Mapeia os jogos locais aos ids de evento da ESPN, casando por par de times.
pub async fn sync_fixtures(mode: SyncMode) -> Result<(), ServerFnError> {
    let db = crate::db::pool();

    if let SyncMode::Override {
        match_id,
        fixture_id,
    } = &mode
    {
        if set_mapping(db, match_id, *fixture_id).await? {
            println!("Mapeado {match_id} -> evento {fixture_id}");
        } else {
            println!("Jogo {match_id} não encontrado.");
        }
        return Ok(());
    }

    let apply = matches!(mode, SyncMode::Apply);

    let locals: Vec<LocalMatch> =
        sqlx::query_as("SELECT id, home_team, away_team, kickoff FROM matches")
            .fetch_all(db)
            .await
            .map_err(|e| crate::security::internal_error("football_sync_locals", e))?;

    // Busca os scoreboards de todas as datas ET dos jogos locais e indexa por par.
    let dates = distinct_et_dates(locals.iter().map(|m| m.kickoff.clone()));
    println!("Consultando {} data(s) na ESPN...", dates.len());
    let mut by_pair: HashMap<(String, String), i64> = HashMap::new();
    for date in &dates {
        for ev in fetch_scoreboard(date).await? {
            let Some(comp) = ev.competition() else { continue };
            let (Some(home), Some(away)) = (comp.side("home"), comp.side("away")) else {
                continue;
            };
            if let Ok(id) = ev.id.parse::<i64>() {
                by_pair.insert(
                    (
                        canonical_team(&home.team.display_name),
                        canonical_team(&away.team.display_name),
                    ),
                    id,
                );
            }
        }
    }
    println!("ESPN retornou {} confronto(s).", by_pair.len());

    let mut matched = 0usize;
    let mut unmatched: Vec<String> = Vec::new();
    for local in &locals {
        let key = (
            canonical_team(&local.home_team),
            canonical_team(&local.away_team),
        );
        match by_pair.get(&key) {
            Some(&id) => {
                matched += 1;
                if apply {
                    set_mapping(db, &local.id, id).await?;
                }
            }
            None => unmatched.push(format!(
                "  {} : {} x {}",
                local.id, local.home_team, local.away_team
            )),
        }
    }

    println!("\nMapeados: {matched}. Não casados: {}.", unmatched.len());
    if !unmatched.is_empty() {
        println!("Não casados (mata-mata sem times definidos ou nome novo — use --fixture jogo-XXX=ID):");
        for line in &unmatched {
            println!("{line}");
        }
    }
    if !apply {
        println!("\n(dry-run — nada foi gravado. Use --apply para gravar.)");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn event(state: &str, completed: bool, hs: &str, aws: &str, clock: &str) -> Event {
        Event {
            id: "760415".into(),
            competitions: vec![Competition {
                status: Status {
                    display_clock: clock.into(),
                    type_: StatusType {
                        state: state.into(),
                        completed,
                        name: "STATUS_X".into(),
                        short_detail: "1st".into(),
                    },
                },
                competitors: vec![
                    Competitor {
                        home_away: "home".into(),
                        score: hs.into(),
                        team: Team { display_name: "México".into() },
                    },
                    Competitor {
                        home_away: "away".into(),
                        score: aws.into(),
                        team: Team { display_name: "África do Sul".into() },
                    },
                ],
            }],
        }
    }

    #[test]
    fn live_group_game() {
        assert_eq!(
            classify_event(false, &event("in", false, "1", "0", "67'")),
            GameApply::Live { home: 1, away: 0, status: "67'".into(), elapsed: Some(67) }
        );
    }

    #[test]
    fn scheduled_is_skipped() {
        assert_eq!(classify_event(false, &event("pre", false, "0", "0", "")), GameApply::Skip);
    }

    #[test]
    fn finished_group_autofilled() {
        assert_eq!(
            classify_event(false, &event("post", true, "2", "0", "")),
            GameApply::FinalGroup { home: 2, away: 0, raw_status: "STATUS_X".into() }
        );
    }

    #[test]
    fn finished_knockout_left_to_admin() {
        assert_eq!(
            classify_event(true, &event("post", true, "1", "1", "")),
            GameApply::FinishedKnockout
        );
    }

    #[test]
    fn team_aliases_espn_variants() {
        assert_eq!(canonical_team("Catar"), canonical_team("Qatar"));
        assert_eq!(canonical_team("Holanda"), canonical_team("Países Baixos"));
        assert_eq!(
            canonical_team("República Democrática do Congo"),
            canonical_team("RD Congo")
        );
        assert_eq!(canonical_team("República Tcheca"), canonical_team("Tchéquia"));
        assert_eq!(canonical_team("Brasil"), canonical_team("Brazil"));
    }

    #[test]
    fn et_date_shifts_late_utc_games_back_a_day() {
        // 02:00Z do dia 15 é, em ET (UTC−4), 22:00 do dia 14.
        assert_eq!(et_date("2026-06-15T02:00:00Z").as_deref(), Some("20260614"));
        assert_eq!(et_date("2026-06-14T19:00:00Z").as_deref(), Some("20260614"));
    }
}
