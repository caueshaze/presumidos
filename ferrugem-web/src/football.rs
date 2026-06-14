//! Integração de resultados ao vivo via API pública worldcup26.ir
//! (open-source, gratuita, sem chave): <https://github.com/rezarahiminia/worldcup2026>.
//!
//! - Poller em background (`spawn_poller`) que, a cada `poll_interval_secs`, só
//!   chama a API se há jogo na janela (evita martelar o host gratuito).
//! - Comando CLI (`sync_fixtures`) que mapeia cada `jogo-NNN` local ao `id` da API.
//!   O `id` da API é `1..104` e bate 1:1 com a ordem de `jogo-001..jogo-104`,
//!   então o mapeamento é direto (com validação dos nomes na fase de grupos).
//!
//! Limitações da fonte (decisões de produto):
//! - A API **não** traz placar de pênaltis nem o classificado, e não distingue o
//!   placar do tempo normal do placar com prorrogação. Por isso, o poller só
//!   **finaliza automaticamente os jogos de fase de grupos**. No mata-mata ele
//!   apenas exibe o placar ao vivo; o resultado oficial (classificado/pênaltis)
//!   continua sendo lançado pelo admin.
//! - Resultado de origem `manual` (admin) é soberano: nunca é sobrescrito.

#![cfg(feature = "server")]

use crate::config::settings;
use crate::error::ServerFnError;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Estruturas da resposta da API (apenas os campos que usamos). Tudo vem como
// string nessa API ("2", "TRUE", ...), então parseamos manualmente.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GamesResponse {
    #[serde(default)]
    games: Vec<Game>,
}

#[derive(Debug, Clone, Deserialize)]
struct Game {
    /// Id "1".."104" (alinhado a jogo-001..jogo-104).
    id: String,
    #[serde(default)]
    home_score: String,
    #[serde(default)]
    away_score: String,
    /// "TRUE" / "FALSE".
    #[serde(default)]
    finished: String,
    /// "notstarted" / "finished" / minuto corrido (ex.: "45'", "HT") ao vivo.
    #[serde(default)]
    time_elapsed: String,
    #[serde(default)]
    home_team_name_en: String,
    #[serde(default)]
    away_team_name_en: String,
}

fn parse_score(raw: &str) -> i64 {
    raw.trim().parse::<i64>().unwrap_or(0)
}

fn is_finished(game: &Game) -> bool {
    game.finished.trim().eq_ignore_ascii_case("true")
}

/// Detecta jogo em andamento pelo `time_elapsed` (qualquer valor que não seja
/// "notstarted"/"finished"/vazio), desde que ainda não esteja finalizado.
fn live_label(game: &Game) -> Option<String> {
    let t = game.time_elapsed.trim();
    if is_finished(game) || t.is_empty() {
        return None;
    }
    match t.to_ascii_lowercase().as_str() {
        "notstarted" | "finished" => None,
        _ => Some(t.to_string()),
    }
}

/// Minuto corrido extraído do `time_elapsed`, se houver dígitos (ex.: "45'"→45).
fn live_elapsed(label: &str) -> Option<i64> {
    let digits: String = label.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<i64>().ok()
}

// ---------------------------------------------------------------------------
// Classificação de um jogo da API: o que fazer com ele no banco.
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
    FinalGroup { home: i64, away: i64 },
    /// Mata-mata encerrado — não finaliza (admin lança). Limpa o placar ao vivo.
    FinishedKnockout,
    /// Não começou ou status irrelevante — ignorar.
    Skip,
}

/// Decide, de forma pura e testável, o que fazer com um jogo da API.
fn classify_game(is_knockout: bool, game: &Game) -> GameApply {
    if let Some(label) = live_label(game) {
        return GameApply::Live {
            home: parse_score(&game.home_score),
            away: parse_score(&game.away_score),
            elapsed: live_elapsed(&label),
            status: label,
        };
    }

    if !is_finished(game) {
        return GameApply::Skip;
    }

    if is_knockout {
        // Sem pênaltis/classificado/tempo-normal confiáveis: deixa para o admin.
        return GameApply::FinishedKnockout;
    }

    GameApply::FinalGroup {
        home: parse_score(&game.home_score),
        away: parse_score(&game.away_score),
    }
}

// ---------------------------------------------------------------------------
// Cliente HTTP.
// ---------------------------------------------------------------------------

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .expect("falha ao construir cliente HTTP")
    })
}

async fn fetch_games() -> Result<Vec<Game>, ServerFnError> {
    let base = settings().football.base_url.trim_end_matches('/');
    let url = format!("{base}/get/games");

    let resp = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| crate::security::internal_error("football_fetch_send", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| crate::security::internal_error("football_fetch_body", e))?;

    if !status.is_success() {
        return Err(crate::security::public_error(format!(
            "API de resultados respondeu {status}: {}",
            body.chars().take(200).collect::<String>()
        )));
    }

    let parsed: GamesResponse = serde_json::from_str(&body)
        .map_err(|e| crate::security::internal_error("football_parse", e))?;
    Ok(parsed.games)
}

