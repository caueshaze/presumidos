use crate::error::ServerFnError;
use crate::models::{NotificationPreference, NotificationStatus, WebPushSubscriptionInput};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushCleanupSummary {
    pub inactive_subscriptions_deleted: u64,
    pub old_deliveries_deleted: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusRegistration {
    pub ok: bool,
}

fn disabled_error() -> ServerFnError {
    crate::security::public_error("Notificacoes web nao estao disponiveis neste build.")
}

pub async fn get_notification_status(_token: String) -> Result<NotificationStatus, ServerFnError> {
    Ok(NotificationStatus {
        web_push_enabled: false,
        vapid_public_key: None,
        preference: NotificationPreference {
            enabled: false,
            lead_time_minutes: 20,
            reaction_enabled: true,
        },
        active_subscription_count: 0,
    })
}

pub async fn update_notification_preference(
    _token: String,
    _enabled: bool,
    _lead_time_minutes: i64,
    _reaction_enabled: bool,
    _csrf_token: String,
) -> Result<NotificationPreference, ServerFnError> {
    Err(disabled_error())
}

pub async fn upsert_push_subscription(
    _token: String,
    _input: WebPushSubscriptionInput,
    _csrf_token: String,
) -> Result<StatusRegistration, ServerFnError> {
    Err(disabled_error())
}

pub async fn deactivate_push_subscription(
    _token: String,
    _endpoint: String,
    _csrf_token: String,
) -> Result<StatusRegistration, ServerFnError> {
    Err(disabled_error())
}

pub async fn cleanup_stale_push_data(
    _db: &sqlx::SqlitePool,
) -> Result<PushCleanupSummary, ServerFnError> {
    Ok(PushCleanupSummary::default())
}

pub async fn send_reaction_notification(
    _db: &sqlx::SqlitePool,
    _user_id: &str,
    _title: &str,
    _body: &str,
    _url: &str,
    _tag: &str,
) -> Result<bool, ServerFnError> {
    Ok(false)
}

#[cfg(all(test, feature = "server"))]
pub(crate) async fn test_active_subscription_user_ids(
    _db: &sqlx::SqlitePool,
) -> Result<std::collections::HashSet<String>, ServerFnError> {
    Ok(std::collections::HashSet::new())
}

pub fn spawn_reminder_worker() {}
