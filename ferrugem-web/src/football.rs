//! Integração de resultados ao vivo via provedor público de placares da
//! competição.
//!
//! - Poller em background (`spawn_poller`) que, a cada `poll_interval_secs`, só
//!   chama a API quando há jogo na janela.
//! - Comando CLI (`sync_fixtures`) que mapeia cada `jogo-NNN` local ao `id` do
//!   evento externo, casando por par de seleções (nomes em PT, normalizados).
//!
//! Semântica de produto:
//! - O poller **finaliza automaticamente apenas a fase de grupos** (placar +
//!   `finished`). No mata-mata ele só exibe o placar ao vivo; o resultado oficial
//!   (classificado/pênaltis) é lançado pelo admin.
//! - Resultado de origem `manual` (admin) é soberano: nunca é sobrescrito.
//!
//! Detalhe de data: o provedor agrupa o `dates=YYYYMMDD` por horário ET (EDT durante
//! a Copa, UTC−4). Por isso a data consultada é o kickoff convertido para ET.

#![cfg(feature = "server")]

use crate::config::settings;
use crate::error::ServerFnError;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Estruturas da resposta do scoreboard externo (apenas o que usamos).
// ---------------------------------------------------------------------------

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
    /// A fonte marca o classificado/vencedor do confronto.
    #[serde(default)]
    winner: bool,
    team: Team,
}

#[derive(Debug, Clone, Deserialize)]
struct Team {
    #[serde(default)]
    id: String,
    #[serde(default, rename = "displayName")]
    display_name: String,
}

// --- Estruturas do endpoint `summary` (apenas a disputa de pênaltis) ---------

#[derive(Debug, Clone, Deserialize)]
struct Summary {
    #[serde(default)]
    shootout: Vec<ShootoutTeam>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct ShootoutTeam {
    /// Id da seleção na fonte (ex.: "202"); casa com `competitor.team.id`.
    #[serde(default)]
    id: String,
    #[serde(default)]
    team: String,
    #[serde(default)]
    shots: Vec<Shot>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct Shot {
    #[serde(default, rename = "didScore")]
    did_score: bool,
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
// Classificação de um evento externo: o que fazer com ele no banco.
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
    /// Mata-mata encerrado. O poller calcula o recorte completo (placar +
    /// classificado + pênaltis via `summary`) e a etapa de aplicação decide entre
    /// autofinalizar com segurança ou deixar pendente para revisão do admin.
    KnockoutFinal {
        home: i64,
        away: i64,
        /// 'home'/'away' do competidor marcado como `winner`, quando houver.
        winner_side: Option<String>,
        home_id: String,
        away_id: String,
        status_name: String,
        /// Empate no tempo normal/prorrogação decidido nos pênaltis.
        went_to_penalties: bool,
    },
    /// Não começou ou sem dados — ignorar.
    Skip,
}

/// Minuto-base extraído do relógio do provedor. Pega só os dígitos iniciais, para
/// que acréscimos como "45'+3'" virem 45 (e não 453).
fn live_elapsed(clock: &str) -> Option<i64> {
    let digits: String = clock.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<i64>().ok()
}

/// Rótulo amigável da fase do jogo ao vivo. Detecta intervalo, prorrogação e
/// pênaltis pelo status do provedor; caso contrário, mostra o minuto do relógio.
fn live_label(status_name: &str, clock: &str, short_detail: &str) -> String {
    match status_name {
        "STATUS_HALFTIME" => "Intervalo".to_string(),
        "STATUS_END_OF_REGULATION" => "Fim do 2º tempo".to_string(),
        "STATUS_EXTRA_TIME_HALFTIME" => "Intervalo da prorrogação".to_string(),
        "STATUS_PENALTIES" | "STATUS_SHOOTOUT" => "Pênaltis".to_string(),
        name if name.contains("EXTRA_TIME") => {
            if clock.is_empty() {
                "Prorrogação".to_string()
            } else {
                format!("Prorrogação · {clock}")
            }
        }
        _ if !clock.is_empty() => clock.to_string(),
        _ if !short_detail.is_empty() => short_detail.to_string(),
        _ => "Ao vivo".to_string(),
    }
}

/// Decide, de forma pura e testável, o que fazer com um evento externo.
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
        let clock = comp.status.display_clock.trim();
        return GameApply::Live {
            home: home_score,
            away: away_score,
            elapsed: live_elapsed(clock),
            status: live_label(comp.status.type_.name.as_str(), clock, comp.status.type_.short_detail.trim()),
        };
    }

