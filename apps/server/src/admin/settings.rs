use crate::error::ServerFnError;
use crate::models::{AdminSettings, EventKind, FeaturedPool};

fn sqlite_bool(flag: bool) -> &'static str {
    if flag {
        "1"
    } else {
        "0"
    }
}

async fn app_setting(db: &sqlx::SqlitePool, key: &str) -> Result<Option<String>, ServerFnError> {
    sqlx::query_as("SELECT value FROM app_settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(db)
        .await
        .map(|row: Option<(String,)>| row.map(|(value,)| value))
        .map_err(|e| crate::security::internal_error("admin_app_setting", e))
}

async fn set_app_setting(
    db: &sqlx::SqlitePool,
    key: &str,
    value: &str,
) -> Result<(), ServerFnError> {
    sqlx::query("INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key).bind(value).execute(db).await
        .map_err(|e| crate::security::internal_error("admin_set_app_setting", e))?;
    Ok(())
}

pub async fn load_admin_settings() -> Result<AdminSettings, ServerFnError> {
    let db = crate::db::pool();
    let value = |key| app_setting(db, key);
    let knockout_released = value("knockout_released")
        .await?
        .unwrap_or_else(|| "0".to_string())
        == "1";
    let prediction_lock_minutes = value("prediction_lock_minutes")
        .await?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let global_banner_enabled = value("global_banner_enabled")
        .await?
        .unwrap_or_else(|| "0".to_string())
        == "1";
    let global_banner_text = value("global_banner_text").await?.unwrap_or_default();
    let final_theme_enabled = value("final_theme_enabled")
        .await?
        .unwrap_or_else(|| "0".to_string())
        == "1";
    let closing_screen_enabled = value("closing_screen_enabled")
        .await?
        .unwrap_or_else(|| "0".to_string())
        == "1";
    let featured_pool_id = value("featured_pool_id")
        .await?
        .filter(|value| !value.trim().is_empty());
    let featured_pool = if let Some(pool_id) = featured_pool_id.as_deref() {
        let row: Option<(String, String, String, String, String, Option<String>, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT p.id, p.name, p.invite_code, e.name, e.kind, p.join_closed_at, e.status, e.ends_at, (SELECT COUNT(*) FROM pool_members pm WHERE pm.pool_id = p.id) FROM pools p JOIN events e ON e.id = p.event_id WHERE p.id = ?1"
        ).bind(pool_id).fetch_optional(db).await.map_err(|e| crate::security::internal_error("featured_pool_load", e))?;
        row.map(
            |(
                pool_id,
                pool_name,
                invite_code,
                event_name,
                event_kind,
                join_closed_at,
                event_status,
                ends_at,
                member_count,
            )| {
                let event_kind = if event_kind == "custom" {
                    EventKind::Custom
                } else {
                    EventKind::Football
                };
                let is_historical = event_status == "finished"
                    || ends_at
                        .as_deref()
                        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                        .is_some_and(|value| value <= chrono::Utc::now());
                let can_join = join_closed_at.is_none() && !is_historical;
                FeaturedPool {
                    pool_id,
                    pool_name,
                    event_name,
                    event_kind,
                    is_historical,
                    member_count,
                    can_join,
                    join_code: can_join.then_some(invite_code),
                }
            },
        )
    } else {
        None
    };
    Ok(AdminSettings {
        knockout_released,
        prediction_lock_minutes,
        global_banner_enabled,
        global_banner_text,
        final_theme_enabled,
        closing_screen_enabled,
        featured_pool_id,
        featured_pool,
    })
}

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
    set_app_setting(
        db,
        "knockout_released",
        sqlite_bool(settings.knockout_released),
    )
    .await?;
    set_app_setting(
        db,
        "featured_pool_id",
        settings.featured_pool_id.as_deref().unwrap_or(""),
    )
    .await?;
    set_app_setting(
        db,
        "prediction_lock_minutes",
        &settings.prediction_lock_minutes.to_string(),
    )
    .await?;
    set_app_setting(
        db,
        "global_banner_enabled",
        sqlite_bool(settings.global_banner_enabled),
    )
    .await?;
    set_app_setting(db, "global_banner_text", &settings.global_banner_text).await?;
    set_app_setting(
        db,
        "final_theme_enabled",
        sqlite_bool(settings.final_theme_enabled),
    )
    .await?;
    set_app_setting(
        db,
        "closing_screen_enabled",
        sqlite_bool(settings.closing_screen_enabled),
    )
    .await?;
    crate::security::append_audit_log(db, Some(&session.user_id), "admin_settings_updated", "app_settings", None, Some(&crate::security::client_ip(&headers)), serde_json::json!({
        "knockout_released": settings.knockout_released, "prediction_lock_minutes": settings.prediction_lock_minutes,
        "global_banner_enabled": settings.global_banner_enabled, "final_theme_enabled": settings.final_theme_enabled,
        "closing_screen_enabled": settings.closing_screen_enabled, "featured_pool_id": settings.featured_pool_id,
    })).await?;
    load_admin_settings().await
}

pub async fn prediction_lock_minutes() -> Result<i64, ServerFnError> {
    Ok(load_admin_settings().await?.prediction_lock_minutes)
}
