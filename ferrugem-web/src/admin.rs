use crate::error::ServerFnError;
use crate::models::{
    AdminActivityItem, AdminMatchRecord, AdminOverview, AdminPredictionRow, AdminSettings,
    AdminUserRecord, AuditLogEntry, MatchRecord, PoolSummary, PredictionRecord,
    PredictionReopenOverride, ScoringJob, SyncStatus, UserPublic,
};

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct AdminMatchRow {
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
    result_source: Option<String>,
    result_synced_at: Option<String>,
    result_external_raw_status: Option<String>,
    live_updated_at: Option<String>,
    last_audit_at: Option<String>,
    // Derivado em SQL: o jogo já começou (kickoff <= agora).
    started: bool,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    actor_user_id: Option<String>,
    actor_username: Option<String>,
    action: String,
    target_type: String,
    target_id: Option<String>,
    ip_address: Option<String>,
    details_json: String,
    created_at: String,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct PredictionAdminRow {
    user_id: String,
    username: String,
    pool_id: String,
    pool_name: String,
    match_id: String,
    home_team: String,
    away_team: String,
    kickoff: String,
    phase: Option<String>,
    home_score: Option<i64>,
    away_score: Option<i64>,
    qualifier: Option<String>,
    went_to_penalties: Option<bool>,
    penalty_home_score: Option<i64>,
    penalty_away_score: Option<i64>,
    override_id: Option<String>,
    override_reason: Option<String>,
    override_reopened_by: Option<String>,
    override_expires_at: Option<String>,
    override_used_at: Option<String>,
    override_created_at: Option<String>,
    override_revoked_at: Option<String>,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct SyncRunRow {
    id: String,
    trigger_source: String,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    summary_json: String,
}

#[cfg(feature = "server")]
fn sqlite_bool(flag: bool) -> &'static str {
    if flag { "1" } else { "0" }
}

#[cfg(feature = "server")]
fn to_match_record(row: AdminMatchRow) -> AdminMatchRecord {
    // Status é puramente ciclo de vida (ao vivo / finalizado / agendado); a
    // origem do resultado (manual vs API) é filtrada à parte por `result_source`.
    // "Ao vivo" = começou e ainda não foi finalizado, espelhando o badge
    // "AO VIVO" da interface do usuário (isMatchLive).
    let admin_status = if row.finished {
        "finalized"
    } else if row.live_status.is_some() || row.started {
        "live"
    } else {
        "scheduled"
    };

    AdminMatchRecord {
        match_record: MatchRecord {
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
            result_source: row.result_source,
            result_synced_at: row.result_synced_at,
            result_external_raw_status: row.result_external_raw_status,
            live_updated_at: row.live_updated_at,
        },
        admin_status: admin_status.to_string(),
        last_audit_at: row.last_audit_at,
    }
}

#[cfg(feature = "server")]
async fn app_setting(db: &sqlx::SqlitePool, key: &str) -> Result<Option<String>, ServerFnError> {
    sqlx::query_as("SELECT value FROM app_settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(db)
        .await
        .map(|row: Option<(String,)>| row.map(|(value,)| value))
        .map_err(|e| crate::security::internal_error("admin_app_setting", e))
}

#[cfg(feature = "server")]
async fn set_app_setting(db: &sqlx::SqlitePool, key: &str, value: &str) -> Result<(), ServerFnError> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_set_app_setting", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn load_admin_settings() -> Result<AdminSettings, ServerFnError> {
    let db = crate::db::pool();
    let knockout_released = app_setting(db, "knockout_released").await?.unwrap_or_else(|| "0".to_string()) == "1";
    let auto_sync_enabled = app_setting(db, "auto_sync_enabled").await?.unwrap_or_else(|| "1".to_string()) == "1";
    let sync_interval_minutes = app_setting(db, "sync_interval_minutes")
        .await?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(10);
    let prediction_lock_minutes = app_setting(db, "prediction_lock_minutes")
        .await?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    let global_banner_enabled = app_setting(db, "global_banner_enabled").await?.unwrap_or_else(|| "0".to_string()) == "1";
    let global_banner_text = app_setting(db, "global_banner_text").await?.unwrap_or_default();

    Ok(AdminSettings {
        knockout_released,
        auto_sync_enabled,
        sync_interval_minutes,
        prediction_lock_minutes,
        global_banner_enabled,
        global_banner_text,
    })
}

#[cfg(feature = "server")]
pub async fn save_admin_settings(
    token: String,
    settings: AdminSettings,
    csrf_token: String,
) -> Result<AdminSettings, ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = crate::db::pool();
    set_app_setting(db, "knockout_released", sqlite_bool(settings.knockout_released)).await?;
    set_app_setting(db, "auto_sync_enabled", sqlite_bool(settings.auto_sync_enabled)).await?;
    set_app_setting(db, "sync_interval_minutes", &settings.sync_interval_minutes.to_string()).await?;
    set_app_setting(db, "prediction_lock_minutes", &settings.prediction_lock_minutes.to_string()).await?;
    set_app_setting(db, "global_banner_enabled", sqlite_bool(settings.global_banner_enabled)).await?;
    set_app_setting(db, "global_banner_text", &settings.global_banner_text).await?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "admin_settings_updated",
        "app_settings",
        None,
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "knockout_released": settings.knockout_released,
            "auto_sync_enabled": settings.auto_sync_enabled,
            "sync_interval_minutes": settings.sync_interval_minutes,
            "prediction_lock_minutes": settings.prediction_lock_minutes,
            "global_banner_enabled": settings.global_banner_enabled,
        }),
    )
    .await?;

    load_admin_settings().await
}

