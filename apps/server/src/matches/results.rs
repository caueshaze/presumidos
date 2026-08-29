use crate::error::ServerFnError;
use crate::models::{is_knockout, KnockoutEntry, MatchRecord};

use super::predictions::sanitize_knockout_input;
use super::repository::{load_match_record, MatchResultAuditRow};

#[cfg(feature = "server")]
pub async fn set_match_result(
    token: String,
    match_id: String,
    home_score: i64,
    away_score: i64,
    knockout: KnockoutEntry,
    csrf_token: String,
) -> Result<MatchRecord, ServerFnError> {
    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_match_id(&match_id)?;
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    if home_score < 0 || away_score < 0 {
        return Err(crate::security::public_error(
            "Os placares nao podem ser negativos.",
        ));
    }
    let db = crate::db::pool();
    let previous: Option<MatchResultAuditRow> = sqlx::query_as("SELECT phase,home_score,away_score,qualifier,went_to_penalties,penalty_home_score,penalty_away_score FROM matches WHERE id=?1")
        .bind(&match_id).fetch_optional(db).await.map_err(|error| crate::security::internal_error("set_match_result_lookup", error))?;
    let Some(previous) = previous else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };
    let ko = sanitize_knockout_input(
        is_knockout(previous.phase.as_deref()),
        home_score,
        away_score,
        knockout,
    )?;
    sqlx::query("UPDATE matches SET home_score=?1,away_score=?2,qualifier=?3,went_to_penalties=?4,penalty_home_score=?5,penalty_away_score=?6,finished=1 WHERE id=?7")
        .bind(home_score).bind(away_score).bind(&ko.qualifier).bind(ko.went_to_penalties).bind(ko.penalty_home).bind(ko.penalty_away).bind(&match_id).execute(db).await
        .map_err(|error| crate::security::internal_error("set_match_result_update", error))?;
    crate::security::append_audit_log(db, Some(&session.user_id), "match_result_updated", "match", Some(&match_id), Some(&crate::security::client_ip(&headers)), serde_json::json!({
        "before": { "home_score": previous.home_score, "away_score": previous.away_score, "qualifier": previous.qualifier, "went_to_penalties": previous.went_to_penalties, "penalty_home_score": previous.penalty_home_score, "penalty_away_score": previous.penalty_away_score },
        "after": { "home_score": home_score, "away_score": away_score, "qualifier": ko.qualifier, "went_to_penalties": ko.went_to_penalties, "penalty_home_score": ko.penalty_home, "penalty_away_score": ko.penalty_away }
    })).await?;
    let _ = crate::scoring::recalculate_match_breakdowns(&match_id, Some(&session.user_id)).await?;
    load_match_record(db, &match_id).await
}

#[cfg(feature = "server")]
pub async fn set_match_finished(
    token: String,
    match_id: String,
    finished: bool,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    let headers = crate::security::current_headers();
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let result = sqlx::query("UPDATE matches SET finished=?1 WHERE id=?2")
        .bind(finished)
        .bind(&match_id)
        .execute(db)
        .await
        .map_err(|error| crate::security::internal_error("set_match_finished", error))?;
    if result.rows_affected() == 0 {
        return Err(crate::security::public_error("Partida nao encontrada."));
    }
    crate::security::append_audit_log(
        db,
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
