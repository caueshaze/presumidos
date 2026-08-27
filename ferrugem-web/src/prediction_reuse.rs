//! Reuso pontual de Predictions entre Pools da mesma EventVersion.
//!
//! A operação nunca cria vínculo com a fonte: novas rows pertencem somente ao
//! Pool de destino e a transação só copia valores de Prediction.

use crate::error::ServerFnError;
use crate::models::{PredictionReuseResult, PredictionReuseSource, PredictionReuseSuggestion};

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct SourceCandidate {
    id: String,
    name: String,
    answered: i64,
}

#[cfg(feature = "server")]
async fn target_is_eligible(
    db: &sqlx::SqlitePool,
    pool_id: &str,
    user_id: &str,
) -> Result<Option<(String, String)>, ServerFnError> {
    let target: Option<(String, String, String)> = sqlx::query_as(
        "SELECT p.event_version_id, e.status, pm.prediction_reuse_decision
         FROM pool_members pm JOIN pools p ON p.id=pm.pool_id
         JOIN events e ON e.id=p.event_id
         WHERE pm.pool_id=?1 AND pm.user_id=?2",
    )
    .bind(pool_id)
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("prediction_reuse_target", e))?;
    let Some((version_id, event_status, decision)) = target else {
        return Err(crate::security::public_error(
            "Você não participa deste bolão.",
        ));
    };
    if event_status != "active" || decision != "undecided" {
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
async fn best_source(
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
async fn copyable_rows(
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
struct CopyRow {
    source_prediction_id: String,
    item_id: String,
    match_id: Option<String>,
    home_score: Option<i64>,
    away_score: Option<i64>,
    qualifier: Option<String>,
    went_to_penalties: bool,
    penalty_home_score: Option<i64>,
    penalty_away_score: Option<i64>,
    kind: String,
    lock_at: String,
    option_id: Option<String>,
    value_scaled: Option<i64>,
}

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
