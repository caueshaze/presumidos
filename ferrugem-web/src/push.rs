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
struct PreferenceRow {
    enabled: bool,
    lead_time_minutes: i64,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow, Clone)]
struct SubscriptionRow {
    user_id: String,
    endpoint: String,
    p256dh: String,
    auth: String,
    user_agent: Option<String>,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow, Clone)]
struct MatchCandidateRow {
    id: String,
    home_team: String,
    away_team: String,
    kickoff: String,
}

#[cfg(feature = "server")]
#[derive(Clone)]
struct PendingReminder {
    match_id: String,
    home_team: String,
    away_team: String,
    kickoff: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "server")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PushReminderPayload {
    title: String,
    body: String,
    url: String,
    tag: String,
    matches: Vec<PushReminderMatchPayload>,
}

#[cfg(feature = "server")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PushReminderMatchPayload {
    match_id: String,
    home_team: String,
    away_team: String,
    kickoff: String,
    url: String,
}

#[cfg(feature = "server")]
fn sqlite_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(feature = "server")]
fn current_preference() -> NotificationPreference {
    NotificationPreference {
        enabled: false,
        lead_time_minutes: DEFAULT_LEAD_TIME_MINUTES,
    }
}

#[cfg(feature = "server")]
fn validate_lead_time(lead_time_minutes: i64) -> Result<i64, ServerFnError> {
    if ALLOWED_LEAD_TIMES.contains(&lead_time_minutes) {
        Ok(lead_time_minutes)
    } else {
        Err(crate::security::public_error(
            "Antecedencia invalida. Use 10, 20 ou 30 minutos.",
        ))
    }
}

#[cfg(feature = "server")]
fn normalize_subscription(
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
fn web_push_client() -> Result<&'static HyperWebPushClient, ServerFnError> {
    if !crate::config::settings().web_push.enabled {
        return Err(crate::security::public_error(
            "Notificacoes web nao estao habilitadas neste ambiente.",
        ));
    }

    Ok(WEB_PUSH_CLIENT.get_or_init(HyperWebPushClient::new))
}

#[cfg(feature = "server")]
fn vapid_builder() -> Result<&'static PartialVapidSignatureBuilder, ServerFnError> {
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
async fn load_preference(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<NotificationPreference, ServerFnError> {
    let row: Option<PreferenceRow> = sqlx::query_as(
        "SELECT enabled, lead_time_minutes
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
        })
        .unwrap_or_else(current_preference))
}

#[cfg(feature = "server")]
async fn active_subscription_count(
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

#[cfg(feature = "server")]
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
        "INSERT INTO notification_preferences (user_id, enabled, lead_time_minutes, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(user_id) DO UPDATE SET
            enabled = excluded.enabled,
            lead_time_minutes = excluded.lead_time_minutes,
            updated_at = excluded.updated_at",
    )
    .bind(&session.user_id)
    .bind(enabled)
    .bind(lead_time_minutes)
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
        }),
    )
    .await?;

    Ok(NotificationPreference {
        enabled,
        lead_time_minutes,
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

#[cfg(feature = "server")]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusRegistration {
    pub ok: bool,
}

#[cfg(feature = "server")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PushCleanupSummary {
    pub inactive_subscriptions_deleted: u64,
    pub old_deliveries_deleted: u64,
}

#[cfg(feature = "server")]
pub async fn cleanup_stale_push_data(
    db: &sqlx::SqlitePool,
) -> Result<PushCleanupSummary, ServerFnError> {
    let inactive_subscriptions_deleted = sqlx::query(
        "DELETE FROM push_subscriptions
         WHERE active = 0
           AND datetime(updated_at) <= datetime('now', '-30 days')",
    )
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("cleanup_inactive_push_subscriptions", e))?
    .rows_affected();

    let old_deliveries_deleted = sqlx::query(
        "DELETE FROM push_reminder_deliveries
         WHERE datetime(sent_at) <= datetime('now', '-30 days')",
    )
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("cleanup_old_push_deliveries", e))?
    .rows_affected();

    Ok(PushCleanupSummary {
        inactive_subscriptions_deleted,
        old_deliveries_deleted,
    })
}

#[cfg(feature = "server")]
fn reminder_target_url(match_id: &str) -> String {
    format!("/predictions?matchId={match_id}")
}

