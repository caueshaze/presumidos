use super::helpers::{best_source, copyable_rows, target_is_eligible};
use crate::error::ServerFnError;
use crate::models::{PredictionReuseSource, PredictionReuseSuggestion};

#[cfg(feature = "server")]
pub async fn suggestion(
    token: String,
    pool_id: String,
) -> Result<PredictionReuseSuggestion, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let Some((version_id, _)) = target_is_eligible(db, &pool_id, &session.user_id).await? else {
        return Ok(PredictionReuseSuggestion::unavailable());
    };
    let Some(source) = best_source(db, &pool_id, &session.user_id, &version_id).await? else {
        return Ok(PredictionReuseSuggestion::unavailable());
    };
    let copyable = copyable_rows(db, &source.id, &session.user_id).await?;
    if copyable.is_empty() {
        return Ok(PredictionReuseSuggestion::unavailable());
    }
    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1")
            .bind(&version_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("prediction_reuse_total", e))?;
    Ok(PredictionReuseSuggestion {
        available: true,
        source_pool: Some(PredictionReuseSource { name: source.name }),
        answered: source.answered,
        copyable: copyable.len() as i64,
        total: total.0,
        locked: source.answered - copyable.len() as i64,
    })
}
