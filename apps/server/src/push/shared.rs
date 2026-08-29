use crate::error::ServerFnError;
use crate::models::{NotificationPreference, NotificationStatus, WebPushSubscriptionInput};

#[cfg(feature = "server")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "server")]
use std::sync::OnceLock;

#[cfg(feature = "server")]
use serde::Serialize;

#[cfg(feature = "server")]
use web_push::{
    ContentEncoding, HyperWebPushClient, PartialVapidSignatureBuilder, SubscriptionInfo, Urgency,
    VapidSignatureBuilder, WebPushClient, WebPushError, WebPushMessageBuilder, URL_SAFE_NO_PAD,
};

#[cfg(feature = "server")]
static WEB_PUSH_CLIENT: OnceLock<HyperWebPushClient> = OnceLock::new();
#[cfg(feature = "server")]
static VAPID_BUILDER: OnceLock<PartialVapidSignatureBuilder> = OnceLock::new();

#[cfg(feature = "server")]
const DEFAULT_LEAD_TIME_MINUTES: i64 = 20;
#[cfg(feature = "server")]
const ALLOWED_LEAD_TIMES: [i64; 3] = [10, 20, 30];

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct PreferenceRow {
    pub(crate) enabled: bool,
    pub(crate) lead_time_minutes: i64,
    pub(crate) reaction_enabled: bool,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow, Clone)]
pub(crate) struct SubscriptionRow {
    pub(crate) user_id: String,
    pub(crate) endpoint: String,
    pub(crate) p256dh: String,
    pub(crate) auth: String,
    pub(crate) user_agent: Option<String>,
}

#[cfg(feature = "server")]
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PushDeliverySummary {
    pub(crate) attempted_count: i64,
    pub(crate) successful_count: i64,
    pub(crate) failed_count: i64,
    pub(crate) deactivated_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminPushResult {
    pub target_user_id: Option<String>,
    pub target_user_count: i64,
    pub active_subscription_count: i64,
    pub attempted_count: i64,
    pub successful_count: i64,
    pub failed_count: i64,
    pub deactivated_count: i64,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow, Clone)]
pub(crate) struct MatchCandidateRow {
    pub(crate) id: String,
    pub(crate) home_team: String,
    pub(crate) away_team: String,
    pub(crate) kickoff: String,
}

#[cfg(feature = "server")]
#[derive(Clone)]
pub(crate) struct PendingReminder {
    pub(crate) match_id: String,
    pub(crate) home_team: String,
    pub(crate) away_team: String,
    pub(crate) kickoff: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "server")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PushReminderPayload {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) url: String,
    pub(crate) tag: String,
    pub(crate) matches: Vec<PushReminderMatchPayload>,
}

#[cfg(feature = "server")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PushReminderMatchPayload {
    pub(crate) match_id: String,
    pub(crate) home_team: String,
    pub(crate) away_team: String,
    pub(crate) kickoff: String,
    pub(crate) url: String,
}

#[cfg(feature = "server")]
pub(crate) fn sqlite_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(feature = "server")]
pub(crate) fn current_preference() -> NotificationPreference {
    NotificationPreference {
        enabled: false,
        lead_time_minutes: DEFAULT_LEAD_TIME_MINUTES,
        reaction_enabled: true,
    }
}

#[cfg(feature = "server")]
pub(crate) fn validate_lead_time(lead_time_minutes: i64) -> Result<i64, ServerFnError> {
    if ALLOWED_LEAD_TIMES.contains(&lead_time_minutes) {
        Ok(lead_time_minutes)
    } else {
        Err(crate::security::public_error(
            "Antecedencia invalida. Use 10, 20 ou 30 minutos.",
        ))
    }
}

#[cfg(feature = "server")]
pub(crate) fn normalize_subscription(
    input: WebPushSubscriptionInput,
) -> Result<WebPushSubscriptionInput, ServerFnError> {
    let endpoint = crate::security::normalize_required_text("Endpoint", input.endpoint, 1, 2048)?;
    let p256dh =
        crate::security::normalize_required_text("Chave p256dh", input.keys.p256dh, 1, 512)?;
    let auth = crate::security::normalize_required_text("Chave auth", input.keys.auth, 1, 512)?;
    let user_agent = match input.user_agent {
        Some(value) => {
            let normalized = crate::security::normalize_optional_text(value, 512)?;
            (!normalized.is_empty()).then_some(normalized)
        }
        None => None,
    };
    let device_label = match input.device_label {
        Some(value) => {
            let normalized = crate::security::normalize_optional_text(value, 120)?;
            (!normalized.is_empty()).then_some(normalized)
        }
        None => None,
    };

    Ok(WebPushSubscriptionInput {
        endpoint,
        expiration_time: input.expiration_time,
        keys: crate::models::WebPushSubscriptionKeys { p256dh, auth },
        user_agent,
        device_label,
    })
}

#[cfg(feature = "server")]
pub(crate) fn web_push_client() -> Result<&'static HyperWebPushClient, ServerFnError> {
    if !crate::config::settings().web_push.enabled {
        return Err(crate::security::public_error(
            "Notificacoes web nao estao habilitadas neste ambiente.",
        ));
    }

    Ok(WEB_PUSH_CLIENT.get_or_init(HyperWebPushClient::new))
}

#[cfg(feature = "server")]
pub(crate) fn vapid_builder() -> Result<&'static PartialVapidSignatureBuilder, ServerFnError> {
    if !crate::config::settings().web_push.enabled {
        return Err(crate::security::public_error(
            "Notificacoes web nao estao habilitadas neste ambiente.",
        ));
    }

    if let Some(builder) = VAPID_BUILDER.get() {
        return Ok(builder);
    }

    let private_key = crate::config::settings()
        .web_push
        .vapid_private_key
        .as_deref()
        .ok_or_else(|| crate::security::public_error("Chave privada VAPID ausente."))?;
    let builder = VapidSignatureBuilder::from_base64_no_sub(private_key, URL_SAFE_NO_PAD)
        .map_err(|e| crate::security::internal_error("vapid_builder_init", e))?;

    // Corrida benigna: se outra thread inicializou primeiro, mantém a versão dela.
    let _ = VAPID_BUILDER.set(builder);
    Ok(VAPID_BUILDER
        .get()
        .expect("VAPID_BUILDER inicializado logo acima"))
}

#[cfg(feature = "server")]
pub(crate) async fn load_preference(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<NotificationPreference, ServerFnError> {
    let row: Option<PreferenceRow> = sqlx::query_as(
        "SELECT enabled, lead_time_minutes, reaction_enabled
         FROM notification_preferences
         WHERE user_id = ?1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("load_notification_preference", e))?;

    Ok(row
        .map(|row| NotificationPreference {
            enabled: row.enabled,
            lead_time_minutes: row.lead_time_minutes,
            reaction_enabled: row.reaction_enabled,
        })
        .unwrap_or_else(current_preference))
}

#[cfg(feature = "server")]
pub(crate) async fn active_subscription_count(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<i64, ServerFnError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)
         FROM push_subscriptions
         WHERE user_id = ?1 AND active = 1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("active_subscription_count", e))?;
    Ok(row.0)
}
