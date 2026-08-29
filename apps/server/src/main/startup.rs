//! Preparação operacional antes de expor o servidor HTTP.

pub async fn run_housekeeping() -> Result<(), crate::error::ServerFnError> {
    let db = crate::db::pool();
    let auth_summary = crate::auth::cleanup_expired_auth_data(db).await?;
    let push_summary = crate::push::cleanup_stale_push_data(db).await?;
    let matches_force_finished = crate::matches::force_finish_matches_for_ended_events().await?;
    crate::security::log_event(
        "startup_housekeeping_completed",
        serde_json::json!({
            "expired_sessions_deleted": auth_summary.expired_sessions_deleted,
            "expired_pending_registrations_deleted": auth_summary.expired_pending_registrations_deleted,
            "expired_password_reset_codes_deleted": auth_summary.expired_password_reset_codes_deleted,
            "inactive_push_subscriptions_deleted": push_summary.inactive_subscriptions_deleted,
            "old_push_deliveries_deleted": push_summary.old_deliveries_deleted,
            "matches_force_finished": matches_force_finished,
        }),
    );
    Ok(())
}
