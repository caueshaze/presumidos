use crate::error::ServerFnError;
use crate::models::{is_knockout, MatchRecord};

use super::lifecycle::{ensure_football_working_revision, working_match_for};
use super::repository::load_match_record;

#[cfg(feature = "server")]
pub async fn set_knockout_released(
    token: String,
    released: bool,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    sqlx::query("UPDATE app_settings SET value=?1 WHERE key='knockout_released'")
        .bind(if released { "1" } else { "0" })
        .execute(db)
        .await
        .map_err(|error| crate::security::internal_error("set_knockout_released", error))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "knockout_release_changed",
        "app_settings",
        Some("knockout_released"),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "released": released }),
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
    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_match_id(&match_id)?;
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let home = crate::security::normalize_required_text("Selecao mandante", home_team, 1, 60)?;
    let away = crate::security::normalize_required_text("Selecao visitante", away_team, 1, 60)?;
    let db = crate::db::pool();
    let before: Option<(String, String)> =
        sqlx::query_as("SELECT home_team,away_team FROM matches WHERE id=?1")
            .bind(&match_id)
            .fetch_optional(db)
            .await
            .map_err(|error| crate::security::internal_error("update_match_teams_lookup", error))?;
    let Some((old_home, old_away)) = before else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };
    sqlx::query("UPDATE matches SET home_team=?1,away_team=?2 WHERE id=?3")
        .bind(&home)
        .bind(&away)
        .bind(&match_id)
        .execute(db)
        .await
        .map_err(|error| crate::security::internal_error("update_match_teams", error))?;
    crate::security::append_audit_log(db, Some(&session.user_id), "match_teams_updated", "match", Some(&match_id), Some(&crate::security::client_ip(&headers)), serde_json::json!({ "before": { "home_team": old_home, "away_team": old_away }, "after": { "home_team": home, "away_team": away } })).await?;
    Ok(())
}

#[cfg(feature = "server")]
fn normalize_knockout_match_input(
    phase: String,
    kickoff: String,
) -> Result<(String, String), ServerFnError> {
    let phase = crate::security::normalize_required_text("Fase", phase, 1, 60)?;
    if !is_knockout(Some(&phase)) {
        return Err(crate::security::public_error(
            "O cadastro manual é apenas para jogos de mata-mata.",
        ));
    }
    let parsed = chrono::DateTime::parse_from_rfc3339(kickoff.trim())
        .map_err(|_| crate::security::public_error("Data/hora do jogo inválida."))?;
    Ok((phase, parsed.with_timezone(&chrono::Utc).to_rfc3339()))
}

#[cfg(feature = "server")]
pub async fn create_match(
    token: String,
    home_team: String,
    away_team: String,
    phase: String,
    kickoff: String,
    csrf_token: String,
) -> Result<MatchRecord, ServerFnError> {
    use uuid::Uuid;
    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let home = crate::security::normalize_required_text("Selecao mandante", home_team, 1, 60)?;
    let away = crate::security::normalize_required_text("Selecao visitante", away_team, 1, 60)?;
    let (phase, kickoff) = normalize_knockout_match_input(phase, kickoff)?;
    let id = Uuid::new_v4().to_string();
    let db = crate::db::pool();
    let event_id = crate::events::world_cup_2026_event_id(db).await?;
    let event_version_id = ensure_football_working_revision(&event_id, &session.user_id).await?;
    let item_id = Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|error| crate::security::internal_error("create_match_begin_tx", error))?;
    crate::prediction_items::create_football_match_item(
        &mut tx,
        &event_id,
        &event_version_id,
        &item_id,
        &home,
        &away,
        &kickoff,
    )
    .await?;
    sqlx::query("INSERT INTO matches (id,prediction_item_id,event_version_id,home_team,away_team,kickoff,group_name,phase) VALUES(?1,?2,?3,?4,?5,?6,NULL,?7)").bind(&id).bind(&item_id).bind(&event_version_id).bind(&home).bind(&away).bind(&kickoff).bind(&phase).execute(&mut *tx).await.map_err(|error| crate::security::internal_error("create_match_insert", error))?;
    tx.commit()
        .await
        .map_err(|error| crate::security::internal_error("create_match_commit", error))?;
    crate::security::append_audit_log(db, Some(&session.user_id), "match_created", "match", Some(&id), Some(&crate::security::client_ip(&headers)), serde_json::json!({ "home_team": home, "away_team": away, "phase": phase, "kickoff": kickoff })).await?;
    load_match_record(db, &id).await
}