#[cfg(feature = "server")]
pub async fn prediction_lock_minutes() -> Result<i64, ServerFnError> {
    Ok(load_admin_settings().await?.prediction_lock_minutes)
}

#[cfg(feature = "server")]
pub async fn auto_sync_enabled() -> Result<bool, ServerFnError> {
    Ok(load_admin_settings().await?.auto_sync_enabled)
}

#[cfg(feature = "server")]
pub async fn sync_interval_minutes() -> Result<i64, ServerFnError> {
    Ok(load_admin_settings().await?.sync_interval_minutes)
}

#[cfg(feature = "server")]
pub async fn list_admin_matches(
    token: String,
    phase: Option<String>,
    group_name: Option<String>,
    date: Option<String>,
    status: Option<String>,
    origin: Option<String>,
) -> Result<Vec<AdminMatchRecord>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let db = crate::db::pool();

    let rows = sqlx::query_as::<_, AdminMatchRow>(
        "SELECT m.id, m.home_team, m.away_team, m.kickoff, m.group_name, m.phase,
                m.home_score, m.away_score, m.qualifier, m.went_to_penalties,
                m.penalty_home_score, m.penalty_away_score, m.finished,
                m.live_home_score, m.live_away_score, m.live_status, m.live_elapsed,
                m.result_source, m.result_synced_at, m.result_external_raw_status, m.live_updated_at,
                (SELECT MAX(a.created_at) FROM audit_logs a WHERE a.target_type = 'match' AND a.target_id = m.id) AS last_audit_at,
                (datetime(m.kickoff) <= datetime('now')) AS started
         FROM matches m
         ORDER BY datetime(m.kickoff) ASC",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_admin_matches", e))?;

    let mut items: Vec<AdminMatchRecord> = rows.into_iter().map(to_match_record).collect();
    if let Some(phase) = phase {
        items.retain(|item| item.match_record.phase.as_deref() == Some(phase.as_str()));
    }
    if let Some(group_name) = group_name {
        items.retain(|item| item.match_record.group_name.as_deref() == Some(group_name.as_str()));
    }
    if let Some(date) = date {
        items.retain(|item| item.match_record.kickoff.starts_with(&date));
    }
    if let Some(status) = status {
        items.retain(|item| item.admin_status == status);
    }
    if let Some(origin) = origin {
        items.retain(|item| item.match_record.result_source.as_deref() == Some(origin.as_str()));
    }
    Ok(items)
}