    if !finished {
        return GameApply::Skip;
    }

    if is_knockout {
        let winner_side = if home.winner {
            Some("home".to_string())
        } else if away.winner {
            Some("away".to_string())
        } else {
            None
        };
        return GameApply::KnockoutFinal {
            home: home_score,
            away: away_score,
            winner_side,
            home_id: home.team.id.clone(),
            away_id: away.team.id.clone(),
            status_name: comp.status.type_.name.clone(),
            went_to_penalties: comp.status.type_.name == "STATUS_FINAL_PEN",
        };
    }

    GameApply::FinalGroup {
        home: home_score,
        away: away_score,
        raw_status: comp.status.type_.name.clone(),
    }
}

/// Calcula o placar dos pênaltis a partir do `shootout` do `summary`. Casa cada
/// time **primeiro pelo id da fonte** e, como fallback, pelo nome canonizado.
/// Conta os chutes convertidos (`did_score`). Retorna `None` se não conseguir
/// casar os dois lados — nesse caso o admin digita manualmente.
fn compute_shootout(
    shootout: &[ShootoutTeam],
    home_id: &str,
    away_id: &str,
    home_team: &str,
    away_team: &str,
) -> Option<(i64, i64)> {
    fn matches(entry: &ShootoutTeam, id: &str, name: &str) -> bool {
        (!id.is_empty() && entry.id == id)
            || (!entry.team.is_empty() && canonical_team(&entry.team) == canonical_team(name))
    }

    let count = |entry: &ShootoutTeam| -> i64 {
        entry.shots.iter().filter(|s| s.did_score).count() as i64
    };

    let home = shootout.iter().find(|e| matches(e, home_id, home_team));
    let away = shootout.iter().find(|e| matches(e, away_id, away_team));
    match (home, away) {
        (Some(h), Some(a)) => Some((count(h), count(a))),
        _ => None,
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

/// Evento da fonte com o seu JSON bruto preservado (para hash de idempotência e
/// recorte de debug em `source_raw_payload`).
struct RawEvent {
    event: Event,
    raw: serde_json::Value,
}

/// Busca o scoreboard externo de uma data (formato YYYYMMDD, em ET).
async fn fetch_scoreboard(date: &str) -> Result<Vec<RawEvent>, ServerFnError> {
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
                        "Provedor de placares respondeu {status}: {}",
                        body.chars().take(200).collect::<String>()
                    )));
                }
                let root: serde_json::Value = serde_json::from_str(&body)
                    .map_err(|e| crate::security::internal_error("football_parse", e))?;
                let events = root
                    .get("events")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|raw| {
                                serde_json::from_value::<Event>(raw.clone())
                                    .ok()
                                    .map(|event| RawEvent { event, raw: raw.clone() })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                return Ok(events);
            }
            Err(e) => {
                eprintln!(
                    "[football] tentativa {attempt}/{FETCH_ATTEMPTS} falhou ao buscar provedor de placares ({date}): {}",
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

/// URL do endpoint `summary`, derivada da `base_url` do scoreboard (endpoints
/// irmãos: `.../scoreboard` → `.../summary`). Sem nova env var.
fn summary_url() -> Option<String> {
    let base = settings().football.base_url.trim_end_matches('/');
    if base.contains("scoreboard") {
        Some(base.replace("scoreboard", "summary"))
    } else {
        None
    }
}

/// Busca a disputa de pênaltis de um evento via `summary`. Retorna `None` quando
/// não há `shootout` ou a URL não pôde ser derivada (mantém o fluxo: sem
/// pênaltis confiáveis, vira conflito e o admin digita).
async fn fetch_summary_shootout(event_id: &str) -> Result<Option<Vec<ShootoutTeam>>, ServerFnError> {
    let Some(url) = summary_url() else {
        eprintln!("[football] summary: base_url sem 'scoreboard'; pulando pênaltis");
        return Ok(None);
    };

    let mut last_err: Option<reqwest::Error> = None;
    for attempt in 1..=FETCH_ATTEMPTS {
        match client()
            .get(&url)
            .query(&[("event", event_id), ("lang", "pt"), ("region", "br")])
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .map_err(|e| crate::security::internal_error("football_summary_body", e))?;
                if !status.is_success() {
                    return Err(crate::security::public_error(format!(
                        "Provedor de placares (summary) respondeu {status}: {}",
                        body.chars().take(200).collect::<String>()
                    )));
                }
                let parsed: Summary = serde_json::from_str(&body)
                    .map_err(|e| crate::security::internal_error("football_summary_parse", e))?;
                return Ok(if parsed.shootout.is_empty() {
                    None
                } else {
                    Some(parsed.shootout)
                });
            }
            Err(e) => {
                eprintln!(
                    "[football] summary tentativa {attempt}/{FETCH_ATTEMPTS} falhou (event {event_id}): {}",
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
        "football_summary_send",
        last_err.expect("last_err preenchido após o loop"),
    ))
}

pub async fn check_fixture_id(event_id: i64) -> Result<crate::models::FixtureCheckResult, ServerFnError> {
    if event_id <= 0 {
        return Err(crate::security::public_error(
            "O ID do evento externo deve ser um número positivo.",
        ));
    }

    let Some(url) = summary_url() else {
        return Err(crate::security::public_error(
            "FOOTBALL_API_BASE_URL precisa apontar para o endpoint scoreboard para checar eventos.",
        ));
    };

    let event = event_id.to_string();
    let mut last_err: Option<reqwest::Error> = None;
    for attempt in 1..=FETCH_ATTEMPTS {
        match client()
            .get(&url)
            .query(&[("event", event.as_str()), ("lang", "pt"), ("region", "br")])
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .map_err(|e| crate::security::internal_error("football_fixture_check_body", e))?;
                if !status.is_success() {
                    return Err(crate::security::public_error(format!(
                        "Provedor de placares respondeu {status}: {}",
                        body.chars().take(200).collect::<String>()
                    )));
                }

                let root: serde_json::Value = serde_json::from_str(&body)
                    .map_err(|e| crate::security::internal_error("football_fixture_check_parse", e))?;
                let header = root.get("header");
                let competition = header
                    .and_then(|h| h.get("competitions"))
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first());
                let competitors = competition
                    .and_then(|c| c.get("competitors"))
                    .and_then(|v| v.as_array());
                let team_name = |side: &str| {
                    competitors
                        .and_then(|items| {
                            items.iter().find(|item| {
                                item.get("homeAway").and_then(|v| v.as_str()) == Some(side)
                            })
                        })
                        .and_then(|item| item.get("team"))
                        .and_then(|team| team.get("displayName").or_else(|| team.get("shortDisplayName")))
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                };
                let home_team = team_name("home");
                let away_team = team_name("away");
                let kickoff = competition
                    .and_then(|c| c.get("date"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        header
                            .and_then(|h| h.get("competitions"))
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|c| c.get("date"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    });
                let status = competition
                    .and_then(|c| c.get("status"))
                    .and_then(|s| s.get("type"))
                    .and_then(|t| {
                        t.get("shortDetail")
                            .or_else(|| t.get("detail"))
                            .or_else(|| t.get("description"))
                            .or_else(|| t.get("name"))
                    })
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                let label = header
                    .and_then(|h| h.get("name").or_else(|| h.get("shortName")))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .or_else(|| match (&away_team, &home_team) {
                        (Some(away), Some(home)) => Some(format!("{away} x {home}")),
                        _ => None,
                    })
                    .unwrap_or_else(|| "Evento encontrado, mas sem detalhes de times.".to_string());

                let found = home_team.is_some() || away_team.is_some() || header.is_some();
                return Ok(crate::models::FixtureCheckResult {
                    event_id,
                    found,
                    label,
                    status,
                    kickoff,
                    home_team,
                    away_team,
                });
            }
            Err(e) => {
                eprintln!(
                    "[football] check tentativa {attempt}/{FETCH_ATTEMPTS} falhou (event {event_id}): {}",
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
        "football_fixture_check_send",
        last_err.expect("last_err preenchido após o loop"),
    ))
}

/// Hash hex do recorte combinado (scoreboard + shootout) para idempotência: se a
/// disputa de pênaltis chegar depois, o hash muda e a sugestão é regravada.
fn payload_hash(scoreboard_event: &serde_json::Value, shootout: &Option<Vec<ShootoutTeam>>) -> String {
    use sha2::Digest;
    let combined = serde_json::json!({
        "scoreboard_event": scoreboard_event,
        "summary_shootout": shootout,
    });
    let serialized = serde_json::to_string(&combined).unwrap_or_default();
    hex::encode(sha2::Sha256::digest(serialized.as_bytes()))
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
    home_team: String,
    away_team: String,
    source_last_payload_hash: Option<String>,
}

/// O que o poller fez com um jogo neste ciclo (para o log de heartbeat).
#[derive(Debug, Clone, Copy, PartialEq)]
enum ApplyOutcome {
    Finalized,
    /// Sugestão de mata-mata gravada (aguardando confirmação do admin).
    Suggested,
    Live,
    Noop,
}

/// Aplica um evento externo a um jogo local já mapeado.
async fn apply_event(
    db: &sqlx::SqlitePool,
    candidate: &PollCandidate,
    raw_event: &RawEvent,
) -> Result<ApplyOutcome, ServerFnError> {
    use crate::models::is_knockout;
    use serde_json::json;

    let event = &raw_event.event;
    let is_ko = is_knockout(candidate.phase.as_deref());
    match classify_event(is_ko, event) {
        GameApply::Skip => Ok(ApplyOutcome::Noop),

        GameApply::KnockoutFinal {
            home,
            away,
            winner_side,
            home_id,
            away_id,
            status_name,
            went_to_penalties,
        } => {
            apply_knockout_suggestion(
                db,
                candidate,
                raw_event,
                home,
                away,
                winner_side,
                &home_id,
                &away_id,
                &status_name,
                went_to_penalties,
            )
            .await
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
                let _ = crate::scoring::recalculate_match_breakdowns(&candidate.id, None).await?;
            }
            Ok(if already {
                ApplyOutcome::Noop
            } else {
                ApplyOutcome::Finalized
            })
        }
    }
}

/// Mata-mata encerrado: calcula o resultado completo. Quando a fonte é coerente
/// (classificado definido e pênaltis completos, se houver), fecha o jogo como
/// resultado de API. Em conflito/dado incompleto, grava `auto_*` para o admin
/// confirmar manualmente.
#[allow(clippy::too_many_arguments)]
async fn apply_knockout_suggestion(
    db: &sqlx::SqlitePool,
    candidate: &PollCandidate,
    raw_event: &RawEvent,
    home: i64,
    away: i64,
    winner_side: Option<String>,
    home_id: &str,
    away_id: &str,
    status_name: &str,
    went_to_penalties: bool,
) -> Result<ApplyOutcome, ServerFnError> {
    use serde_json::json;

    // Pênaltis: só busca o summary quando o tempo normal/prorrogação empatou.
    let shootout = if went_to_penalties {
        fetch_summary_shootout(&candidate.external_fixture_id.map(|i| i.to_string()).unwrap_or_default())
            .await?
    } else {
        None
    };

    // Idempotência: hash do recorte combinado (scoreboard + shootout). Se nada
    // mudou desde a última checagem, não reescreve nada.
    let hash = payload_hash(&raw_event.raw, &shootout);
    if candidate.source_last_payload_hash.as_deref() == Some(hash.as_str()) {
        return Ok(ApplyOutcome::Noop);
    }

    let raw_payload = json!({
        "scoreboard_event": raw_event.raw,
        "summary_shootout": shootout,
        "fetched_at": chrono::Utc::now().to_rfc3339(),
    })
    .to_string();

    // Soberania do admin: jogo já lançado manualmente nunca recebe sugestão.
    // Mantém só a instrumentação de debug e registra (uma vez por mudança).
    if candidate.result_source.as_deref() == Some("manual") {
        sqlx::query(
            "UPDATE matches SET
                source_last_checked_at = datetime('now'), source_last_status = ?1,
                source_last_payload_hash = ?2, source_raw_payload = ?3
             WHERE id = ?4",
        )
        .bind(status_name)
        .bind(&hash)
        .bind(&raw_payload)
        .bind(&candidate.id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("football_ko_debug_update", e))?;

        crate::security::append_audit_log(
            db,
            None,
            "knockout_result_suggestion_skipped_manual",
            "match",
            Some(&candidate.id),
            None,
            json!({ "api": { "home": home, "away": away, "status": status_name } }),
        )
        .await?;
        return Ok(ApplyOutcome::Noop);
    }

    // Classificado e pênaltis. Em caso de divergência, vira conflito: grava a
    // sugestão de placar mas sem qualifier confiável (admin decide).
    let (pen_home, pen_away, qualifier, conflict) = if went_to_penalties {
        match shootout
            .as_ref()
            .and_then(|s| compute_shootout(s, home_id, away_id, &candidate.home_team, &candidate.away_team))
        {
            Some((ph, pa)) if ph != pa => {
                let pen_winner = if ph > pa { "home" } else { "away" };
                let agrees = winner_side.as_deref().map(|w| w == pen_winner).unwrap_or(true);
                if agrees {
                    (Some(ph), Some(pa), Some(pen_winner.to_string()), false)
                } else {
                    (Some(ph), Some(pa), None, true)
                }
            }
            // Sem shootout dos dois lados ou placar empatado: sugestão sem pênaltis.
            _ => (None, None, None, true),
        }
    } else {
        // Vitória no tempo normal: classificado pelo `winner` (ou pelo placar).
        let q = winner_side
            .clone()
            .unwrap_or_else(|| if home >= away { "home".to_string() } else { "away".to_string() });
        (None, None, Some(q), false)
    };

    let can_finalize = !conflict && qualifier.is_some();
    let action = if can_finalize {
        sqlx::query(
            "UPDATE matches SET
                home_score = ?1, away_score = ?2,
                qualifier = ?3, went_to_penalties = ?4,
                penalty_home_score = ?5, penalty_away_score = ?6,
                finished = 1,
                result_source = 'api',
                result_synced_at = datetime('now'),
                result_external_raw_status = ?7,
                live_home_score = NULL, live_away_score = NULL,
                live_status = NULL, live_elapsed = NULL, live_updated_at = NULL,
                auto_home_score = ?1, auto_away_score = ?2,
                auto_penalty_home_score = ?5, auto_penalty_away_score = ?6,
                auto_qualifier = ?3, auto_status = ?7, auto_detected_at = datetime('now'),
                source_home_team_id = ?8, source_away_team_id = ?9,
                source_last_checked_at = datetime('now'), source_last_status = ?7,
                source_last_payload_hash = ?10, source_raw_payload = ?11
             WHERE id = ?12 AND finished = 0",
        )
        .bind(home)
        .bind(away)
        .bind(&qualifier)
        .bind(went_to_penalties)
        .bind(pen_home)
        .bind(pen_away)
        .bind(status_name)
        .bind(home_id)
        .bind(away_id)
        .bind(&hash)
        .bind(&raw_payload)
        .bind(&candidate.id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("football_ko_autofinal_update", e))?;

        let _ = crate::scoring::recalculate_match_breakdowns(&candidate.id, None).await?;
        "knockout_result_autofinalized"
    } else {
        sqlx::query(
            "UPDATE matches SET
                auto_home_score = ?1, auto_away_score = ?2,
                auto_penalty_home_score = ?3, auto_penalty_away_score = ?4,
                auto_qualifier = ?5, auto_status = ?6, auto_detected_at = datetime('now'),
                source_home_team_id = ?7, source_away_team_id = ?8,
                source_last_checked_at = datetime('now'), source_last_status = ?6,
                source_last_payload_hash = ?9, source_raw_payload = ?10
             WHERE id = ?11 AND finished = 0",
        )
        .bind(home)
        .bind(away)
        .bind(pen_home)
        .bind(pen_away)
        .bind(&qualifier)
        .bind(status_name)
        .bind(home_id)
        .bind(away_id)
        .bind(&hash)
        .bind(&raw_payload)
        .bind(&candidate.id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("football_ko_suggestion_update", e))?;

        if conflict { "knockout_result_conflict" } else { "knockout_result_suggested" }
    };

    crate::security::append_audit_log(
        db,
        None,
        action,
        "match",
        Some(&candidate.id),
        None,
        json!({
            "home": home, "away": away,
            "penalty_home": pen_home, "penalty_away": pen_away,
            "qualifier": qualifier, "winner_side": winner_side, "status": status_name,
            "autofinalized": can_finalize,
        }),
    )
    .await?;

    Ok(if can_finalize {
        ApplyOutcome::Finalized
    } else {
        ApplyOutcome::Suggested
    })
}

// ---------------------------------------------------------------------------
// Datas: o provedor agrupa por dia ET (EDT na Copa, UTC−4).
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

    let live_secs = settings().football.live_poll_interval_secs;
    tokio::spawn(async move {
        eprintln!("[football] poller iniciado (cadencia base dinamica pelo admin, + jitter 0–60s, fonte externa)");
        loop {
            let auto_sync_enabled = crate::admin::auto_sync_enabled().await.unwrap_or(true);
            let base_secs = crate::admin::sync_interval_minutes()
                .await
                .ok()
                .and_then(|minutes| u64::try_from(minutes).ok())
                .map(|minutes| minutes.saturating_mul(60))
                .unwrap_or(settings().football.poll_interval_secs);
            if !auto_sync_enabled {
                tokio::time::sleep(std::time::Duration::from_secs(base_secs)).await;
                continue;
            }
            // `active` indica que havia jogo na janela: enquanto rola jogo, o
            // poller acelera (live_secs) para a pontuação ao vivo andar mais
            // rápido; ocioso, volta ao intervalo base.
            let active = match run_poll_cycle().await {
                Ok(active) => active,
                Err(e) => {
                    eprintln!("[football] ciclo falhou: {e:?}");
                    false
                }
            };
            let interval_secs = if active { live_secs } else { base_secs };
            // Jitter de 0–60s: evita bater sempre cravado no mesmo segundo.
            let jitter = u64::from(OsRng.next_u32() % 61);
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs + jitter)).await;
        }
    });
}

