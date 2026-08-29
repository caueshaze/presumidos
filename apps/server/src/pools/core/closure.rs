use crate::{error::ServerFnError, models::PoolLifecycleState};

async fn lifecycle_state(
    db: &sqlx::SqlitePool,
    pool_id: &str,
) -> Result<PoolLifecycleState, ServerFnError> {
    let row: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT predictions_closed_at,closed_at FROM pools WHERE id=?1")
            .bind(pool_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("pool_lifecycle_state", e))?;
    Ok(PoolLifecycleState {
        predictions_closed_at: row.0,
        closed_at: row.1,
    })
}

pub async fn close_predictions(
    token: String,
    pool_id: String,
    csrf: String,
) -> Result<PoolLifecycleState, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    crate::security::validate_uuid("Bolão", &pool_id)?;
    let db = crate::db::pool();
    let eligible: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT p.predictions_closed_at,p.closed_at FROM pools p
         JOIN events e ON e.id=p.event_id
         WHERE p.id=?1 AND p.created_by=?2 AND e.created_by=?2 AND e.kind='custom'",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("close_pool_predictions_authorization", e))?;
    let Some((already_closed, closed_at)) = eligible else {
        return Err(crate::security::public_error(
            "Somente quem criou este evento e bolão pode encerrar os palpites.",
        ));
    };
    if closed_at.is_some() || already_closed.is_some() {
        return lifecycle_state(db, &pool_id).await;
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("close_pool_predictions_begin", e))?;
    let changed = sqlx::query("UPDATE pools SET predictions_closed_at=datetime('now'),join_closed_at=COALESCE(join_closed_at,datetime('now')) WHERE id=?1 AND predictions_closed_at IS NULL AND closed_at IS NULL")
        .bind(&pool_id).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("close_pool_predictions_update", e))?;
    if changed.rows_affected() == 1 {
        sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,details_json) VALUES(?1,?2,'pool_predictions_closed','pool',?3,?4)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(&session.user_id).bind(&pool_id)
            .bind(serde_json::json!({"pool_id": pool_id}).to_string())
            .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("close_pool_predictions_audit", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("close_pool_predictions_commit", e))?;
    lifecycle_state(db, &pool_id).await
}

pub async fn close_pool(
    token: String,
    pool_id: String,
    csrf: String,
) -> Result<PoolLifecycleState, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    crate::security::validate_uuid("Bolão", &pool_id)?;
    let db = crate::db::pool();
    let state: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT p.event_version_id,p.predictions_closed_at,p.closed_at FROM pools p WHERE p.id=?1 AND p.created_by=?2",
    ).bind(&pool_id).bind(&session.user_id).fetch_optional(db).await
        .map_err(|e| crate::security::internal_error("close_pool_authorization", e))?;
    let Some((version_id, predictions_closed_at, closed_at)) = state else {
        return Err(crate::security::public_error(
            "Somente o dono do bolão pode encerrá-lo.",
        ));
    };
    if closed_at.is_some() {
        return lifecycle_state(db, &pool_id).await;
    }
    if predictions_closed_at.is_none() {
        return Err(crate::security::public_error(
            "Encerre os palpites antes de encerrar o bolão.",
        ));
    }
    let unresolved: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id=?1 AND NOT EXISTS (
            SELECT 1 FROM official_results r WHERE r.event_version_id=pi.event_version_id AND r.item_id=pi.id AND r.state IN ('resolved','not_representable')
         )",
    ).bind(&version_id).fetch_one(db).await
        .map_err(|e| crate::security::internal_error("close_pool_results", e))?;
    if unresolved.0 != 0 {
        return Err(crate::security::public_error(
            "Ainda há resultados pendentes.",
        ));
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("close_pool_begin", e))?;
    let changed = sqlx::query("UPDATE pools SET closed_at=datetime('now'),join_closed_at=COALESCE(join_closed_at,datetime('now')) WHERE id=?1 AND closed_at IS NULL")
        .bind(&pool_id).execute(&mut *tx).await.map_err(|e| crate::security::internal_error("close_pool_update", e))?;
    if changed.rows_affected() == 1 {
        sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,details_json) VALUES(?1,?2,'pool_closed','pool',?3,?4)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(&session.user_id).bind(&pool_id)
            .bind(serde_json::json!({"pool_id": pool_id}).to_string()).execute(&mut *tx).await
            .map_err(|e| crate::security::internal_error("close_pool_audit", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("close_pool_commit", e))?;
    lifecycle_state(db, &pool_id).await
}
