use super::*;
use crate::{error::ServerFnError, models::*};

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
         LEFT JOIN predictions pr ON pr.user_id = u.id AND pr.pool_id = p.id AND pr.match_id = m.id
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

            let kickoff = chrono::DateTime::parse_from_rfc3339(&row.kickoff)
                .ok()?
                .with_timezone(&chrono::Utc);
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
                        item_id: String::new(),
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