/// Escopo de um ciclo de sincronização.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CycleScope {
    /// Poller ao vivo: só a janela curta ao redor do kickoff (≈ jogos do dia).
    Live,
    /// Backfill global: todos os jogos passados ainda não finalizados,
    /// independentemente de há quanto tempo aconteceram. Usado para recuperar o
    /// banco quando o poller ficou desligado durante os jogos.
    Backfill,
}

/// Resumo de um ciclo, para log e para o registro em `sync_runs`.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct CycleSummary {
    pub candidates: usize,
    pub finalized: u32,
    pub suggested: u32,
    pub live: u32,
}

async fn load_candidates(
    db: &sqlx::SqlitePool,
    scope: CycleScope,
) -> Result<Vec<PollCandidate>, ServerFnError> {
    // Filtro de janela conforme o escopo. No backfill removemos o limite
    // inferior (qualquer jogo já iniciado no passado entra), o que permite
    // finalizar jogos antigos que o poller perdeu.
    let window = match scope {
        CycleScope::Live => {
            "datetime(kickoff) BETWEEN datetime('now','-5 hours') AND datetime('now','+30 minutes')"
        }
        CycleScope::Backfill => "datetime(kickoff) < datetime('now')",
    };
    let sql = format!(
        "SELECT id, kickoff, external_fixture_id, phase, result_source, home_score, away_score,
                home_team, away_team, source_last_payload_hash
         FROM matches
         WHERE finished = 0
           AND external_fixture_id IS NOT NULL
           AND {window}",
    );
    sqlx::query_as::<_, PollCandidate>(&sql)
        .fetch_all(db)
        .await
        .map_err(|e| crate::security::internal_error("football_load_candidates", e))
}

