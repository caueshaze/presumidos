use super::*;
use crate::{
    error::ServerFnError,
    models::{NotificationPreference, NotificationStatus, WebPushSubscriptionInput},
};

pub async fn get_notification_status(token: String) -> Result<NotificationStatus, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    let db = pool();
    let preference = load_preference(db, &session.user_id).await?;
    let active_subscription_count = active_subscription_count(db, &session.user_id).await?;

    Ok(NotificationStatus {
        web_push_enabled: crate::config::settings().web_push.enabled,
        vapid_public_key: crate::config::settings().web_push.vapid_public_key.clone(),
        preference,
        active_subscription_count,
    })
}

#[cfg(feature = "server")]
pub async fn update_notification_preference(
    token: String,
    enabled: bool,
    lead_time_minutes: i64,
    reaction_enabled: bool,
    csrf_token: String,
) -> Result<NotificationPreference, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let lead_time_minutes = validate_lead_time(lead_time_minutes)?;

    sqlx::query(
        "INSERT INTO notification_preferences
            (user_id, enabled, lead_time_minutes, reaction_enabled, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(user_id) DO UPDATE SET
            enabled = excluded.enabled,
            lead_time_minutes = excluded.lead_time_minutes,
            reaction_enabled = excluded.reaction_enabled,
            updated_at = excluded.updated_at",
    )
    .bind(&session.user_id)
    .bind(enabled)
    .bind(lead_time_minutes)
    .bind(reaction_enabled)
    .bind(sqlite_now())
    .execute(pool())
    .await
    .map_err(|e| crate::security::internal_error("update_notification_preference", e))?;

    crate::security::append_audit_log(
        pool(),
        Some(&session.user_id),
        "notification_preference_updated",
        "notification_preferences",
        Some(&session.user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "enabled": enabled,
            "lead_time_minutes": lead_time_minutes,
            "reaction_enabled": reaction_enabled,
        }),
    )
    .await?;

    Ok(NotificationPreference {
        enabled,
        lead_time_minutes,
        reaction_enabled,
    })
}

#[cfg(feature = "server")]
pub async fn upsert_push_subscription(
    token: String,
    input: WebPushSubscriptionInput,
    csrf_token: String,
) -> Result<StatusRegistration, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let subscription = normalize_subscription(input)?;

    sqlx::query(
        "INSERT INTO push_subscriptions
            (id, user_id, endpoint, p256dh, auth, expiration_time_ms, user_agent, device_label,
             active, updated_at, last_error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, NULL)
         ON CONFLICT(endpoint) DO UPDATE SET
            user_id = excluded.user_id,
            p256dh = excluded.p256dh,
            auth = excluded.auth,
            expiration_time_ms = excluded.expiration_time_ms,
            user_agent = excluded.user_agent,
            device_label = excluded.device_label,
            active = 1,
            updated_at = excluded.updated_at,
            last_error = NULL",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&session.user_id)
    .bind(&subscription.endpoint)
    .bind(&subscription.keys.p256dh)
    .bind(&subscription.keys.auth)
    .bind(subscription.expiration_time)
    .bind(&subscription.user_agent)
    .bind(&subscription.device_label)
    .bind(sqlite_now())
    .execute(pool())
    .await
    .map_err(|e| crate::security::internal_error("upsert_push_subscription", e))?;

    crate::security::append_audit_log(
        pool(),
        Some(&session.user_id),
        "push_subscription_upserted",
        "push_subscription",
        Some(&session.user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "endpoint_hash": crate::security::sensitive_value_hash(&subscription.endpoint),
            "has_user_agent": subscription.user_agent.is_some(),
        }),
    )
    .await?;

    Ok(StatusRegistration { ok: true })
}

#[cfg(feature = "server")]
pub async fn deactivate_push_subscription(
    token: String,
    endpoint: String,
    csrf_token: String,
) -> Result<StatusRegistration, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let endpoint = crate::security::normalize_required_text("Endpoint", endpoint, 1, 2048)?;

    sqlx::query(
        "UPDATE push_subscriptions
         SET active = 0, updated_at = ?1
         WHERE user_id = ?2 AND endpoint = ?3",
    )
    .bind(sqlite_now())
    .bind(&session.user_id)
    .bind(&endpoint)
    .execute(pool())
    .await
    .map_err(|e| crate::security::internal_error("deactivate_push_subscription", e))?;

    crate::security::append_audit_log(
        pool(),
        Some(&session.user_id),
        "push_subscription_deactivated",
        "push_subscription",
        Some(&session.user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "endpoint_hash": crate::security::sensitive_value_hash(&endpoint)
        }),
    )
    .await?;

    Ok(StatusRegistration { ok: true })
}
