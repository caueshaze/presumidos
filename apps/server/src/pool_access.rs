//! Políticas locais de acesso do Pool. O calendário do item permanece soberano.
use crate::error::ServerFnError;

#[cfg(feature = "server")]
pub async fn can_write_predictions(pool_id: &str) -> Result<bool, ServerFnError> {
    let row: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT predictions_closed_at,closed_at FROM pools WHERE id=?1")
            .bind(pool_id)
            .fetch_optional(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("pool_prediction_write_access", e))?;
    Ok(row.is_some_and(|(predictions_closed_at, closed_at)| {
        predictions_closed_at.is_none() && closed_at.is_none()
    }))
}

#[cfg(feature = "server")]
pub async fn can_reveal_early(pool_id: &str) -> Result<bool, ServerFnError> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT predictions_closed_at FROM pools WHERE id=?1")
            .bind(pool_id)
            .fetch_optional(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("pool_prediction_reveal_access", e))?;
    Ok(row.is_some_and(|(closed_at,)| closed_at.is_some()))
}
