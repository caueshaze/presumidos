use crate::error::ServerFnError;

use crate::models::{KnockoutEntry, MatchRecord, PredictionRecord};

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct MatchRow {
    id: String,
    home_team: String,
    away_team: String,
    kickoff: String,
    group_name: Option<String>,
    phase: Option<String>,
    home_score: Option<i64>,
    away_score: Option<i64>,
    qualifier: Option<String>,
    went_to_penalties: bool,
    penalty_home_score: Option<i64>,
    penalty_away_score: Option<i64>,
    finished: bool,
    live_home_score: Option<i64>,
    live_away_score: Option<i64>,
    live_status: Option<String>,
    live_elapsed: Option<i64>,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct MatchResultAuditRow {
    phase: Option<String>,
    home_score: Option<i64>,
    away_score: Option<i64>,
    qualifier: Option<String>,
    went_to_penalties: bool,
    penalty_home_score: Option<i64>,
    penalty_away_score: Option<i64>,
}

/// Lê a flag de liberação do mata-mata em `app_settings`.
#[cfg(feature = "server")]
async fn knockout_released_flag() -> Result<bool, ServerFnError> {
    use crate::db::pool;

    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = 'knockout_released'")
            .fetch_optional(pool())
            .await
            .map_err(|e| crate::security::internal_error("knockout_released_flag", e))?;

    Ok(row.map(|(value,)| value == "1").unwrap_or(false))
}

/// Verifica se o token pertence a um admin sem retornar erro para tokens
/// inválidos (usado apenas para decidir visibilidade).
#[cfg(feature = "server")]
async fn token_is_admin(
    token: &str,
) -> bool {
    crate::auth::require_admin(token).await.is_ok()
}

#[cfg(feature = "server")]
pub async fn is_knockout_released() -> Result<bool, ServerFnError> {
    knockout_released_flag().await
}

#[cfg(feature = "server")]
pub async fn list_matches(
    token: String,
) -> Result<Vec<MatchRecord>, ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    // Admin sempre vê tudo (para montar/conferir os confrontos antes de liberar);
    // os demais só veem o mata-mata depois que ele é liberado.
    let show_knockout = token_is_admin(&token).await || knockout_released_flag().await?;

    let rows = if show_knockout {
        sqlx::query_as::<_, MatchRow>(
            "SELECT id, home_team, away_team, kickoff, group_name, phase,
                    home_score, away_score, qualifier, went_to_penalties,
                    penalty_home_score, penalty_away_score, finished,
                    live_home_score, live_away_score, live_status, live_elapsed
             FROM matches
             ORDER BY kickoff ASC",
        )
        .fetch_all(pool())
        .await
    } else {
        sqlx::query_as::<_, MatchRow>(
            "SELECT id, home_team, away_team, kickoff, group_name, phase,
                    home_score, away_score, qualifier, went_to_penalties,
                    penalty_home_score, penalty_away_score, finished,
                    live_home_score, live_away_score, live_status, live_elapsed
             FROM matches
             WHERE phase = 'Fase de grupos'
             ORDER BY kickoff ASC",
        )
        .fetch_all(pool())
        .await
    }
    .map_err(|e| crate::security::internal_error("list_matches", e))?;

    Ok(rows
        .into_iter()
        .map(|row| MatchRecord {
            id: row.id,
            home_team: row.home_team,
            away_team: row.away_team,
            kickoff: row.kickoff,
            group_name: row.group_name,
            phase: row.phase,
            home_score: row.home_score,
            away_score: row.away_score,
            qualifier: row.qualifier,
            went_to_penalties: row.went_to_penalties,
            penalty_home_score: row.penalty_home_score,
            penalty_away_score: row.penalty_away_score,
            finished: row.finished,
            live_home_score: row.live_home_score,
            live_away_score: row.live_away_score,
            live_status: row.live_status,
            live_elapsed: row.live_elapsed,
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn get_my_predictions(
    token: String,
) -> Result<Vec<PredictionRecord>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;

    #[derive(sqlx::FromRow)]
    struct PredictionRow {
        match_id: String,
        home_score: i64,
        away_score: i64,
        qualifier: Option<String>,
        went_to_penalties: bool,
        penalty_home_score: Option<i64>,
        penalty_away_score: Option<i64>,
    }

    let rows = sqlx::query_as::<_, PredictionRow>(
        "SELECT match_id, home_score, away_score, qualifier, went_to_penalties,
                penalty_home_score, penalty_away_score
         FROM predictions WHERE user_id = ?1",
    )
    .bind(&session.user_id)
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("get_my_predictions", e))?;

    Ok(rows
        .into_iter()
        .map(|row| PredictionRecord {
            match_id: row.match_id,
            home_score: row.home_score,
            away_score: row.away_score,
            qualifier: row.qualifier,
            went_to_penalties: row.went_to_penalties,
            penalty_home_score: row.penalty_home_score,
            penalty_away_score: row.penalty_away_score,
        })
        .collect())
}

/// Normaliza e valida os campos de mata-mata (usado por palpite e resultado).
/// Em jogos de grupo, zera tudo. Em mata-mata, aplica as regras de classificado
/// e pênaltis, limpando campos quando o usuário muda de ideia.
#[cfg(feature = "server")]
fn sanitize_knockout_input(
    is_knockout: bool,
    home_score: i64,
    away_score: i64,
    entry: KnockoutEntry,
) -> Result<KnockoutEntry, ServerFnError> {
    if !is_knockout {
        return Ok(KnockoutEntry::default());
    }

    let qualifier = entry
        .qualifier
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    let qualifier = match qualifier.as_deref() {
        Some("home") => "home".to_string(),
        Some("away") => "away".to_string(),
        _ => {
            return Err(crate::security::public_error(
                "Escolha quem se classifica (mandante ou visitante).",
            ))
        }
    };

    // Pênaltis só fazem sentido se o tempo normal terminou empatado.
    let went_to_penalties = entry.went_to_penalties && home_score == away_score;

    let (penalty_home, penalty_away) = if went_to_penalties {
        match (entry.penalty_home, entry.penalty_away) {
            // Placar dos pênaltis é opcional.
            (None, None) => (None, None),
            (Some(ph), Some(pa)) => {
                if ph < 0 || pa < 0 {
                    return Err(crate::security::public_error(
                        "O placar dos pênaltis não pode ser negativo.",
                    ));
                }
                if ph == pa {
                    return Err(crate::security::public_error(
                        "O placar dos pênaltis não pode terminar empatado.",
                    ));
                }
                let pen_winner = if ph > pa { "home" } else { "away" };
                if pen_winner != qualifier {
                    return Err(crate::security::public_error(
                        "O vencedor dos pênaltis precisa ser o classificado.",
                    ));
                }
                (Some(ph), Some(pa))
            }
            _ => {
                return Err(crate::security::public_error(
                    "Informe o placar dos pênaltis dos dois lados.",
                ))
            }
        }
    } else {
        // Sem pênaltis (ou tempo normal não empatado): limpa o placar.
        (None, None)
    };

    Ok(KnockoutEntry {
        qualifier: Some(qualifier),
        went_to_penalties,
        penalty_home,
        penalty_away,
    })
}

#[cfg(feature = "server")]
pub async fn submit_prediction(
    token: String,
    match_id: String,
    home_score: i64,
    away_score: i64,
    knockout: KnockoutEntry,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use crate::models::is_knockout;
    use chrono::Utc;
    use uuid::Uuid;

    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    if home_score < 0 || away_score < 0 {
        return Err(crate::security::public_error("Os placares nao podem ser negativos."));
    }

    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = pool();

    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT kickoff, phase FROM matches WHERE id = ?1")
            .bind(&match_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("submit_prediction_match_lookup", e))?;

    let Some((kickoff, phase)) = row else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };

    let kickoff_time = chrono::DateTime::parse_from_rfc3339(&kickoff)
        .map_err(|e| crate::security::internal_error("submit_prediction_parse_kickoff", e))?;

    if Utc::now() >= kickoff_time {
        return Err(crate::security::public_error(
            "Essa partida ja comecou; nao e mais possivel enviar palpites.",
        ));
    }

    let ko = sanitize_knockout_input(
        is_knockout(phase.as_deref()),
        home_score,
        away_score,
        knockout,
    )?;

    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO predictions
            (id, user_id, match_id, home_score, away_score,
             qualifier, went_to_penalties, penalty_home_score, penalty_away_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(user_id, match_id) DO UPDATE SET
            home_score = excluded.home_score,
            away_score = excluded.away_score,
            qualifier = excluded.qualifier,
            went_to_penalties = excluded.went_to_penalties,
            penalty_home_score = excluded.penalty_home_score,
            penalty_away_score = excluded.penalty_away_score",
    )
    .bind(&id)
    .bind(&session.user_id)
    .bind(&match_id)
    .bind(home_score)
    .bind(away_score)
    .bind(&ko.qualifier)
    .bind(ko.went_to_penalties)
    .bind(ko.penalty_home)
    .bind(ko.penalty_away)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("submit_prediction_upsert", e))?;

    Ok(())
}

