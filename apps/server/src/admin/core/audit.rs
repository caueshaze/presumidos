use super::*;
use crate::{error::ServerFnError, models::*};

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
        .filter(|row| {
            actor_user_id
                .as_ref()
                .is_none_or(|value| row.actor_user_id.as_deref() == Some(value.as_str()))
        })
        .filter(|row| {
            target_type
                .as_ref()
                .is_none_or(|value| row.target_type == *value)
        })
        .filter(|row| {
            target_id
                .as_ref()
                .is_none_or(|value| row.target_id.as_deref() == Some(value.as_str()))
        })
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
