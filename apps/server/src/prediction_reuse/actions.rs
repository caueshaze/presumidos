use super::helpers::{best_source, copyable_rows, target_is_eligible};
use crate::error::ServerFnError;
use crate::models::PredictionReuseResult;

#[cfg(feature = "server")]
pub async fn start_empty(
    token: String,
    pool_id: String,
    csrf: String,
) -> Result<PredictionReuseResult, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let db = crate::db::pool();
    let update = sqlx::query(
        "UPDATE pool_members SET prediction_reuse_decision='started_empty',prediction_reuse_decided_at=datetime('now')
         WHERE pool_id=?1 AND user_id=?2 AND prediction_reuse_decision='undecided'
           AND NOT EXISTS(SELECT 1 FROM predictions WHERE pool_id=?1 AND user_id=?2)",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("prediction_reuse_start_empty", e))?;
    Ok(PredictionReuseResult {
        copied_count: 0,
        already_initialized: update.rows_affected() == 0,
    })
}

#[cfg(feature = "server")]
pub async fn copy(
    token: String,
    pool_id: String,
    csrf: String,
) -> Result<PredictionReuseResult, ServerFnError> {
    use uuid::Uuid;
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let db = crate::db::pool();
    let Some((version_id, _)) = target_is_eligible(db, &pool_id, &session.user_id).await? else {
        return Ok(PredictionReuseResult {
            copied_count: 0,
            already_initialized: true,
        });
    };
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("prediction_reuse_begin", e))?;
    let Some(source) = best_source(db, &pool_id, &session.user_id, &version_id).await? else {
        return Err(crate::security::public_error(
            "Não há palpites disponíveis para reutilizar.",
        ));
    };
    let rows = copyable_rows(db, &source.id, &session.user_id).await?;
    if rows.is_empty() {
        return Err(crate::security::public_error(
            "Não há palpites disponíveis para reutilizar.",
        ));
    }
    // Esta atualização é a trava da inicialização: uma única aba vence e toda a
    // cópia permanece no mesmo commit. A constraint de Prediction reforça isso.
    let gate = sqlx::query(
        "UPDATE pool_members SET prediction_reuse_decision='copied',prediction_reuse_decided_at=datetime('now')
         WHERE pool_id=?1 AND user_id=?2 AND prediction_reuse_decision='undecided'
           AND NOT EXISTS(SELECT 1 FROM predictions WHERE pool_id=?1 AND user_id=?2)",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("prediction_reuse_gate", e))?;
    if gate.rows_affected() == 0 {
        tx.rollback().await.ok();
        return Ok(PredictionReuseResult {
            copied_count: 0,
            already_initialized: true,
        });
    }
    for row in rows {
        let target_prediction_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO predictions(id,pool_id,user_id,item_id,match_id,home_score,away_score,qualifier,went_to_penalties,penalty_home_score,penalty_away_score)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        )
        .bind(&target_prediction_id).bind(&pool_id).bind(&session.user_id).bind(&row.item_id)
        .bind(&row.match_id).bind(row.home_score).bind(row.away_score).bind(&row.qualifier)
        .bind(row.went_to_penalties).bind(row.penalty_home_score).bind(row.penalty_away_score)
        .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("prediction_reuse_prediction", e))?;
        if let Some(option_id) = row.option_id {
            sqlx::query(
                "INSERT INTO custom_prediction_values(prediction_id,option_id) VALUES(?1,?2)",
            )
            .bind(&target_prediction_id)
            .bind(option_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("prediction_reuse_single", e))?;
        }
        if let Some(value_scaled) = row.value_scaled {
            sqlx::query(
                "INSERT INTO numeric_prediction_values(prediction_id,value_scaled) VALUES(?1,?2)",
            )
            .bind(&target_prediction_id)
            .bind(value_scaled)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("prediction_reuse_numeric", e))?;
        }
        if row.kind == "multiple_choice" {
            let options: Vec<(String,)> = sqlx::query_as("SELECT option_id FROM multiple_choice_prediction_options WHERE prediction_id=?1 ORDER BY option_id")
                .bind(&row.source_prediction_id).fetch_all(&mut *tx).await
                .map_err(|e| crate::security::internal_error("prediction_reuse_multiple_load", e))?;
            for (option_id,) in options {
                sqlx::query("INSERT INTO multiple_choice_prediction_options(prediction_id,option_id) VALUES(?1,?2)")
                    .bind(&target_prediction_id).bind(option_id).execute(&mut *tx).await
                    .map_err(|e| crate::security::internal_error("prediction_reuse_multiple", e))?;
            }
        }
    }
    let copied_count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("prediction_reuse_count", e))?
    .0;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("prediction_reuse_commit", e))?;
    Ok(PredictionReuseResult {
        copied_count,
        already_initialized: false,
    })
}
