use super::*;
use crate::error::ServerFnError;

pub async fn send_reaction_notification(
    db: &sqlx::SqlitePool,
    user_id: &str,
    title: &str,
    body: &str,
    url: &str,
    tag: &str,
) -> Result<bool, ServerFnError> {
    if !crate::config::settings().web_push.enabled {
        return Ok(false);
    }

    let subscriptions: Vec<SubscriptionRow> = sqlx::query_as(
        "SELECT ps.user_id, ps.endpoint, ps.p256dh, ps.auth, ps.user_agent
         FROM push_subscriptions ps
         INNER JOIN notification_preferences np ON np.user_id = ps.user_id
         WHERE ps.user_id = ?1
           AND ps.active = 1
           AND np.enabled = 1
           AND np.reaction_enabled = 1",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("send_reaction_notification_load", e))?;

    if subscriptions.is_empty() {
        return Ok(false);
    }

    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "url": url,
        "tag": tag,
    });
    let payload = serde_json::to_string(&payload)
        .map_err(|e| crate::security::internal_error("send_reaction_notification_payload", e))?;

    send_payload_to_user_subscriptions(db, &subscriptions, &payload).await
}
