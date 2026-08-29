use crate::{error::ServerFnError, models::*};

pub async fn admin_overview(token: String) -> Result<AdminOverview, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let db = crate::db::pool();

    let scheduled_matches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM matches
         WHERE finished = 0 AND home_score IS NULL AND away_score IS NULL",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_scheduled", e))?;
    let finalized_matches: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM matches WHERE finished = 1")
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("admin_overview_finalized", e))?;
    let overdue_matches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM matches
         WHERE datetime(kickoff) < datetime('now')
           AND home_score IS NULL AND away_score IS NULL
           AND finished = 0",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_overdue", e))?;
    let pool_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("admin_overview_pools", e))?;
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("admin_overview_users", e))?;
    let blocked_user_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE blocked_at IS NOT NULL")
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("admin_overview_blocked", e))?;
    let users_without_predictions_soon: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT pm.user_id)
         FROM pool_members pm
         JOIN matches m ON datetime(m.kickoff) BETWEEN datetime('now') AND datetime('now', '+6 hours')
         LEFT JOIN predictions pr ON pr.user_id = pm.user_id AND pr.pool_id = pm.pool_id AND pr.match_id = m.id
         WHERE pr.id IS NULL",
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_overview_missing_predictions", e))?;

    let feed_rows = sqlx::query_as::<_, (String, String, String, Option<String>, String, Option<i64>, Option<i64>, String)>(
        "SELECT a.id, a.action, m.home_team, a.target_id, m.away_team, m.home_score, m.away_score, a.created_at
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
        .map(
            |(id, action, home_team, target_id, away_team, home_score, away_score, at)| {
                let label = if let (Some(home_score), Some(away_score)) = (home_score, away_score) {
                    format!("{home_team} {home_score}x{away_score} {away_team} atualizado")
                } else {
                    format!("{home_team} x {away_team} atualizado")
                };
                AdminActivityItem {
                    id,
                    action,
                    label,
                    at,
                    target_id,
                }
            },
        )
        .collect();

    Ok(AdminOverview {
        scheduled_matches: scheduled_matches.0,
        finalized_matches: finalized_matches.0,
        overdue_matches: overdue_matches.0,
        users_without_predictions_soon: users_without_predictions_soon.0,
        pool_count: pool_count.0,
        user_count: user_count.0,
        blocked_user_count: blocked_user_count.0,
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
pub async fn admin_recalculate_all(
    token: String,
    csrf_token: String,
) -> Result<ScoringJob, ServerFnError> {
    use crate::auth::require_recent_admin;
    crate::security::apply_security_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    crate::scoring::recalculate_all_breakdowns(Some(&session.user_id)).await
}
