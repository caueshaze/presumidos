use super::*;
use crate::error::ServerFnError;

#[derive(serde::Serialize)]
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
