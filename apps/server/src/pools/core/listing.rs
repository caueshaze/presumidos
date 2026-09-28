use super::*;
use crate::{error::ServerFnError, models::*};

#[derive(sqlx::FromRow)]
struct PoolDashboardRow {
    #[sqlx(flatten)]
    pool: PoolSummaryRow,
    answered_count: i64,
    item_count: i64,
}

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
                p.join_closed_at,p.predictions_closed_at,p.closed_at,p.reopened_at,
                v.name AS event_name,e.slug AS event_slug,e.kind AS event_kind,e.status AS event_status,e.ends_at AS event_ends_at
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
    let closable_event_ids: std::collections::HashSet<String> =
        sqlx::query_scalar("SELECT id FROM events WHERE created_by=?1 AND kind='custom'")
            .bind(&session.user_id)
            .fetch_all(pool())
            .await
            .map_err(|e| crate::security::internal_error("list_my_pools_capabilities", e))?
            .into_iter()
            .collect();

    Ok(rows
        .into_iter()
        .map(|row| {
            let can_close_predictions = row.event_kind == "custom"
                && (row.created_by == session.user_id
                    || closable_event_ids.contains(&row.event_id));
            PoolSummary {
                id: row.id,
                event_id: row.event_id.clone(),
                event: event_summary(
                    row.event_id,
                    row.event_name,
                    row.event_slug,
                    row.event_kind,
                    row.event_status,
                    row.event_ends_at,
                ),
                name: row.name,
                invite_code: row.invite_code,
                member_count: row.member_count,
                created_by: row.created_by,
                can_close_predictions,
                description: row.description,
                visible_rules: row.visible_rules,
                join_closed_at: row.join_closed_at,
                predictions_closed_at: row.predictions_closed_at,
                closed_at: row.closed_at,
                reopened_at: row.reopened_at,
            }
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn dashboard_pools(token: String) -> Result<Vec<PoolDashboardSummary>, ServerFnError> {
    use crate::auth::require_user;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    let rows: Vec<PoolDashboardRow> = sqlx::query_as(
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id) AS member_count,
                p.created_by,'' AS description,'' AS visible_rules,
                p.join_closed_at,p.predictions_closed_at,p.closed_at,p.reopened_at,
                COALESCE(v.name, e.name) AS event_name,e.slug AS event_slug,e.kind AS event_kind,e.status AS event_status,e.ends_at AS event_ends_at,
                (SELECT COUNT(*) FROM predictions pr WHERE pr.pool_id = p.id AND pr.user_id = ?1) AS answered_count,
                (SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id = COALESCE(p.event_version_id, e.current_published_version_id) OR (p.event_version_id IS NULL AND pi.event_id = p.event_id)) AS item_count
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
    let closable_event_ids: std::collections::HashSet<String> =
        sqlx::query_scalar("SELECT id FROM events WHERE created_by=?1 AND kind='custom'")
            .bind(&session.user_id)
            .fetch_all(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("dashboard_pools_capabilities", e))?
            .into_iter()
            .collect();
    Ok(rows
        .into_iter()
        .map(|row| {
            let pool = row.pool;
            let can_close_predictions = pool.event_kind == "custom"
                && (pool.created_by == session.user_id
                    || closable_event_ids.contains(&pool.event_id));
            PoolDashboardSummary {
                pool: PoolSummary {
                    id: pool.id,
                    event_id: pool.event_id.clone(),
                    event: event_summary(
                        pool.event_id,
                        pool.event_name,
                        pool.event_slug,
                        pool.event_kind,
                        pool.event_status,
                        pool.event_ends_at,
                    ),
                    name: pool.name,
                    invite_code: pool.invite_code,
                    member_count: pool.member_count,
                    created_by: pool.created_by,
                    can_close_predictions,
                    description: pool.description,
                    visible_rules: pool.visible_rules,
                    join_closed_at: pool.join_closed_at,
                    predictions_closed_at: pool.predictions_closed_at,
                    closed_at: pool.closed_at,
                    reopened_at: pool.reopened_at,
                },
                answered_count: row.answered_count,
                item_count: row.item_count,
            }
        })
        .collect())
}