#[cfg(feature = "server")]
pub async fn list_match_audit(token: String, match_id: String) -> Result<Vec<AuditLogEntry>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    require_admin(&token).await?;
    let db = crate::db::pool();
    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT a.id, a.actor_user_id, u.username AS actor_username, a.action, a.target_type, a.target_id,
                a.ip_address, a.details_json, a.created_at
         FROM audit_logs a
         LEFT JOIN users u ON u.id = a.actor_user_id
         WHERE a.target_type = 'match' AND a.target_id = ?1
         ORDER BY datetime(a.created_at) DESC",
    )
    .bind(&match_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_match_audit", e))?;
    Ok(rows
        .into_iter()
        .map(|row| AuditLogEntry {
            id: row.id,
            actor_user_id: row.actor_user_id,
            actor_username: row.actor_username,
            action: row.action,
            target_type: row.target_type,
            target_id: row.target_id,
            ip_address: row.ip_address,
            details_json: row.details_json,
            created_at: row.created_at,
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn latest_sync_status() -> Result<Option<SyncStatus>, ServerFnError> {
    let db = crate::db::pool();
    let row = sqlx::query_as::<_, SyncRunRow>(
        "SELECT id, trigger_source, status, started_at, finished_at, summary_json
         FROM sync_runs
         ORDER BY datetime(started_at) DESC
         LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("latest_sync_status", e))?;

    Ok(row.map(|row| SyncStatus {
        id: row.id,
        status: row.status,
        trigger_source: row.trigger_source,
        started_at: row.started_at,
        finished_at: row.finished_at,
        summary_json: row.summary_json,
    }))
}

#[cfg(feature = "server")]
pub async fn run_sync_now(token: String, csrf_token: String) -> Result<SyncStatus, ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    if !crate::config::settings().football.enabled {
        return Err(crate::security::public_error("A sincronizacao externa nao esta habilitada."));
    }

    let db = crate::db::pool();
    let run_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sync_runs (id, triggered_by, trigger_source, status, summary_json)
         VALUES (?1, ?2, 'admin', 'running', '{}')",
    )
    .bind(&run_id)
    .bind(&session.user_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("run_sync_now_insert", e))?;

    let result = crate::football::run_poll_cycle().await;
    let (status, summary_json) = match result {
        Ok(active) => (
            "completed",
            serde_json::json!({ "active_window": active }),
        ),
        Err(error) => (
            "failed",
            serde_json::json!({ "error": error.to_string() }),
        ),
    };

    sqlx::query(
        "UPDATE sync_runs
         SET status = ?1, finished_at = datetime('now'), summary_json = ?2
         WHERE id = ?3",
    )
    .bind(status)
    .bind(summary_json.to_string())
    .bind(&run_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("run_sync_now_update", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "sync_run_triggered",
        "sync_run",
        Some(&run_id),
        Some(&crate::security::client_ip(&headers)),
        summary_json.clone(),
    )
    .await?;

    Ok(SyncStatus {
        id: run_id,
        status: status.to_string(),
        trigger_source: "admin".to_string(),
        started_at: String::new(),
        finished_at: None,
        summary_json: summary_json.to_string(),
    })
}

#[cfg(feature = "server")]
pub async fn list_admin_predictions(
    token: String,
    match_id: Option<String>,
    user_id: Option<String>,
    pool_id: Option<String>,
    missing_only: bool,
) -> Result<Vec<AdminPredictionRow>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let db = crate::db::pool();
    let lock_minutes = prediction_lock_minutes().await?;
    let rows = sqlx::query_as::<_, PredictionAdminRow>(
        "SELECT u.id AS user_id,
                u.username AS username,
                p.id AS pool_id,
                p.name AS pool_name,
                m.id AS match_id,
                m.home_team AS home_team,
                m.away_team AS away_team,
                m.kickoff AS kickoff,
                m.phase AS phase,
                pr.home_score AS home_score,
                pr.away_score AS away_score,
                pr.qualifier AS qualifier,
                pr.went_to_penalties AS went_to_penalties,
                pr.penalty_home_score AS penalty_home_score,
                pr.penalty_away_score AS penalty_away_score,
                o.id AS override_id,
                o.reason AS override_reason,
                o.reopened_by AS override_reopened_by,
                o.expires_at AS override_expires_at,
                o.used_at AS override_used_at,
                o.created_at AS override_created_at,
                o.revoked_at AS override_revoked_at
         FROM pool_members pm
         JOIN users u ON u.id = pm.user_id
         JOIN pools p ON p.id = pm.pool_id
         JOIN matches m
         LEFT JOIN predictions pr ON pr.user_id = u.id AND pr.match_id = m.id
         LEFT JOIN prediction_admin_overrides o
                ON o.user_id = u.id
               AND o.match_id = m.id
               AND o.revoked_at IS NULL
               AND datetime(o.expires_at) > datetime('now')
         ORDER BY datetime(m.kickoff) ASC, p.name COLLATE NOCASE, u.username COLLATE NOCASE",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_admin_predictions", e))?;

    let mut items = rows
        .into_iter()
        .filter_map(|row| {
            let row_user_id = row.user_id.clone();
            let row_match_id = row.match_id.clone();
            if let Some(ref wanted) = match_id {
                if row.match_id != *wanted {
                    return None;
                }
            }
            if let Some(ref wanted) = user_id {
                if row.user_id != *wanted {
                    return None;
                }
            }
            if let Some(ref wanted) = pool_id {
                if row.pool_id != *wanted {
                    return None;
                }
            }

            let kickoff = chrono::DateTime::parse_from_rfc3339(&row.kickoff).ok()?.with_timezone(&chrono::Utc);
            let locked_at = kickoff - chrono::Duration::minutes(lock_minutes);
            let locked = chrono::Utc::now() >= locked_at;
            let missing = row.home_score.is_none() || row.away_score.is_none();
            if missing_only && !missing {
                return None;
            }

            Some(AdminPredictionRow {
                user_id: row.user_id,
                username: row.username,
                pool_id: Some(row.pool_id),
                pool_name: Some(row.pool_name),
                match_id: row.match_id,
                home_team: row.home_team,
                away_team: row.away_team,
                kickoff: row.kickoff,
                phase: row.phase,
                prediction: if let (Some(home_score), Some(away_score), Some(went_to_penalties)) =
                    (row.home_score, row.away_score, row.went_to_penalties)
                {
                    Some(PredictionRecord {
                        match_id: String::new(),
                        home_score,
                        away_score,
                        qualifier: row.qualifier,
                        went_to_penalties,
                        penalty_home_score: row.penalty_home_score,
                        penalty_away_score: row.penalty_away_score,
                    })
                } else {
                    None
                },
                locked,
                missing,
                override_info: row.override_id.map(|id| PredictionReopenOverride {
                    id,
                    match_id: row_match_id.clone(),
                    user_id: row_user_id.clone(),
                    reason: row.override_reason.unwrap_or_default(),
                    reopened_by: row.override_reopened_by.unwrap_or_default(),
                    expires_at: row.override_expires_at.unwrap_or_default(),
                    used_at: row.override_used_at,
                    created_at: row.override_created_at.unwrap_or_default(),
                    revoked_at: row.override_revoked_at,
                }),
            })
        })
        .collect::<Vec<_>>();

    for item in &mut items {
        if let Some(prediction) = &mut item.prediction {
            prediction.match_id = item.match_id.clone();
        }
    }

    Ok(items)
}

#[cfg(feature = "server")]
pub async fn reopen_prediction(
    token: String,
    match_id: String,
    user_id: String,
    reason: String,
    expires_at: String,
    csrf_token: String,
) -> Result<PredictionReopenOverride, ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    crate::security::validate_uuid("Usuario", &user_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();

    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM prediction_admin_overrides
         WHERE match_id = ?1 AND user_id = ?2
           AND revoked_at IS NULL
           AND used_at IS NULL
           AND datetime(expires_at) > datetime('now')",
    )
    .bind(&match_id)
    .bind(&user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("reopen_prediction_existing", e))?;
    if existing.is_some() {
        return Err(crate::security::public_error("Ja existe uma reabertura ativa para esse palpite."));
    }

    let override_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO prediction_admin_overrides
            (id, match_id, user_id, reason, reopened_by, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&override_id)
    .bind(&match_id)
    .bind(&user_id)
    .bind(&reason)
    .bind(&session.user_id)
    .bind(&expires_at)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("reopen_prediction_insert", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "prediction_reopened",
        "prediction_override",
        Some(&override_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "match_id": match_id,
            "user_id": user_id,
            "expires_at": expires_at,
        }),
    )
    .await?;

    Ok(PredictionReopenOverride {
        id: override_id,
        match_id,
        user_id,
        reason,
        reopened_by: session.user_id,
        expires_at,
        used_at: None,
        created_at: String::new(),
        revoked_at: None,
    })
}