#[cfg(feature = "server")]
pub async fn set_match_result(
    token: String,
    match_id: String,
    home_score: i64,
    away_score: i64,
    knockout: KnockoutEntry,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;
    use crate::models::is_knockout;
    use serde_json::json;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_match_id(&match_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    if home_score < 0 || away_score < 0 {
        return Err(crate::security::public_error("Os placares nao podem ser negativos."));
    }

    let db = pool();

    let row: Option<MatchResultAuditRow> = sqlx::query_as(
        "SELECT phase, home_score, away_score, qualifier, went_to_penalties, penalty_home_score, penalty_away_score
         FROM matches WHERE id = ?1",
    )
        .bind(&match_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("set_match_result_lookup", e))?;

    let Some(previous) = row else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };

    let ko = sanitize_knockout_input(
        is_knockout(previous.phase.as_deref()),
        home_score,
        away_score,
        knockout,
    )?;

    // Resultado lançado pelo admin é soberano: marca a origem como 'manual'
    // (o poller nunca sobrescreve) e limpa o placar ao vivo.
    sqlx::query(
        "UPDATE matches SET
            home_score = ?1, away_score = ?2,
            qualifier = ?3, went_to_penalties = ?4,
            penalty_home_score = ?5, penalty_away_score = ?6,
            result_source = 'manual',
            result_synced_at = datetime('now'),
            live_home_score = NULL, live_away_score = NULL,
            live_status = NULL, live_elapsed = NULL, live_updated_at = NULL
         WHERE id = ?7",
    )
    .bind(home_score)
    .bind(away_score)
    .bind(&ko.qualifier)
    .bind(ko.went_to_penalties)
    .bind(ko.penalty_home)
    .bind(ko.penalty_away)
    .bind(&match_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("set_match_result_update", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "match_result_updated",
        "match",
        Some(&match_id),
        Some(&crate::security::client_ip(&headers)),
        json!({
            "before": {
                "home_score": previous.home_score,
                "away_score": previous.away_score,
                "qualifier": previous.qualifier,
                "went_to_penalties": previous.went_to_penalties,
                "penalty_home_score": previous.penalty_home_score,
                "penalty_away_score": previous.penalty_away_score
            },
            "after": {
                "home_score": home_score,
                "away_score": away_score,
                "qualifier": ko.qualifier,
                "went_to_penalties": ko.went_to_penalties,
                "penalty_home_score": ko.penalty_home,
                "penalty_away_score": ko.penalty_away
            }
        }),
    )
    .await?;

    Ok(())
}

#[cfg(feature = "server")]
pub async fn set_knockout_released(
    token: String,
    released: bool,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    sqlx::query("UPDATE app_settings SET value = ?1 WHERE key = 'knockout_released'")
        .bind(if released { "1" } else { "0" })
        .execute(pool())
        .await
        .map_err(|e| crate::security::internal_error("set_knockout_released", e))?;

    crate::security::append_audit_log(
        pool(),
        Some(&session.user_id),
        "knockout_release_changed",
        "app_settings",
        Some("knockout_released"),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "released": released,
        }),
    )
    .await?;

    Ok(())
}