#[cfg(feature = "server")]
pub async fn update_match_schedule(
    token: String,
    match_id: String,
    home_team: String,
    away_team: String,
    phase: String,
    kickoff: String,
    csrf_token: String,
) -> Result<MatchRecord, ServerFnError> {
    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_match_id(&match_id)?;
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let home = crate::security::normalize_required_text("Selecao mandante", home_team, 1, 60)?;
    let away = crate::security::normalize_required_text("Selecao visitante", away_team, 1, 60)?;
    let (phase, kickoff) = normalize_knockout_match_input(phase, kickoff)?;
    let db = crate::db::pool();
    let event: Option<(String,)> = sqlx::query_as("SELECT pi.event_id FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id WHERE m.id=?1").bind(&match_id).fetch_optional(db).await.map_err(|error| crate::security::internal_error("update_match_event_lookup", error))?;
    let Some((event_id,)) = event else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };
    ensure_football_working_revision(&event_id, &session.user_id).await?;
    let (target_match_id, target_item_id) = working_match_for(db, &event_id, &match_id).await?;
    let before: Option<(String, String, Option<String>, String)> =
        sqlx::query_as("SELECT home_team,away_team,phase,kickoff FROM matches WHERE id=?1")
            .bind(&target_match_id)
            .fetch_optional(db)
            .await
            .map_err(|error| {
                crate::security::internal_error("update_match_schedule_lookup", error)
            })?;
    let Some((old_home, old_away, old_phase, old_kickoff)) = before else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };
    let mut tx = db.begin().await.map_err(|error| {
        crate::security::internal_error("update_match_schedule_begin_tx", error)
    })?;
    sqlx::query("UPDATE matches SET home_team=?1,away_team=?2,phase=?3,kickoff=?4 WHERE id=?5")
        .bind(&home)
        .bind(&away)
        .bind(&phase)
        .bind(&kickoff)
        .bind(&target_match_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| crate::security::internal_error("update_match_schedule", error))?;
    sqlx::query("UPDATE prediction_items SET title=?1,lock_at=?2,reveal_at=?2,updated_at=datetime('now') WHERE id=?3").bind(crate::prediction_items::football_match_title(&home,&away)).bind(&kickoff).bind(&target_item_id).execute(&mut *tx).await.map_err(|error| crate::security::internal_error("update_match_schedule_prediction_item", error))?;
    tx.commit()
        .await
        .map_err(|error| crate::security::internal_error("update_match_schedule_commit", error))?;
    crate::security::append_audit_log(db,Some(&session.user_id),"match_schedule_updated","match",Some(&match_id),Some(&crate::security::client_ip(&headers)),serde_json::json!({ "before": { "home_team": old_home,"away_team": old_away,"phase": old_phase,"kickoff": old_kickoff }, "after": { "home_team": home,"away_team": away,"phase": phase,"kickoff": kickoff } })).await?;
    load_match_record(db, &target_match_id).await
}

#[cfg(feature = "server")]
pub async fn delete_match(
    token: String,
    match_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_match_id(&match_id)?;
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let before: Option<(String,String,Option<String>,String,String)> = sqlx::query_as("SELECT m.home_team,m.away_team,m.phase,m.prediction_item_id,pi.event_id FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id WHERE m.id=?1").bind(&match_id).fetch_optional(db).await.map_err(|error| crate::security::internal_error("delete_match_lookup", error))?;
    let Some((home, away, phase, _item_id, event_id)) = before else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };
    ensure_football_working_revision(&event_id, &session.user_id).await?;
    let (target_match_id, target_item_id) = working_match_for(db, &event_id, &match_id).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|error| crate::security::internal_error("delete_match_begin_tx", error))?;
    for (sql, context) in [("DELETE FROM prediction_score_breakdowns WHERE match_id=?1","delete_match_breakdowns"),("DELETE FROM prediction_reactions WHERE prediction_id IN (SELECT id FROM predictions WHERE match_id=?1)","delete_match_reactions"),("DELETE FROM predictions WHERE match_id=?1","delete_match_predictions"),("DELETE FROM matches WHERE id=?1","delete_match")] { sqlx::query(sql).bind(&target_match_id).execute(&mut *tx).await.map_err(|error| crate::security::internal_error(context,error))?; }
    sqlx::query("DELETE FROM prediction_items WHERE id=?1")
        .bind(&target_item_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| crate::security::internal_error("delete_match_prediction_item", error))?;
    tx.commit()
        .await
        .map_err(|error| crate::security::internal_error("delete_match_commit", error))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "match_deleted",
        "match",
        Some(&match_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "home_team": home,"away_team": away,"phase": phase }),
    )
    .await?;
    Ok(())
}
