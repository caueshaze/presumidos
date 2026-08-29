use super::*;
use crate::{error::ServerFnError, models::*};

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
                (SELECT MAX(a.created_at) FROM audit_logs a WHERE a.target_type = 'match' AND a.target_id = m.id) AS last_audit_at
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
        items.retain(|item| kickoff_matches_brasilia_date(&item.match_record.kickoff, &date));
    }
    if let Some(status) = status {
        items.retain(|item| item.admin_status == status);
    }
    let _ = origin;
    Ok(items)
}

#[cfg(feature = "server")]
pub async fn list_match_audit(
    token: String,
    match_id: String,
) -> Result<Vec<AuditLogEntry>, ServerFnError> {
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