/// Roda um ciclo de sincronização no escopo dado e devolve o resumo.
pub async fn run_cycle(scope: CycleScope) -> Result<CycleSummary, ServerFnError> {
    let db = crate::db::pool();
    let candidates = load_candidates(db, scope).await?;
    if candidates.is_empty() {
        eprintln!("[football] ciclo ({scope:?}): nenhum jogo elegível");
        return Ok(CycleSummary::default());
    }

    // Busca os scoreboards das datas ET envolvidas. No backfill podem ser muitas
    // (uma chamada por dia de jogo); é uma operação manual e pontual.
    let dates = distinct_et_dates(candidates.iter().map(|c| c.kickoff.clone()));
    let mut by_id: HashMap<String, RawEvent> = HashMap::new();
    for date in &dates {
        for ev in fetch_scoreboard(date).await? {
            by_id.insert(ev.event.id.clone(), ev);
        }
    }

    let mut summary = CycleSummary {
        candidates: candidates.len(),
        ..Default::default()
    };
    for candidate in &candidates {
        let key = candidate.external_fixture_id.map(|id| id.to_string());
        if let Some(event) = key.as_deref().and_then(|k| by_id.get(k)) {
            match apply_event(db, candidate, event).await? {
                ApplyOutcome::Finalized => summary.finalized += 1,
                ApplyOutcome::Suggested => summary.suggested += 1,
                ApplyOutcome::Live => summary.live += 1,
                ApplyOutcome::Noop => {}
            }
        }
    }

    eprintln!(
        "[football] ciclo ({scope:?}): {} jogo(s) em {} data(s), {} finalizado(s), {} sugerido(s), {} ao vivo",
        summary.candidates, dates.len(), summary.finalized, summary.suggested, summary.live
    );
    Ok(summary)
}

