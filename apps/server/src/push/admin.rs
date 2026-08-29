use super::*;
use crate::error::ServerFnError;

pub async fn send_admin_push_to_user(
    token: String,
    target_user_id: String,
    title: String,
    body: String,
    url: Option<String>,
    csrf_token: String,
) -> Result<AdminPushResult, ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Usuario", &target_user_id)?;
    let (title, body, url, payload) = normalize_admin_push_payload(title, body, url)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = pool();

    let target_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE id = ?1")
        .bind(&target_user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("admin_push_user_lookup", e))?;
    if target_exists.is_none() {
        return Err(crate::security::public_error("Usuario nao encontrado."));
    }

    let subscriptions: Vec<SubscriptionRow> = sqlx::query_as(
        "SELECT ps.user_id, ps.endpoint, ps.p256dh, ps.auth, ps.user_agent
         FROM push_subscriptions ps
         INNER JOIN notification_preferences np ON np.user_id = ps.user_id
         WHERE ps.user_id = ?1
           AND ps.active = 1
           AND np.enabled = 1",
    )
    .bind(&target_user_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_push_load_subscriptions", e))?;

    let delivery = if subscriptions.is_empty() {
        PushDeliverySummary::default()
    } else {
        send_payload_to_user_subscriptions_with_summary(db, &subscriptions, &payload).await?
    };

    let result = AdminPushResult {
        target_user_id: Some(target_user_id.clone()),
        target_user_count: i64::from(!subscriptions.is_empty()),
        active_subscription_count: subscriptions.len() as i64,
        attempted_count: delivery.attempted_count,
        successful_count: delivery.successful_count,
        failed_count: delivery.failed_count,
        deactivated_count: delivery.deactivated_count,
    };

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "admin_push_sent",
        "user",
        Some(&target_user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "target_user_id": target_user_id,
            "title_len": title.len(),
            "body_len": body.len(),
            "url": url,
            "active_subscription_count": result.active_subscription_count,
            "attempted_count": result.attempted_count,
            "successful_count": result.successful_count,
            "failed_count": result.failed_count,
            "deactivated_count": result.deactivated_count,
        }),
    )
    .await?;

    Ok(result)
}

#[cfg(feature = "server")]
pub async fn send_admin_push_broadcast(
    token: String,
    title: String,
    body: String,
    url: Option<String>,
    csrf_token: String,
) -> Result<AdminPushResult, ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let (title, body, url, payload) = normalize_admin_push_payload(title, body, url)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = pool();

    let subscriptions: Vec<SubscriptionRow> = sqlx::query_as(
        "SELECT ps.user_id, ps.endpoint, ps.p256dh, ps.auth, ps.user_agent
         FROM push_subscriptions ps
         INNER JOIN notification_preferences np ON np.user_id = ps.user_id
         INNER JOIN users u ON u.id = ps.user_id
         WHERE ps.active = 1
           AND np.enabled = 1
           AND u.blocked_at IS NULL",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_push_broadcast_load", e))?;

    let target_user_count = subscriptions
        .iter()
        .map(|subscription| subscription.user_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len() as i64;

    let delivery = if subscriptions.is_empty() {
        PushDeliverySummary::default()
    } else {
        send_payload_to_user_subscriptions_with_summary(db, &subscriptions, &payload).await?
    };

    let result = AdminPushResult {
        target_user_id: None,
        target_user_count,
        active_subscription_count: subscriptions.len() as i64,
        attempted_count: delivery.attempted_count,
        successful_count: delivery.successful_count,
        failed_count: delivery.failed_count,
        deactivated_count: delivery.deactivated_count,
    };

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "admin_push_broadcast_sent",
        "users",
        None,
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "title_len": title.len(),
            "body_len": body.len(),
            "url": url,
            "target_user_count": result.target_user_count,
            "active_subscription_count": result.active_subscription_count,
            "attempted_count": result.attempted_count,
            "successful_count": result.successful_count,
            "failed_count": result.failed_count,
            "deactivated_count": result.deactivated_count,
        }),
    )
    .await?;

    Ok(result)
}