#[cfg(feature = "server")]
fn format_payload(matches: &[PendingReminder]) -> PushReminderPayload {
    let primary = &matches[0];
    let tag = if matches.len() == 1 {
        format!("prediction-reminder-{}", primary.match_id)
    } else {
        format!("prediction-reminder-batch-{}", primary.match_id)
    };
    let title = if matches.len() == 1 {
        format!(
            "Palpite pendente: {} x {}",
            primary.home_team, primary.away_team
        )
    } else {
        format!("Voce tem {} palpites pendentes", matches.len())
    };
    let body = if matches.len() == 1 {
        "Falta pouco para o jogo. Envie seu palpite antes do apito inicial.".to_string()
    } else {
        format!(
            "{} x {} e mais {} jogo(s) vao comecar em breve.",
            primary.home_team,
            primary.away_team,
            matches.len() - 1
        )
    };

    PushReminderPayload {
        title,
        body,
        url: reminder_target_url(&primary.match_id),
        tag,
        matches: matches
            .iter()
            .map(|game| PushReminderMatchPayload {
                match_id: game.match_id.clone(),
                home_team: game.home_team.clone(),
                away_team: game.away_team.clone(),
                kickoff: game.kickoff.to_rfc3339(),
                url: reminder_target_url(&game.match_id),
            })
            .collect(),
    }
}

#[cfg(feature = "server")]
fn build_message_for_subscription(
    subscription: &SubscriptionRow,
    payload: &str,
) -> Result<web_push::WebPushMessage, ServerFnError> {
    let subscription_info = SubscriptionInfo::new(
        subscription.endpoint.clone(),
        subscription.p256dh.clone(),
        subscription.auth.clone(),
    );

    let mut sig_builder = vapid_builder()?.clone().add_sub_info(&subscription_info);
    let contact_email = crate::config::settings()
        .web_push
        .contact_email
        .as_deref()
        .ok_or_else(|| crate::security::public_error("Email de contato VAPID ausente."))?;
    sig_builder.add_claim("sub", format!("mailto:{contact_email}"));
    let signature = sig_builder
        .build()
        .map_err(|e| crate::security::internal_error("build_vapid_signature", e))?;

    let mut builder = WebPushMessageBuilder::new(&subscription_info);
    builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
    builder.set_vapid_signature(signature);
    builder.set_ttl(60 * 30);
    builder.set_urgency(Urgency::High);
    builder
        .build()
        .map_err(|e| crate::security::internal_error("build_web_push_message", e))
}

#[cfg(feature = "server")]
async fn mark_subscription_failure(
    db: &sqlx::SqlitePool,
    endpoint: &str,
    deactivate: bool,
    error: &WebPushError,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "UPDATE push_subscriptions
         SET active = CASE WHEN ?1 THEN 0 ELSE active END,
             updated_at = ?2,
             last_error = ?3
         WHERE endpoint = ?4",
    )
    .bind(deactivate)
    .bind(sqlite_now())
    .bind(error.short_description())
    .bind(endpoint)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("mark_subscription_failure", e))?;
    Ok(())
}

#[cfg(feature = "server")]
async fn mark_subscription_sent(
    db: &sqlx::SqlitePool,
    endpoint: &str,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "UPDATE push_subscriptions
         SET last_sent_at = ?1, updated_at = ?1, last_error = NULL
         WHERE endpoint = ?2",
    )
    .bind(sqlite_now())
    .bind(endpoint)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("mark_subscription_sent", e))?;
    Ok(())
}

#[cfg(feature = "server")]
fn is_terminal_push_error(error: &WebPushError) -> bool {
    matches!(
        error,
        WebPushError::EndpointNotValid
            | WebPushError::EndpointNotFound
            | WebPushError::InvalidCryptoKeys
            | WebPushError::Unauthorized
            | WebPushError::BadRequest(_)
    )
}

#[cfg(feature = "server")]
async fn send_payload_to_user_subscriptions(
    db: &sqlx::SqlitePool,
    subscriptions: &[SubscriptionRow],
    payload: &str,
) -> Result<bool, ServerFnError> {
    let client = web_push_client()?;
    let mut any_success = false;

    for subscription in subscriptions {
        let message = match build_message_for_subscription(subscription, payload) {
            Ok(message) => message,
            Err(error) => {
                crate::security::log_event(
                    "web_push_message_build_failed",
                    serde_json::json!({
                        "endpoint": subscription.endpoint,
                        "user_id": subscription.user_id,
                        "error": error.message(),
                    }),
                );
                continue;
            }
        };

        match client.send(message).await {
            Ok(()) => {
                any_success = true;
                mark_subscription_sent(db, &subscription.endpoint).await?;
            }
            Err(error) => {
                let deactivate = is_terminal_push_error(&error);
                mark_subscription_failure(db, &subscription.endpoint, deactivate, &error).await?;
                crate::security::log_event(
                    "web_push_send_failed",
                    serde_json::json!({
                        "endpoint": subscription.endpoint,
                        "user_id": subscription.user_id,
                        "error": error.short_description(),
                        "deactivated": deactivate,
                        "has_user_agent": subscription.user_agent.is_some(),
                    }),
                );
            }
        }
    }

    Ok(any_success)
}

#[cfg(feature = "server")]
async fn knockout_released() -> Result<bool, ServerFnError> {
    crate::matches::is_knockout_released().await
}

