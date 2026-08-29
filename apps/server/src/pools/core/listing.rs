use super::*;
use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub async fn list_my_pools(token: String) -> Result<Vec<PoolSummary>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;

    let rows: Vec<PoolSummaryRow> = sqlx::query_as(
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id) AS member_count,
                p.created_by,
                p.description,
                p.visible_rules,
                p.join_closed_at,p.predictions_closed_at,p.closed_at, v.name, e.slug, e.kind, e.status, e.ends_at
         FROM pools p
         JOIN events e ON e.id = p.event_id
         JOIN event_versions v ON v.id = p.event_version_id
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE pm.user_id = ?1
         ORDER BY p.created_at DESC",
    )
    .bind(&session.user_id)
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_my_pools", e))?;

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
                predictions_closed_at,
                closed_at,
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
                predictions_closed_at,
                closed_at,
            },
        )
        .collect())
}

#[cfg(feature = "server")]
pub async fn dashboard_pools(token: String) -> Result<Vec<PoolDashboardSummary>, ServerFnError> {
    use crate::auth::require_user;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    let rows: Vec<(String, String, String, String, i64, String, Option<String>, Option<String>, Option<String>, String, String, String, String, Option<String>, i64, i64)> = sqlx::query_as(
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id),
                p.created_by, p.join_closed_at,p.predictions_closed_at,p.closed_at,
                COALESCE(v.name, e.name), e.slug, e.kind, e.status, e.ends_at,
                (SELECT COUNT(*) FROM predictions pr WHERE pr.pool_id = p.id AND pr.user_id = ?1),
                (SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id = COALESCE(p.event_version_id, e.current_published_version_id) OR (p.event_version_id IS NULL AND pi.event_id = p.event_id))
         FROM pools p
         JOIN events e ON e.id = p.event_id
         LEFT JOIN event_versions v ON v.id = COALESCE(p.event_version_id, e.current_published_version_id)
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE pm.user_id = ?1
         ORDER BY CASE WHEN e.ends_at IS NULL THEN 0 ELSE 1 END, datetime(e.ends_at) DESC, p.created_at DESC",
    )
    .bind(&session.user_id)
    .fetch_all(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("dashboard_pools", e))?;
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
                join_closed_at,
                predictions_closed_at,
                closed_at,
                event_name,
                event_slug,
                event_kind,
                event_status,
                event_ends_at,
                answered_count,
                item_count,
            )| PoolDashboardSummary {
                pool: PoolSummary {
                    id,
                    event_id: event_id.clone(),
                    event: event_summary(
                        event_id,
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
                    description: String::new(),
                    visible_rules: String::new(),
                    join_closed_at,
                    predictions_closed_at,
                    closed_at,
                },
                answered_count,
                item_count,
            },
        )
        .collect())
}
