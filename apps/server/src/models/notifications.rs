use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreference {
    pub enabled: bool,
    pub lead_time_minutes: i64,
    pub reaction_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebPushSubscriptionKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebPushSubscriptionInput {
    pub endpoint: String,
    pub expiration_time: Option<i64>,
    pub keys: WebPushSubscriptionKeys,
    pub user_agent: Option<String>,
    pub device_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub web_push_enabled: bool,
    pub vapid_public_key: Option<String>,
    pub preference: NotificationPreference,
    pub active_subscription_count: i64,
}
