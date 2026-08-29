use super::*;
use crate::{error::ServerFnError, models::NotificationPreference};
use std::collections::{HashMap, HashSet};

pub(crate) async fn knockout_released() -> Result<bool, ServerFnError> {
    crate::matches::is_knockout_released().await
}

#[cfg(feature = "server")]
pub(crate) async fn load_active_subscriptions(
    db: &sqlx::SqlitePool,
) -> Result<HashMap<String, (NotificationPreference, Vec<SubscriptionRow>)>, ServerFnError> {
    let rows: Vec<(String, i64, bool, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT ps.user_id, np.lead_time_minutes, np.reaction_enabled,
                ps.endpoint, ps.p256dh, ps.auth, ps.user_agent
         FROM push_subscriptions ps
         INNER JOIN notification_preferences np ON np.user_id = ps.user_id
         WHERE ps.active = 1
           AND np.enabled = 1",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("load_active_subscriptions", e))?;

    let mut grouped = HashMap::<String, (NotificationPreference, Vec<SubscriptionRow>)>::new();
    for (user_id, lead_time_minutes, reaction_enabled, endpoint, p256dh, auth, user_agent) in rows {
        let entry = grouped.entry(user_id.clone()).or_insert_with(|| {
            (
                NotificationPreference {
                    enabled: true,
                    lead_time_minutes,
                    reaction_enabled,
                },
                Vec::new(),
            )
        });
        entry.1.push(SubscriptionRow {
            user_id,
            endpoint,
            p256dh,
            auth,
            user_agent,
        });
    }

    Ok(grouped)
}

#[cfg(all(test, feature = "server"))]
pub(crate) async fn test_active_subscription_user_ids(
    db: &sqlx::SqlitePool,
) -> Result<HashSet<String>, ServerFnError> {
    Ok(load_active_subscriptions(db).await?.into_keys().collect())
}

#[cfg(feature = "server")]
pub(crate) async fn load_candidate_matches(
    db: &sqlx::SqlitePool,
    max_cutoff: &str,
    now_rfc3339: &str,
    show_knockout: bool,
) -> Result<Vec<PendingReminder>, ServerFnError> {
    let rows: Vec<MatchCandidateRow> = if show_knockout {
        sqlx::query_as(
            "SELECT id, home_team, away_team, kickoff
             FROM matches
             WHERE kickoff > ?1 AND kickoff <= ?2
             ORDER BY kickoff ASC",
        )
        .bind(now_rfc3339)
        .bind(max_cutoff)
        .fetch_all(db)
        .await
    } else {
        sqlx::query_as(
            "SELECT id, home_team, away_team, kickoff
             FROM matches
             WHERE kickoff > ?1 AND kickoff <= ?2
               AND phase = 'Fase de grupos'
             ORDER BY kickoff ASC",
        )
        .bind(now_rfc3339)
        .bind(max_cutoff)
        .fetch_all(db)
        .await
    }
    .map_err(|e| crate::security::internal_error("load_candidate_matches", e))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let kickoff = chrono::DateTime::parse_from_rfc3339(&row.kickoff)
            .map_err(|e| crate::security::internal_error("load_candidate_matches_parse", e))?
            .with_timezone(&chrono::Utc);
        out.push(PendingReminder {
            match_id: row.id,
            home_team: row.home_team,
            away_team: row.away_team,
            kickoff,
        });
    }
    Ok(out)
}

#[cfg(feature = "server")]
pub(crate) async fn load_prediction_keys(
    db: &sqlx::SqlitePool,
    user_ids: &[String],
    match_ids: &[String],
) -> Result<HashSet<(String, String)>, ServerFnError> {
    if user_ids.is_empty() || match_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT user_id, match_id
         FROM predictions",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("load_prediction_keys", e))?;

    let user_filter: HashSet<&str> = user_ids.iter().map(String::as_str).collect();
    let match_filter: HashSet<&str> = match_ids.iter().map(String::as_str).collect();
    Ok(rows
        .into_iter()
        .filter(|(user_id, match_id)| {
            user_filter.contains(user_id.as_str()) && match_filter.contains(match_id.as_str())
        })
        .collect())
}

#[cfg(feature = "server")]
pub(crate) async fn load_delivery_keys(
    db: &sqlx::SqlitePool,
    user_ids: &[String],
    match_ids: &[String],
) -> Result<HashSet<(String, String)>, ServerFnError> {
    if user_ids.is_empty() || match_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT user_id, match_id
         FROM push_reminder_deliveries",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("load_delivery_keys", e))?;

    let user_filter: HashSet<&str> = user_ids.iter().map(String::as_str).collect();
    let match_filter: HashSet<&str> = match_ids.iter().map(String::as_str).collect();
    Ok(rows
        .into_iter()
        .filter(|(user_id, match_id)| {
            user_filter.contains(user_id.as_str()) && match_filter.contains(match_id.as_str())
        })
        .collect())
}

#[cfg(feature = "server")]
pub(crate) async fn record_deliveries(
    db: &sqlx::SqlitePool,
    user_id: &str,
    matches: &[PendingReminder],
    payload: &str,
) -> Result<(), ServerFnError> {
    for game in matches {
        sqlx::query(
            "INSERT OR IGNORE INTO push_reminder_deliveries
                (id, user_id, match_id, payload_json)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(&game.match_id)
        .bind(payload)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("record_push_delivery", e))?;
    }
    Ok(())
}
