use super::*;
use crate::{error::ServerFnError, models::*};

pub async fn list_admin_users(token: String) -> Result<Vec<AdminUserRecord>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let db = crate::db::pool();
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            bool,
            Option<String>,
            Option<String>,
            i64,
        ),
    >(
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
        .map(
            |(id, username, email, is_admin, blocked_at, blocked_reason, pool_count)| {
                AdminUserRecord {
                    user: UserPublic {
                        id,
                        username,
                        email,
                        is_admin,
                        blocked_at,
                        blocked_reason,
                    },
                    pool_count,
                }
            },
        )
        .collect())
}

#[cfg(feature = "server")]
pub async fn list_user_pools(
    token: String,
    user_id: String,
) -> Result<Vec<PoolSummary>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Usuario", &user_id)?;
    require_admin(&token).await?;
    let db = crate::db::pool();
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            i64,
            String,
            String,
            String,
            Option<String>,
            String,
            String,
            String,
            String,
            Option<String>,
        ),
    >(
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id) AS member_count,
                p.created_by, p.description, p.visible_rules, p.join_closed_at,
                e.name, e.slug, e.kind, e.status, e.ends_at
         FROM pools p
         JOIN events e ON e.id=p.event_id
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
        .map(
            |(
                id,
                event_id,
                name,
                invite_code,
                member_count,
                created_by,
                description,
                visible_rules,
                join_closed_at,
                event_name,
                event_slug,
                event_kind,
                event_status,
                event_ends_at,
            )| PoolSummary {
                id,
                event: crate::pools::event_summary(
                    event_id.clone(),
                    event_name,
                    event_slug,
                    event_kind,
                    event_status,
                    event_ends_at,
                ),
                event_id,
                name,
                invite_code,
                member_count,
                created_by,
                description,
                visible_rules,
                join_closed_at,
            },
        )
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
pub async fn unblock_user(
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