/// Ciclo do poller ao vivo. Retorna `true` se havia jogo na janela (sinal para o
/// poller acelerar a cadência enquanto há jogo).
pub async fn run_poll_cycle() -> Result<bool, ServerFnError> {
    Ok(run_cycle(CycleScope::Live).await?.candidates > 0)
}

/// Backfill global: varre todos os jogos passados ainda não finalizados e aplica
/// o resultado da fonte externa. Finaliza jogos de grupo, gera sugestões de
/// mata-mata e respeita resultado manual — mesma semântica do poller.
pub async fn run_backfill() -> Result<CycleSummary, ServerFnError> {
    run_cycle(CycleScope::Backfill).await
}

// ---------------------------------------------------------------------------
// Comando CLI: mapeamento jogo local <-> id do evento externo.
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

/// Token canônico por seleção, cobrindo o nome local (PT) e o do provedor (PT, com
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

/// Mapeia os jogos locais aos ids de evento externos, casando por par de times.
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
    println!("Consultando {} data(s) no provedor de placares...", dates.len());
    let mut by_pair: HashMap<(String, String), i64> = HashMap::new();
    for date in &dates {
        for raw in fetch_scoreboard(date).await? {
            let ev = &raw.event;
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
    println!("O provedor retornou {} confronto(s).", by_pair.len());

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
        // `winner` derivado do placar (empate ⇒ ninguém vence no tempo normal).
        let (hw, aw) = match hs.parse::<i64>().ok().zip(aws.parse::<i64>().ok()) {
            Some((h, a)) if h > a => (true, false),
            Some((h, a)) if a > h => (false, true),
            _ => (false, false),
        };
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
                        winner: hw,
                        team: Team { id: "1".into(), display_name: "México".into() },
                    },
                    Competitor {
                        home_away: "away".into(),
                        score: aws.into(),
                        winner: aw,
                        team: Team { id: "2".into(), display_name: "África do Sul".into() },
                    },
                ],
            }],
        }
    }

    fn shootout_team(id: &str, name: &str, scored: &[bool]) -> ShootoutTeam {
        ShootoutTeam {
            id: id.into(),
            team: name.into(),
            shots: scored.iter().map(|&s| Shot { did_score: s }).collect(),
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
    fn halftime_shows_intervalo() {
        // displayClock real observado no provedor durante o intervalo: "45'+3'".
        let mut ev = event("in", false, "0", "0", "45'+3'");
        ev.competitions[0].status.type_.name = "STATUS_HALFTIME".into();
        assert_eq!(
            classify_event(false, &ev),
            GameApply::Live { home: 0, away: 0, status: "Intervalo".into(), elapsed: Some(45) }
        );
    }

    #[test]
    fn stoppage_time_keeps_base_minute() {
        // "45'+3'" em jogo (não intervalo): rótulo mostra o relógio, minuto = 45.
        let g = event("in", false, "1", "0", "45'+3'");
        assert_eq!(
            classify_event(false, &g),
            GameApply::Live { home: 1, away: 0, status: "45'+3'".into(), elapsed: Some(45) }
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
    fn finished_knockout_win_in_regulation() {
        // Mata-mata 2x0 encerrado: classificado pelo placar, sem pênaltis.
        assert_eq!(
            classify_event(true, &event("post", true, "2", "0", "")),
            GameApply::KnockoutFinal {
                home: 2,
                away: 0,
                winner_side: Some("home".into()),
                home_id: "1".into(),
                away_id: "2".into(),
                status_name: "STATUS_X".into(),
                went_to_penalties: false,
            }
        );
    }

    #[test]
    fn finished_knockout_penalties_flagged() {
        // Empate 3x3 com STATUS_FINAL_PEN: sinaliza pênaltis (busca summary depois).
        let mut ev = event("post", true, "3", "3", "");
        ev.competitions[0].status.type_.name = "STATUS_FINAL_PEN".into();
        // A fonte marca o vencedor dos pênaltis mesmo no empate do tempo normal.
        ev.competitions[0].competitors[0].winner = true;
        let result = classify_event(true, &ev);
        assert_eq!(
            result,
            GameApply::KnockoutFinal {
                home: 3,
                away: 3,
                winner_side: Some("home".into()),
                home_id: "1".into(),
                away_id: "2".into(),
                status_name: "STATUS_FINAL_PEN".into(),
                went_to_penalties: true,
            }
        );
    }

    #[test]
    fn shootout_counts_by_id_then_name() {
        // Final 2022 (Argentina 4 x 2 França). Casa por id, ordem invertida no array.
        let shootout = vec![
            shootout_team("478", "França", &[true, false, true, false]),
            shootout_team("202", "Argentina", &[true, true, true, true]),
        ];
        // home = Argentina (id 202), away = França (id 478).
        assert_eq!(
            compute_shootout(&shootout, "202", "478", "Argentina", "França"),
            Some((4, 2))
        );
        // Sem id, casa pelo nome canonizado (EN/PT).
        let by_name = vec![
            shootout_team("", "Argentina", &[true, true, true, false]),
            shootout_team("", "France", &[true, false, false, false]),
        ];
        assert_eq!(
            compute_shootout(&by_name, "", "", "Argentina", "França"),
            Some((3, 1))
        );
    }

    #[test]
    fn shootout_unmatched_side_returns_none() {
        let shootout = vec![shootout_team("202", "Argentina", &[true, true])];
        assert_eq!(
            compute_shootout(&shootout, "202", "478", "Argentina", "França"),
            None
        );
    }

    #[test]
    fn payload_hash_changes_when_summary_arrives() {
        // Mesmo scoreboard, mas a chegada do shootout muda o hash (não é pulado).
        let sb = serde_json::json!({ "id": "633850", "status": "post" });
        let none = payload_hash(&sb, &None);
        let with_pens = payload_hash(
            &sb,
            &Some(vec![shootout_team("202", "Argentina", &[true])]),
        );
        assert_ne!(none, with_pens);
    }

    #[test]
    fn team_aliases_provider_variants() {
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
