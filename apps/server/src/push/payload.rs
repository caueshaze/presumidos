use super::*;
use crate::error::ServerFnError;
use web_push::{
    ContentEncoding, SubscriptionInfo, Urgency, WebPushClient, WebPushError, WebPushMessageBuilder,
};

pub(crate) fn reminder_target_url(match_id: &str) -> String {
    format!("/predictions?matchId={match_id}")
}

#[cfg(feature = "server")]
pub(crate) fn normalize_admin_push_url(url: Option<String>) -> Result<String, ServerFnError> {
    let Some(value) = url else {
        return Ok("/".to_string());
    };
    let value = crate::security::normalize_optional_text(value, 256)?;
    if value.is_empty() {
        return Ok("/".to_string());
    }
    if !value.starts_with('/') || value.starts_with("//") {
        return Err(crate::security::public_error(
            "Link do push deve ser um caminho interno iniciado por /.",
        ));
    }
    Ok(value)
}

#[cfg(feature = "server")]
pub(crate) fn normalize_admin_push_payload(
    title: String,
    body: String,
    url: Option<String>,
) -> Result<(String, String, String, String), ServerFnError> {
    let title = crate::security::normalize_required_text("Titulo", title, 1, 80)?;
    let body = crate::security::normalize_required_text("Mensagem", body, 1, 240)?;
    let url = normalize_admin_push_url(url)?;
    let tag = format!("admin-push-{}", uuid::Uuid::new_v4());
    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "url": url,
        "tag": tag,
    });
    let payload = serde_json::to_string(&payload)
        .map_err(|e| crate::security::internal_error("admin_push_payload", e))?;
    Ok((title, body, url, payload))
}

#[cfg(feature = "server")]
pub(crate) fn format_payload(matches: &[PendingReminder]) -> PushReminderPayload {
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
pub(crate) fn build_message_for_subscription(
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
pub(crate) async fn mark_subscription_failure(
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
pub(crate) async fn mark_subscription_sent(
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
pub(crate) fn is_terminal_push_error(error: &WebPushError) -> bool {
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
pub(crate) async fn send_payload_to_user_subscriptions(
    db: &sqlx::SqlitePool,
    subscriptions: &[SubscriptionRow],
    payload: &str,
) -> Result<bool, ServerFnError> {
    Ok(
        send_payload_to_user_subscriptions_with_summary(db, subscriptions, payload)
            .await?
            .successful_count
            > 0,
    )
}

#[cfg(feature = "server")]
pub(crate) async fn send_payload_to_user_subscriptions_with_summary(
    db: &sqlx::SqlitePool,
    subscriptions: &[SubscriptionRow],
    payload: &str,
) -> Result<PushDeliverySummary, ServerFnError> {
    let client = web_push_client()?;
    let mut summary = PushDeliverySummary {
        attempted_count: subscriptions.len() as i64,
        ..PushDeliverySummary::default()
    };

    for subscription in subscriptions {
        let message = match build_message_for_subscription(subscription, payload) {
            Ok(message) => message,
            Err(error) => {
                summary.failed_count += 1;
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
                summary.successful_count += 1;
                mark_subscription_sent(db, &subscription.endpoint).await?;
            }
            Err(error) => {
                let deactivate = is_terminal_push_error(&error);
                summary.failed_count += 1;
                if deactivate {
                    summary.deactivated_count += 1;
                }
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

    Ok(summary)
}