/// Marca/desmarca um jogo como finalizado (rótulo oficial). Não altera o placar
/// nem a pontuação — o placar já conta no ranking quando preenchido.
#[cfg(feature = "server")]
pub async fn set_match_finished(
    token: String,
    match_id: String,
    finished: bool,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let result = sqlx::query("UPDATE matches SET finished = ?1 WHERE id = ?2")
        .bind(finished)
        .bind(&match_id)
        .execute(pool())
        .await
        .map_err(|e| crate::security::internal_error("set_match_finished", e))?;

    if result.rows_affected() == 0 {
        return Err(crate::security::public_error("Partida nao encontrada."));
    }

    crate::security::append_audit_log(
        pool(),
        Some(&session.user_id),
        "match_finished_changed",
        "match",
        Some(&match_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "finished": finished }),
    )
    .await?;

    Ok(())
}

#[cfg(feature = "server")]
pub async fn update_match_teams(
    token: String,
    match_id: String,
    home_team: String,
    away_team: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_match_id(&match_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let home = crate::security::normalize_required_text("Selecao mandante", home_team, 1, 60)?;
    let away = crate::security::normalize_required_text("Selecao visitante", away_team, 1, 60)?;

    let db = pool();
    let before: Option<(String, String)> =
        sqlx::query_as("SELECT home_team, away_team FROM matches WHERE id = ?1")
            .bind(&match_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("update_match_teams_lookup", e))?;

    let Some((old_home, old_away)) = before else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };

    sqlx::query("UPDATE matches SET home_team = ?1, away_team = ?2 WHERE id = ?3")
        .bind(&home)
        .bind(&away)
        .bind(&match_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("update_match_teams", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "match_teams_updated",
        "match",
        Some(&match_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "before": { "home_team": old_home, "away_team": old_away },
            "after": { "home_team": home, "away_team": away }
        }),
    )
    .await?;

    Ok(())
}
