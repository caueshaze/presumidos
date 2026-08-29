//! Reuso pontual de Predictions entre Pools da mesma EventVersion.
//!
//! A operação nunca cria vínculo com a fonte: novas rows pertencem somente ao
//! Pool de destino e a transação só copia valores de Prediction.

use crate::error::ServerFnError;

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(super) struct SourceCandidate {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) answered: i64,
}

#[cfg(feature = "server")]
pub(super) async fn target_is_eligible(
    db: &sqlx::SqlitePool,
    pool_id: &str,
    user_id: &str,
) -> Result<Option<(String, String)>, ServerFnError> {
    let target: Option<(String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT p.event_version_id, e.status, pm.prediction_reuse_decision,p.predictions_closed_at,p.closed_at
         FROM pool_members pm JOIN pools p ON p.id=pm.pool_id
         JOIN events e ON e.id=p.event_id
         WHERE pm.pool_id=?1 AND pm.user_id=?2",
    )
    .bind(pool_id)
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("prediction_reuse_target", e))?;
    let Some((version_id, event_status, decision, predictions_closed_at, closed_at)) = target
    else {
        return Err(crate::security::public_error(
            "Você não participa deste bolão.",
        ));
    };
    if event_status != "active"
        || decision != "undecided"
        || predictions_closed_at.is_some()
        || closed_at.is_some()
    {
        return Ok(None);
    }
    let existing: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2")
            .bind(pool_id)
            .bind(user_id)
            .fetch_one(db)
            .await
            .map_err(|e| {
                crate::security::internal_error("prediction_reuse_target_predictions", e)
            })?;
    Ok((existing.0 == 0).then_some((version_id, decision)))
}

#[cfg(feature = "server")]
pub(super) async fn best_source(
    db: &sqlx::SqlitePool,
    pool_id: &str,
    user_id: &str,
    version_id: &str,
) -> Result<Option<SourceCandidate>, ServerFnError> {
    sqlx::query_as(
        "SELECT p.id,p.name,COUNT(pr.id) AS answered
         FROM pool_members pm
         JOIN pools p ON p.id=pm.pool_id
         JOIN predictions pr ON pr.pool_id=p.id AND pr.user_id=pm.user_id
         JOIN prediction_items pi ON pi.id=pr.item_id AND pi.event_version_id=?3
         WHERE pm.user_id=?2 AND p.id<>?1 AND p.event_version_id=?3
         GROUP BY p.id,p.name
         ORDER BY COUNT(pr.id) DESC, MAX(pr.submitted_at) DESC, p.id ASC
         LIMIT 1",
    )
    .bind(pool_id)
    .bind(user_id)
    .bind(version_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("prediction_reuse_source", e))
}

#[cfg(feature = "server")]
pub(super) async fn copyable_rows(
    db: &sqlx::SqlitePool,
    source_pool_id: &str,
    user_id: &str,
) -> Result<Vec<CopyRow>, ServerFnError> {
    let rows: Vec<CopyRow> = sqlx::query_as(
        "SELECT pr.id AS source_prediction_id,pr.item_id,pr.match_id,pr.home_score,pr.away_score,
                pr.qualifier,pr.went_to_penalties,pr.penalty_home_score,pr.penalty_away_score,
                pi.kind,pi.lock_at,cpv.option_id,npv.value_scaled
         FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id
         LEFT JOIN custom_prediction_values cpv ON cpv.prediction_id=pr.id
         LEFT JOIN numeric_prediction_values npv ON npv.prediction_id=pr.id
         WHERE pr.pool_id=?1 AND pr.user_id=?2
         ORDER BY pi.sort_order,pr.id",
    )
    .bind(source_pool_id)
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("prediction_reuse_rows", e))?;

    let mut copyable = Vec::new();
    for row in rows {
        if crate::prediction_access::can_edit_item(
            &row.kind,
            &row.lock_at,
            row.match_id.as_deref(),
            user_id,
        )
        .await?
        .is_some()
        {
            copyable.push(row);
        }
    }
    Ok(copyable)
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(super) struct CopyRow {
    pub(super) source_prediction_id: String,
    pub(super) item_id: String,
    pub(super) match_id: Option<String>,
    pub(super) home_score: Option<i64>,
    pub(super) away_score: Option<i64>,
    pub(super) qualifier: Option<String>,
    pub(super) went_to_penalties: bool,
    pub(super) penalty_home_score: Option<i64>,
    pub(super) penalty_away_score: Option<i64>,
    pub(super) kind: String,
    pub(super) lock_at: String,
    pub(super) option_id: Option<String>,
    pub(super) value_scaled: Option<i64>,
}
