use super::super::*;
use super::time::*;
pub(crate) async fn insert_audit_log(
    db: &sqlx::SqlitePool,
    actor_user_id: Option<&str>,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    ip: Option<&str>,
    details: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip)
    .bind(details.to_string())
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("insert_audit_log", e))?;

    Ok(())
}

pub(crate) async fn insert_audit_log_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    actor_user_id: Option<&str>,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    ip: Option<&str>,
    details: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip)
    .bind(details.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|e| crate::security::internal_error("insert_audit_log_tx", e))?;

    Ok(())
}
pub(crate) async fn delete_session_by_token(
    db: &sqlx::SqlitePool,
    token: &str,
) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM sessions WHERE token = ?1")
        .bind(token)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("delete_session_by_token", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub(crate) async fn invalidate_user_sessions(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("invalidate_user_sessions", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub(crate) async fn create_session(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<AuthSession, ServerFnError> {
    let token = uuid::Uuid::new_v4().to_string();
    let csrf_token = crate::security::csrf_token();
    let now = sqlite_utc_now();
    let expires_at = sqlite_utc_after_hours(crate::config::settings().session_ttl_hours);

    sqlx::query(
        "INSERT INTO sessions
            (token, user_id, expires_at, csrf_token, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(&expires_at)
    .bind(&csrf_token)
    .bind(&now)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("create_session", e))?;

    Ok(AuthSession {
        token,
        user_id: user_id.to_string(),
        csrf_token,
        admin_reauthed_at: None,
    })
}
pub(crate) async fn touch_session(
    db: &sqlx::SqlitePool,
    session: &AuthSession,
) -> Result<(), ServerFnError> {
    let now = sqlite_utc_now();
    let expires_at = sqlite_utc_after_hours(crate::config::settings().session_ttl_hours);

    sqlx::query(
        "UPDATE sessions
         SET expires_at = ?1, last_seen_at = ?2
         WHERE token = ?3",
    )
    .bind(&expires_at)
    .bind(&now)
    .bind(&session.token)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("touch_session", e))?;

    crate::security::set_session_cookie(&session.token);
    Ok(())
}

#[cfg(feature = "server")]
pub(crate) async fn resolve_session(
    db: &sqlx::SqlitePool,
    legacy_token: &str,
    headers: &HeaderMap,
) -> Result<Option<AuthSession>, ServerFnError> {
    let cookie_token =
        crate::security::parse_cookie(headers, crate::security::session_cookie_name());
    let token = cookie_token
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| legacy_token.trim().to_string());

    if token.is_empty() {
        return Ok(None);
    }

    let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT user_id, csrf_token, expires_at, admin_reauthed_at
         FROM sessions
         WHERE token = ?1",
    )
    .bind(&token)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("resolve_session", e))?;

    let Some((user_id, csrf_token, expires_at, admin_reauthed_at)) = row else {
        crate::security::clear_session_cookie();
        return Ok(None);
    };

    let expired = parsed_sqlite_utc(&expires_at)
        .map(|value| chrono::Utc::now() >= value)
        .unwrap_or(true);

    if expired {
        delete_session_by_token(db, &token).await?;
        crate::security::clear_session_cookie();
        crate::security::log_event(
            "session_expired",
            serde_json::json!({
                "user_id": user_id,
            }),
        );
        return Ok(None);
    }

    let session = AuthSession {
        token,
        user_id,
        csrf_token,
        admin_reauthed_at,
    };

    touch_session(db, &session).await?;
    Ok(Some(session))
}

#[cfg(feature = "server")]
pub(crate) async fn load_user_public(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<UserPublic, ServerFnError> {
    let row: UserPublicRow = sqlx::query_as(
        "SELECT id, username, email, is_admin, blocked_at, blocked_reason FROM users WHERE id = ?1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("load_user_public", e))?;

    Ok(UserPublic {
        id: row.0,
        username: row.1,
        email: row.2,
        is_admin: row.3,
        blocked_at: row.4,
        blocked_reason: row.5,
    })
}
