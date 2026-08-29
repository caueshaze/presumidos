use super::super::*;
use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub async fn list_all_pools_admin(token: String) -> Result<Vec<PoolSummary>, ServerFnError> {
    use crate::auth::require_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    require_admin(&token).await?;

    let rows: Vec<PoolSummaryRow> = sqlx::query_as(
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm WHERE pm.pool_id = p.id) AS member_count,
                p.created_by,
                p.description,
                p.visible_rules,
                p.join_closed_at, v.name, e.slug, e.kind, e.status, e.ends_at
         FROM pools p
         JOIN events e ON e.id = p.event_id
         JOIN event_versions v ON v.id = p.event_version_id
         ORDER BY p.name COLLATE NOCASE",
    )
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_all_pools_admin", e))?;

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
                event_id: event_id.clone(),
                event: event_summary(
                    event_id.clone(),
                    event_name,
                    event_slug,
                    event_kind,
                    event_status,
                    event_ends_at,
                ),
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

/// Lista os membros de um bolão (visão de admin), independente de o admin
/// participar dele.
#[cfg(feature = "server")]
pub async fn list_pool_members_admin(
    token: String,
    pool_id: String,
) -> Result<Vec<UserPublic>, ServerFnError> {
    use crate::auth::require_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    require_admin(&token).await?;

    let rows: Vec<PoolMemberUserRow> = sqlx::query_as(
        "SELECT u.id, u.username, u.email, u.is_admin, u.blocked_at, u.blocked_reason
         FROM pool_members pm
         JOIN users u ON u.id = pm.user_id
         WHERE pm.pool_id = ?1
         ORDER BY u.username COLLATE NOCASE",
    )
    .bind(&pool_id)
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_pool_members_admin", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, username, email, is_admin, blocked_at, blocked_reason)| UserPublic {
                id,
                username,
                email,
                is_admin,
                blocked_at,
                blocked_reason,
            },
        )
        .collect())
}

/// Adiciona um usuário a um bolão já existente (visão de admin).
#[cfg(feature = "server")]
pub async fn add_pool_member_admin(
    token: String,
    pool_id: String,
    user_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    crate::security::validate_uuid("Usuario", &user_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();

    let pool_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM pools WHERE id = ?1")
        .bind(&pool_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("add_pool_member_admin_pool_lookup", e))?;
    if pool_exists.is_none() {
        return Err(crate::security::public_error("Bolao nao encontrado."));
    }

    let user_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("add_pool_member_admin_user_lookup", e))?;
    if user_exists.is_none() {
        return Err(crate::security::public_error("Usuario nao encontrado."));
    }

    sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(&pool_id)
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("add_pool_member_admin_insert", e))?;

    let _ = crate::scoring::recalculate_pool_user_breakdowns(
        &pool_id,
        &user_id,
        Some(&session.user_id),
    )
    .await?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "pool_member_added",
        "pool",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "target_user_id": user_id }),
    )
    .await?;

    Ok(())
}

/// Remove um usuário de um bolão (visão de admin).
#[cfg(feature = "server")]
pub async fn remove_pool_member_admin(
    token: String,
    pool_id: String,
    user_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    crate::security::validate_uuid("Usuario", &user_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();

    sqlx::query("DELETE FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
        .bind(&pool_id)
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("remove_pool_member_admin_delete", e))?;
    sqlx::query("DELETE FROM prediction_score_breakdowns WHERE pool_id = ?1 AND user_id = ?2")
        .bind(&pool_id)
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("remove_pool_member_admin_breakdowns", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "pool_member_removed",
        "pool",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "target_user_id": user_id }),
    )
    .await?;

    Ok(())
}