#[cfg(feature = "server")]
async fn load_active_subscriptions(
    db: &sqlx::SqlitePool,
) -> Result<HashMap<String, (NotificationPreference, Vec<SubscriptionRow>)>, ServerFnError> {
    let rows: Vec<(String, i64, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT ps.user_id, np.lead_time_minutes, ps.endpoint, ps.p256dh, ps.auth, ps.user_agent
         FROM push_subscriptions ps
         INNER JOIN notification_preferences np ON np.user_id = ps.user_id
         WHERE ps.active = 1
           AND np.enabled = 1",
    )
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("load_active_subscriptions", e))?;

    let mut grouped = HashMap::<String, (NotificationPreference, Vec<SubscriptionRow>)>::new();
    for (user_id, lead_time_minutes, endpoint, p256dh, auth, user_agent) in rows {
        let entry = grouped.entry(user_id.clone()).or_insert_with(|| {
            (
                NotificationPreference {
                    enabled: true,
                    lead_time_minutes,
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
async fn load_candidate_matches(
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
async fn load_prediction_keys(
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
async fn load_delivery_keys(
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
async fn record_deliveries(
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

#[cfg(feature = "server")]
async fn run_reminder_cycle() -> Result<(), ServerFnError> {
    use crate::db::pool;

    if !crate::config::settings().web_push.enabled {
        return Ok(());
    }

    let db = pool();
    let subscriptions_by_user = load_active_subscriptions(db).await?;
    if subscriptions_by_user.is_empty() {
        return Ok(());
    }

    let now = chrono::Utc::now();
    let now_rfc3339 = now.to_rfc3339();
    let max_cutoff = (now + chrono::Duration::minutes(30)).to_rfc3339();
    let show_knockout = knockout_released().await?;
    let matches = load_candidate_matches(db, &max_cutoff, &now_rfc3339, show_knockout).await?;
    if matches.is_empty() {
        return Ok(());
    }

    let user_ids: Vec<String> = subscriptions_by_user.keys().cloned().collect();
    let match_ids: Vec<String> = matches.iter().map(|game| game.match_id.clone()).collect();
    let predicted = load_prediction_keys(db, &user_ids, &match_ids).await?;
    let delivered = load_delivery_keys(db, &user_ids, &match_ids).await?;

    for (user_id, (preference, subscriptions)) in subscriptions_by_user {
        let mut pending = matches
            .iter()
            .filter(|game| {
                let minutes_until = (game.kickoff - now).num_minutes();
                minutes_until >= 0
                    && minutes_until <= preference.lead_time_minutes
                    && !predicted.contains(&(user_id.clone(), game.match_id.clone()))
                    && !delivered.contains(&(user_id.clone(), game.match_id.clone()))
            })
            .cloned()
            .collect::<Vec<_>>();

        if pending.is_empty() {
            continue;
        }

        pending.sort_by_key(|game| game.kickoff);
        let payload = serde_json::to_string(&format_payload(&pending))
            .map_err(|e| crate::security::internal_error("serialize_push_payload", e))?;

        if send_payload_to_user_subscriptions(db, &subscriptions, &payload).await? {
            record_deliveries(db, &user_id, &pending, &payload).await?;
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
pub fn spawn_reminder_worker() {
    let interval_secs = crate::config::settings().web_push.poll_interval_secs;
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        eprintln!("[web-push] worker iniciado (intervalo {interval_secs}s)");
        loop {
            ticker.tick().await;
            if let Err(error) = run_reminder_cycle().await {
                crate::security::log_event(
                    "web_push_cycle_failed",
                    serde_json::json!({ "error": error.message() }),
                );
            }
        }
    });
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{format_payload, validate_lead_time, PendingReminder};
    use chrono::{Duration, Utc};

    #[test]
    fn validates_allowed_lead_times() {
        assert!(validate_lead_time(10).is_ok());
        assert!(validate_lead_time(20).is_ok());
        assert!(validate_lead_time(30).is_ok());
        assert!(validate_lead_time(25).is_err());
    }

    #[test]
    fn consolidates_payload_for_multiple_matches() {
        let now = Utc::now();
        let payload = format_payload(&[
            PendingReminder {
                match_id: "jogo-001".to_string(),
                home_team: "Brasil".to_string(),
                away_team: "Argentina".to_string(),
                kickoff: now + Duration::minutes(10),
            },
            PendingReminder {
                match_id: "jogo-002".to_string(),
                home_team: "Espanha".to_string(),
                away_team: "Franca".to_string(),
                kickoff: now + Duration::minutes(12),
            },
        ]);

        assert_eq!(payload.url, "/predictions?matchId=jogo-001");
        assert_eq!(payload.matches.len(), 2);
        assert!(payload.title.contains("2"));
    }
}