// ---------------------------------------------------------------------------
// Aplicação no banco.
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct PollCandidate {
    id: String,
    external_fixture_id: Option<i64>,
    phase: Option<String>,
    result_source: Option<String>,
    home_score: Option<i64>,
    away_score: Option<i64>,
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

/// Aplica um jogo da API a um jogo local já mapeado.
async fn apply_game(
    db: &sqlx::SqlitePool,
    candidate: &PollCandidate,
    game: &Game,
) -> Result<(), ServerFnError> {
    use crate::models::is_knockout;
    use serde_json::json;

    let is_ko = is_knockout(candidate.phase.as_deref());
    match classify_game(is_ko, game) {
        GameApply::Skip => Ok(()),

        GameApply::FinishedKnockout => clear_live(db, &candidate.id).await,

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
            Ok(())
        }

        GameApply::FinalGroup { home, away } => {
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
                return Ok(());
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
                    result_external_raw_status = 'finished',
                    live_home_score = NULL, live_away_score = NULL,
                    live_status = NULL, live_elapsed = NULL, live_updated_at = NULL
                 WHERE id = ?3",
            )
            .bind(home)
            .bind(away)
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
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Poller em background.
// ---------------------------------------------------------------------------

/// Sobe a task de polling. Deve ser chamada apenas se a integração e o poller
/// estiverem habilitados (ver [crate::config::FootballConfig]).
pub fn spawn_poller() {
    let interval_secs = settings().football.poll_interval_secs;
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        eprintln!("[football] poller iniciado (intervalo {interval_secs}s)");
        loop {
            ticker.tick().await;
            if let Err(e) = run_poll_cycle().await {
                eprintln!("[football] ciclo falhou: {e:?}");
            }
        }
    });
}