#[cfg(feature = "server")]
pub async fn revoke_prediction_reopen(
    token: String,
    override_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Reabertura", &override_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    sqlx::query(
        "UPDATE prediction_admin_overrides
         SET revoked_at = datetime('now')
         WHERE id = ?1",
    )
    .bind(&override_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("revoke_prediction_reopen", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "prediction_reopen_revoked",
        "prediction_override",
        Some(&override_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn active_prediction_override(
    match_id: &str,
    user_id: &str,
) -> Result<Option<PredictionReopenOverride>, ServerFnError> {
    let db = crate::db::pool();
    let row = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, String, Option<String>)>(
        "SELECT id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at
         FROM prediction_admin_overrides
         WHERE match_id = ?1 AND user_id = ?2
           AND revoked_at IS NULL
           AND datetime(expires_at) > datetime('now')
         ORDER BY datetime(created_at) DESC
         LIMIT 1",
    )
    .bind(match_id)
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("active_prediction_override", e))?;

    Ok(row.map(|(id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at)| {
        PredictionReopenOverride {
            id,
            match_id,
            user_id: user_id.to_string(),
            reason,
            reopened_by,
            expires_at,
            used_at,
            created_at,
            revoked_at,
        }
    }))
}

/// Reaberturas ativas do usuario autenticado (usado pela tela de palpites para
/// liberar o formulario mesmo apos o travamento padrao por horario).
#[cfg(feature = "server")]
pub async fn list_my_prediction_overrides(
    token: String,
) -> Result<Vec<PredictionReopenOverride>, ServerFnError> {
    use crate::auth::require_user;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    let db = crate::db::pool();

    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, String, Option<String>)>(
        "SELECT id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at
         FROM prediction_admin_overrides
         WHERE user_id = ?1
           AND revoked_at IS NULL
           AND datetime(expires_at) > datetime('now')
         ORDER BY datetime(created_at) DESC",
    )
    .bind(&session.user_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_my_prediction_overrides", e))?;

    Ok(rows
        .into_iter()
        .map(|(id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at)| {
            PredictionReopenOverride {
                id,
                match_id,
                user_id: session.user_id.clone(),
                reason,
                reopened_by,
                expires_at,
                used_at,
                created_at,
                revoked_at,
            }
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn mark_prediction_override_used(override_id: &str) -> Result<(), ServerFnError> {
    let db = crate::db::pool();
    sqlx::query(
        "UPDATE prediction_admin_overrides
         SET used_at = COALESCE(used_at, datetime('now'))
         WHERE id = ?1",
    )
    .bind(override_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("mark_prediction_override_used", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn list_admin_users(token: String) -> Result<Vec<AdminUserRecord>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let db = crate::db::pool();
    let rows = sqlx::query_as::<_, (String, String, String, bool, Option<String>, Option<String>, i64)>(
        "SELECT u.id, u.username, u.email, u.is_admin, u.blocked_at, u.blocked_reason,
                COUNT(pm.pool_id) AS pool_count
         FROM users u
         LEFT JOIN pool_members pm ON pm.user_id = u.id
         GROUP BY u.id
         ORDER BY u.username COLLATE NOCASE",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_admin_users", e))?;
    Ok(rows
        .into_iter()
        .map(|(id, username, email, is_admin, blocked_at, blocked_reason, pool_count)| AdminUserRecord {
            user: UserPublic {
                id,
                username,
                email,
                is_admin,
                blocked_at,
                blocked_reason,
            },
            pool_count,
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn list_user_pools(token: String, user_id: String) -> Result<Vec<PoolSummary>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Usuario", &user_id)?;
    require_admin(&token).await?;
    let db = crate::db::pool();
    let rows = sqlx::query_as::<_, (String, String, String, i64, String, String, String, Option<String>)>(
        "SELECT p.id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id) AS member_count,
                p.created_by, p.description, p.visible_rules, p.join_closed_at
         FROM pools p
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE pm.user_id = ?1
         ORDER BY p.name COLLATE NOCASE",
    )
    .bind(&user_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_user_pools", e))?;

    Ok(rows
        .into_iter()
        .map(|(id, name, invite_code, member_count, created_by, description, visible_rules, join_closed_at)| PoolSummary {
            id,
            name,
            invite_code,
            member_count,
            created_by,
            description,
            visible_rules,
            join_closed_at,
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn block_user(
    token: String,
    user_id: String,
    reason: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Usuario", &user_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    sqlx::query(
        "UPDATE users
         SET blocked_at = datetime('now'), blocked_reason = ?1, blocked_by = ?2
         WHERE id = ?3",
    )
    .bind(&reason)
    .bind(&session.user_id)
    .bind(&user_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("block_user", e))?;
    sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("block_user_sessions", e))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "user_blocked",
        "user",
        Some(&user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "reason": reason }),
    )
    .await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn unblock_user(token: String, user_id: String, csrf_token: String) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Usuario", &user_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    sqlx::query(
        "UPDATE users
         SET blocked_at = NULL, blocked_reason = NULL, blocked_by = NULL
         WHERE id = ?1",
    )
    .bind(&user_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("unblock_user", e))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "user_unblocked",
        "user",
        Some(&user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn invalidate_user_sessions_admin(
    token: String,
    user_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Usuario", &user_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("invalidate_user_sessions_admin", e))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "user_sessions_invalidated",
        "user",
        Some(&user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn trigger_user_password_reset(
    token: String,
    user_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Usuario", &user_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let row: Option<(String,)> = sqlx::query_as("SELECT email FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("trigger_user_password_reset_lookup", e))?;
    let Some((email,)) = row else {
        return Err(crate::security::public_error("Usuario nao encontrado."));
    };
    crate::auth::request_password_reset(email.clone()).await?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "admin_password_reset_triggered",
        "user",
        Some(&user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "email": email }),
    )
    .await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn list_audit(
    token: String,
    action: Option<String>,
    actor_user_id: Option<String>,
    target_type: Option<String>,
    target_id: Option<String>,
) -> Result<Vec<AuditLogEntry>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let db = crate::db::pool();
    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT a.id, a.actor_user_id, u.username AS actor_username, a.action, a.target_type,
                a.target_id, a.ip_address, a.details_json, a.created_at
         FROM audit_logs a
         LEFT JOIN users u ON u.id = a.actor_user_id
         ORDER BY datetime(a.created_at) DESC
         LIMIT 250",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_audit", e))?;

    Ok(rows
        .into_iter()
        .filter(|row| action.as_ref().is_none_or(|value| row.action == *value))
        .filter(|row| actor_user_id.as_ref().is_none_or(|value| row.actor_user_id.as_deref() == Some(value.as_str())))
        .filter(|row| target_type.as_ref().is_none_or(|value| row.target_type == *value))
        .filter(|row| target_id.as_ref().is_none_or(|value| row.target_id.as_deref() == Some(value.as_str())))
        .map(|row| AuditLogEntry {
            id: row.id,
            actor_user_id: row.actor_user_id,
            actor_username: row.actor_username,
            action: row.action,
            target_type: row.target_type,
            target_id: row.target_id,
            ip_address: row.ip_address,
            details_json: row.details_json,
            created_at: row.created_at,
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn admin_overview(token: String) -> Result<AdminOverview, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let db = crate::db::pool();

    let scheduled_matches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM matches
         WHERE finished = 0 AND home_score IS NULL AND away_score IS NULL AND live_status IS NULL",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_scheduled", e))?;
    let live_matches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM matches WHERE finished = 0 AND live_status IS NOT NULL",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_live", e))?;
    let finalized_matches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM matches WHERE finished = 1 AND result_source = 'api'",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_finalized", e))?;
    let manually_corrected_matches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM matches WHERE result_source = 'manual'",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_manual", e))?;
    let overdue_matches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM matches
         WHERE datetime(kickoff) < datetime('now')
           AND home_score IS NULL AND away_score IS NULL
           AND finished = 0",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_overdue", e))?;
    let api_conflicts: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'match_result_api_conflict'",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_conflicts", e))?;
    let pool_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("admin_overview_pools", e))?;
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("admin_overview_users", e))?;
    let blocked_user_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE blocked_at IS NOT NULL",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_blocked", e))?;
    let users_without_predictions_soon: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT pm.user_id)
         FROM pool_members pm
         JOIN matches m ON datetime(m.kickoff) BETWEEN datetime('now') AND datetime('now', '+6 hours')
         LEFT JOIN predictions pr ON pr.user_id = pm.user_id AND pr.match_id = m.id
         WHERE pr.id IS NULL",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_missing_predictions", e))?;

    let feed_rows = sqlx::query_as::<_, (String, String, Option<String>, String, Option<i64>, Option<i64>, String)>(
        "SELECT a.action, m.home_team, a.target_id, m.away_team, m.home_score, m.away_score, a.created_at
         FROM audit_logs a
         LEFT JOIN matches m ON m.id = a.target_id
         WHERE a.target_type = 'match'
         ORDER BY datetime(a.created_at) DESC
         LIMIT 8",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_feed", e))?;

    let activity_feed = feed_rows
        .into_iter()
        .map(|(action, home_team, target_id, away_team, home_score, away_score, at)| {
            let label = if let (Some(home_score), Some(away_score)) = (home_score, away_score) {
                format!("{home_team} {home_score}x{away_score} {away_team} atualizado")
            } else {
                format!("{home_team} x {away_team} atualizado")
            };
            AdminActivityItem { action, label, at, target_id }
        })
        .collect();

    Ok(AdminOverview {
        scheduled_matches: scheduled_matches.0,
        live_matches: live_matches.0,
        finalized_matches: finalized_matches.0,
        manually_corrected_matches: manually_corrected_matches.0,
        overdue_matches: overdue_matches.0,
        api_conflicts: api_conflicts.0,
        users_without_predictions_soon: users_without_predictions_soon.0,
        pool_count: pool_count.0,
        user_count: user_count.0,
        blocked_user_count: blocked_user_count.0,
        last_sync: latest_sync_status().await?,
        sync_enabled: auto_sync_enabled().await?,
        activity_feed,
    })
}

#[cfg(feature = "server")]
pub async fn admin_recalculate_match(
    token: String,
    match_id: String,
    csrf_token: String,
) -> Result<ScoringJob, ServerFnError> {
    use crate::auth::require_recent_admin;
    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    crate::scoring::recalculate_match_breakdowns(&match_id, Some(&session.user_id)).await
}

#[cfg(feature = "server")]
pub async fn admin_recalculate_all(token: String, csrf_token: String) -> Result<ScoringJob, ServerFnError> {
    use crate::auth::require_recent_admin;
    crate::security::apply_security_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    crate::scoring::recalculate_all_breakdowns(Some(&session.user_id)).await
}
