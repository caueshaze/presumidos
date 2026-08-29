use super::*;
use crate::error::ServerFnError;

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
    use super::{format_payload, normalize_admin_push_url, validate_lead_time, PendingReminder};
    use chrono::{Duration, Utc};

    #[test]
    fn validates_allowed_lead_times() {
        assert!(validate_lead_time(10).is_ok());
        assert!(validate_lead_time(20).is_ok());
        assert!(validate_lead_time(30).is_ok());
        assert!(validate_lead_time(25).is_err());
    }

    #[test]
    fn admin_push_url_must_be_internal_path() {
        assert_eq!(normalize_admin_push_url(None).unwrap(), "/");
        assert_eq!(
            normalize_admin_push_url(Some(" /predictions ".to_string())).unwrap(),
            "/predictions"
        );
        assert!(normalize_admin_push_url(Some("https://example.com".to_string())).is_err());
        assert!(normalize_admin_push_url(Some("//example.com".to_string())).is_err());
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