async fn load_candidates(db: &sqlx::SqlitePool) -> Result<Vec<PollCandidate>, ServerFnError> {
    sqlx::query_as::<_, PollCandidate>(
        "SELECT id, external_fixture_id, phase, result_source, home_score, away_score
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
        return Ok(()); // Nenhum jogo na janela — não chama a API.
    }

    let games = fetch_games().await?;
    let by_id: HashMap<&str, &Game> = games.iter().map(|g| (g.id.as_str(), g)).collect();

    for candidate in &candidates {
        let key = candidate.external_fixture_id.map(|id| id.to_string());
        if let Some(game) = key.as_deref().and_then(|k| by_id.get(k)) {
            apply_game(db, candidate, game).await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Comando CLI: mapeamento jogo local <-> id da API.
// ---------------------------------------------------------------------------

pub enum SyncMode {
    /// Mostra o casamento proposto sem gravar.
    DryRun,
    /// Grava o `external_fixture_id` dos jogos casados.
    Apply,
    /// Override manual de um único mapeamento.
    Override { match_id: String, fixture_id: i64 },
}

#[derive(sqlx::FromRow)]
struct LocalMatch {
    id: String,
    home_team: String,
    away_team: String,
    phase: Option<String>,
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

/// Token canônico por seleção, cobrindo o nome em PT (local) e em inglês (API).
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
        ("cape-verde", &["Cabo Verde", "Cape Verde", "Cabo Verde Islands"]),
        ("canada", &["Canadá", "Canada"]),
        ("colombia", &["Colômbia", "Colombia"]),
        ("south-korea", &["Coreia do Sul", "South Korea", "Korea Republic"]),
        ("ivory-coast", &["Costa do Marfim", "Ivory Coast", "Cote d'Ivoire", "Côte d'Ivoire"]),
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
        ("iran", &["Irã", "Iran", "IR Iran"]),
        ("iraq", &["Iraque", "Iraq"]),
        ("japan", &["Japão", "Japan"]),
        ("jordan", &["Jordânia", "Jordan"]),
        ("morocco", &["Marrocos", "Morocco"]),
        ("mexico", &["México", "Mexico"]),
        ("norway", &["Noruega", "Norway"]),
        ("new-zealand", &["Nova Zelândia", "New Zealand"]),
        ("netherlands", &["Países Baixos", "Netherlands", "Holland"]),
        ("panama", &["Panamá", "Panama"]),
        ("paraguay", &["Paraguai", "Paraguay"]),
        ("portugal", &["Portugal"]),
        ("qatar", &["Qatar"]),
        ("dr-congo", &[
            "RD Congo",
            "DR Congo",
            "Congo DR",
            "Congo-Kinshasa",
            "Democratic Republic of the Congo",
        ]),
        ("senegal", &["Senegal"]),
        ("sweden", &["Suécia", "Sweden"]),
        ("switzerland", &["Suíça", "Switzerland"]),
        ("czechia", &["Tchéquia", "Czech Republic", "Czechia"]),
        ("tunisia", &["Tunísia", "Tunisia"]),
        ("turkey", &["Turquia", "Turkey", "Türkiye", "Turkiye"]),
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

/// Número do fixture externo derivado do id local (jogo-027 -> 27).
fn fixture_id_from_match(match_id: &str) -> Option<i64> {
    match_id.rsplit('-').next()?.parse::<i64>().ok()
}

/// Grava (UPSERT) o mapeamento de um jogo para um id externo, com auditoria.
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

/// Mapeia os jogos locais aos ids da API (jogo-NNN -> id N). Valida os nomes na
/// fase de grupos e avisa quando divergem. Em `Apply`, grava os mapeamentos.
pub async fn sync_fixtures(mode: SyncMode) -> Result<(), ServerFnError> {
    let db = crate::db::pool();

    if let SyncMode::Override {
        match_id,
        fixture_id,
    } = &mode
    {
        if set_mapping(db, match_id, *fixture_id).await? {
            println!("Mapeado {match_id} -> id {fixture_id}");
        } else {
            println!("Jogo {match_id} não encontrado.");
        }
        return Ok(());
    }

    let apply = matches!(mode, SyncMode::Apply);

    let locals: Vec<LocalMatch> =
        sqlx::query_as("SELECT id, home_team, away_team, phase FROM matches")
            .fetch_all(db)
            .await
            .map_err(|e| crate::security::internal_error("football_sync_locals", e))?;

    let games = fetch_games().await?;
    println!("API retornou {} jogos.", games.len());

    // Fase de grupos: a numeração das duas fontes diverge (a ordem dos jogos do
    // mesmo dia muda), então casamos por par de times (canônico, único).
    let by_pair: HashMap<(String, String), &Game> = games
        .iter()
        .map(|g| {
            (
                (
                    canonical_team(&g.home_team_name_en),
                    canonical_team(&g.away_team_name_en),
                ),
                g,
            )
        })
        .collect();

    let mut matched = 0usize;
    let mut unmatched: Vec<String> = Vec::new();

    for local in &locals {
        let is_knockout = crate::models::is_knockout(local.phase.as_deref());

        let fixture_id = if is_knockout {
            // Mata-mata: os números 73..104 são canônicos da FIFA e alinhados nas
            // duas fontes (os confrontos ainda são placeholders, sem times).
            fixture_id_from_match(&local.id)
        } else {
            // Grupos: casa pelo par de times.
            let key = (
                canonical_team(&local.home_team),
                canonical_team(&local.away_team),
            );
            by_pair
                .get(&key)
                .and_then(|g| g.id.parse::<i64>().ok())
        };

        match fixture_id {
            Some(id) => {
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
        println!("Não casados (mapeie manualmente com --fixture jogo-XXX=ID):");
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

    fn game(id: &str, hs: &str, aws: &str, finished: &str, elapsed: &str) -> Game {
        Game {
            id: id.into(),
            home_score: hs.into(),
            away_score: aws.into(),
            finished: finished.into(),
            time_elapsed: elapsed.into(),
            home_team_name_en: "A".into(),
            away_team_name_en: "B".into(),
        }
    }

    #[test]
    fn live_group_game_updates_partial_score() {
        let g = game("1", "1", "0", "FALSE", "45'");
        assert_eq!(
            classify_game(false, &g),
            GameApply::Live {
                home: 1,
                away: 0,
                status: "45'".into(),
                elapsed: Some(45)
            }
        );
    }

    #[test]
    fn not_started_is_skipped() {
        assert_eq!(
            classify_game(false, &game("1", "0", "0", "FALSE", "notstarted")),
            GameApply::Skip
        );
    }

    #[test]
    fn finished_group_is_autofilled() {
        assert_eq!(
            classify_game(false, &game("1", "2", "1", "TRUE", "finished")),
            GameApply::FinalGroup { home: 2, away: 1 }
        );
    }

    #[test]
    fn finished_knockout_is_left_to_admin() {
        assert_eq!(
            classify_game(true, &game("73", "1", "1", "TRUE", "finished")),
            GameApply::FinishedKnockout
        );
    }

    #[test]
    fn live_knockout_still_shows_score() {
        let g = game("73", "0", "0", "FALSE", "HT");
        assert_eq!(
            classify_game(true, &g),
            GameApply::Live {
                home: 0,
                away: 0,
                status: "HT".into(),
                elapsed: None
            }
        );
    }

    #[test]
    fn fixture_id_is_parsed_from_match_id() {
        assert_eq!(fixture_id_from_match("jogo-027"), Some(27));
        assert_eq!(fixture_id_from_match("jogo-104"), Some(104));
        assert_eq!(fixture_id_from_match("jogo-abc"), None);
    }

    #[test]
    fn team_name_normalization_handles_variants() {
        assert_eq!(canonical_team("Brasil"), canonical_team("Brazil"));
        assert_eq!(canonical_team("Estados Unidos"), canonical_team("United States"));
        assert_eq!(canonical_team("Tchéquia"), canonical_team("Czech Republic"));
        assert_eq!(canonical_team("Coreia do Sul"), canonical_team("Korea Republic"));
        assert_eq!(canonical_team("RD Congo"), canonical_team("Congo DR"));
    }
}
