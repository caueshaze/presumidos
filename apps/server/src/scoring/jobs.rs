use crate::{
    error::ServerFnError,
    models::ScoringJob,
    scoring::{recompute_breakdowns, recompute_custom_breakdowns},
};

#[cfg(feature = "server")]
pub(super) async fn create_scoring_job(
    db: &sqlx::SqlitePool,
    scope_type: &str,
    scope_id: Option<&str>,
    triggered_by: Option<&str>,
) -> Result<String, ServerFnError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO scoring_jobs (id, scope_type, scope_id, triggered_by, status, summary_json)
         VALUES (?1, ?2, ?3, ?4, 'running', '{}')",
    )
    .bind(&job_id)
    .bind(scope_type)
    .bind(scope_id)
    .bind(triggered_by)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("create_scoring_job", e))?;
    Ok(job_id)
}

#[cfg(feature = "server")]
pub(super) async fn finish_scoring_job(
    db: &sqlx::SqlitePool,
    job_id: &str,
    status: &str,
    summary_json: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "UPDATE scoring_jobs
         SET status = ?1, finished_at = datetime('now'), summary_json = ?2
         WHERE id = ?3",
    )
    .bind(status)
    .bind(summary_json.to_string())
    .bind(job_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("finish_scoring_job", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn recalculate_custom_breakdowns() -> Result<(), ServerFnError> {
    recompute_custom_breakdowns(crate::db::pool()).await
}

#[cfg(feature = "server")]
pub async fn recalculate_match_breakdowns(
    match_id: &str,
    triggered_by: Option<&str>,
) -> Result<ScoringJob, ServerFnError> {
    let db = crate::db::pool();
    recompute_breakdowns(
        db,
        " AND pr.match_id = ?1",
        &[match_id.to_string()],
        "match",
        Some(match_id),
        triggered_by,
    )
    .await
}

#[cfg(feature = "server")]
pub async fn recalculate_all_breakdowns(
    triggered_by: Option<&str>,
) -> Result<ScoringJob, ServerFnError> {
    let db = crate::db::pool();
    sqlx::query("DELETE FROM prediction_score_breakdowns")
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("recalculate_all_breakdowns_clear", e))?;
    let job = recompute_breakdowns(db, "", &[], "all", None, triggered_by).await?;
    recompute_custom_breakdowns(db).await?;
    Ok(job)
}

#[cfg(feature = "server")]
pub async fn recalculate_pool_user_breakdowns(
    pool_id: &str,
    user_id: &str,
    triggered_by: Option<&str>,
) -> Result<ScoringJob, ServerFnError> {
    let db = crate::db::pool();
    recompute_breakdowns(
        db,
        " AND pm.pool_id = ?1 AND pm.user_id = ?2",
        &[pool_id.to_string(), user_id.to_string()],
        "pool_user",
        Some(pool_id),
        triggered_by,
    )
    .await
}
