//! Regra compartilhada de escrita de Prediction. A cópia da Fase 18 usa esta
//! mesma regra; `reveal_at` nunca concede autorização de escrita.
use crate::error::ServerFnError;

#[cfg(feature = "server")]
pub async fn can_edit_item(
    kind: &str,
    lock_at: &str,
    match_id: Option<&str>,
    user_id: &str,
) -> Result<Option<String>, ServerFnError> {
    if kind != "football_match" {
        let locked: (i64,) = sqlx::query_as("SELECT datetime(?1)<=datetime('now')")
            .bind(lock_at)
            .fetch_one(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("prediction_edit_lock", e))?;
        return Ok((locked.0 == 0).then_some(String::new()));
    }
    let lock_time = chrono::DateTime::parse_from_rfc3339(lock_at)
        .map_err(|e| crate::security::internal_error("prediction_edit_parse_lock", e))?;
    let locked_at = lock_time.with_timezone(&chrono::Utc)
        - chrono::Duration::minutes(crate::admin::prediction_lock_minutes().await?);
    if chrono::Utc::now() < locked_at {
        return Ok(Some(String::new()));
    }
    let Some(match_id) = match_id else {
        return Ok(None);
    };
    Ok(crate::admin::active_prediction_override(match_id, user_id)
        .await?
        .map(|value| value.id))
}
