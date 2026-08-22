use crate::error::ServerFnError;
use crate::models::{CustomPredictionValue, CustomQuestion, CustomQuestionOption};

#[cfg(feature = "server")]
pub async fn submit_single_choice_prediction(
    token: String,
    pool_id: String,
    item_id: String,
    option_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use uuid::Uuid;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = pool();
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT pi.kind, pi.lock_at, o.item_id
         FROM pools p JOIN prediction_items pi ON pi.event_id = p.event_id
         LEFT JOIN custom_question_options o ON o.id = ?3
         WHERE p.id = ?1 AND pi.id = ?2",
    )
    .bind(&pool_id)
    .bind(&item_id)
    .bind(&option_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("submit_single_choice_load", e))?;
    let Some((kind, lock_at, option_item_id)) = row else {
        return Err(crate::security::public_error("Bolao ou pergunta invalida."));
    };
    if kind != "single_choice" || option_item_id != item_id {
        return Err(crate::security::public_error(
            "Opção incompatível com a pergunta.",
        ));
    }
    let member: Option<(String,)> =
        sqlx::query_as("SELECT user_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("submit_single_choice_membership", e))?;
    if member.is_none() {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    }
    let locked: (i64,) = sqlx::query_as("SELECT datetime(?1) <= datetime('now')")
        .bind(&lock_at)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("submit_single_choice_lock", e))?;
    if locked.0 != 0 {
        return Err(crate::security::public_error(
            "Esta pergunta esta travada para palpite.",
        ));
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("submit_single_choice_begin", e))?;
    let prediction_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO predictions (id, pool_id, user_id, item_id) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(pool_id, user_id, item_id) DO UPDATE SET submitted_at = datetime('now')")
        .bind(&prediction_id).bind(&pool_id).bind(&session.user_id).bind(&item_id).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("submit_single_choice_prediction", e))?;
    let stored: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3")
            .bind(&pool_id)
            .bind(&session.user_id)
            .bind(&item_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("submit_single_choice_id", e))?;
    sqlx::query("INSERT INTO custom_prediction_values (prediction_id, option_id) VALUES (?1, ?2)
                 ON CONFLICT(prediction_id) DO UPDATE SET option_id=excluded.option_id, updated_at=datetime('now')")
        .bind(&stored.0).bind(&option_id).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("submit_single_choice_value", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("submit_single_choice_commit", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn list_custom_member_predictions(
    token: String,
    pool_id: String,
) -> Result<Vec<crate::models::CustomMemberPredictions>, ServerFnError> {
    use crate::auth::require_user;
    use std::collections::BTreeMap;
    let session = require_user(&token).await?;
    let db = crate::db::pool();
    let allowed: Option<(String,)> =
        sqlx::query_as("SELECT ?2 WHERE EXISTS (SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?2) OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| {
                crate::security::internal_error("custom_member_predictions_membership", e)
            })?;
    if allowed.is_none() {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    }
    let members: Vec<(String, String)> = sqlx::query_as("SELECT u.id,u.username FROM pool_members pm JOIN users u ON u.id=pm.user_id WHERE pm.pool_id=?1 ORDER BY u.username COLLATE NOCASE")
        .bind(&pool_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("custom_member_predictions_members", e))?;
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT pr.user_id, pi.id, pi.title, o.label FROM predictions pr
         JOIN prediction_items pi ON pi.id=pr.item_id
         JOIN custom_prediction_values cpv ON cpv.prediction_id=pr.id
         JOIN custom_question_options o ON o.id=cpv.option_id
         WHERE pr.pool_id=?1 AND datetime(pi.reveal_at)<=datetime('now') ORDER BY pi.sort_order",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("custom_member_predictions_load", e))?;
    let mut by_user: BTreeMap<String, Vec<crate::models::CustomMemberPrediction>> = BTreeMap::new();
    for (user_id, item_id, title, option_label) in rows {
        by_user
            .entry(user_id)
            .or_default()
            .push(crate::models::CustomMemberPrediction {
                item_id,
                title,
                option_label,
            });
    }
    Ok(members
        .into_iter()
        .map(
            |(user_id, username)| crate::models::CustomMemberPredictions {
                predictions: by_user.remove(&user_id).unwrap_or_default(),
                user_id,
                username,
            },
        )
        .collect())
}

#[cfg(feature = "server")]
pub async fn list_custom_questions(
    token: String,
    pool_id: String,
) -> Result<Vec<CustomQuestion>, ServerFnError> {
    use crate::auth::require_user;
    let session = require_user(&token).await?;
    let db = crate::db::pool();
    let allowed: Option<(String,)> =
        sqlx::query_as("SELECT ?2 WHERE EXISTS (SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?2) OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("list_custom_questions_membership", e))?;
    if allowed.is_none() {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    }
    let rows: Vec<(String,String,String,String,i64,String,Option<String>,Option<String>,i64,i64)> = sqlx::query_as(
        "SELECT pi.id, pi.title, pi.lock_at, pi.reveal_at, pi.sort_order, pi.status, cpv.option_id,
                CASE WHEN datetime(pi.reveal_at) <= datetime('now') THEN q.correct_option_id ELSE NULL END,
                s.correct_points, s.incorrect_points
         FROM prediction_items pi
         JOIN pools p ON p.event_id=pi.event_id
         JOIN custom_questions q ON q.item_id=pi.id
         JOIN custom_pool_item_scoring s ON s.pool_id=p.id AND s.item_id=pi.id
         LEFT JOIN predictions pr ON pr.pool_id=p.id AND pr.user_id=?2 AND pr.item_id=pi.id
         LEFT JOIN custom_prediction_values cpv ON cpv.prediction_id=pr.id
         WHERE p.id=?1 ORDER BY pi.sort_order")
        .bind(&pool_id).bind(&session.user_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("list_custom_questions", e))?;
    let mut out = Vec::new();
    for (
        item_id,
        title,
        lock_at,
        reveal_at,
        sort_order,
        stored_status,
        current_option_id,
        correct_option_id,
        correct_points,
        incorrect_points,
    ) in rows
    {
        let options=sqlx::query_as::<_,(String,String,i64)>("SELECT id,label,sort_order FROM custom_question_options WHERE item_id=?1 ORDER BY sort_order")
        .bind(&item_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("list_custom_options", e))?
        .into_iter().map(|(id,label,sort_order)| CustomQuestionOption{id,label,sort_order}).collect();
        let status = if correct_option_id.is_some() {
            crate::models::PredictionItemStatus::Resolved
        } else if chrono::DateTime::parse_from_rfc3339(&lock_at)
            .map(|value| value <= chrono::Utc::now())
            .unwrap_or(stored_status == "locked")
        {
            crate::models::PredictionItemStatus::Locked
        } else {
            crate::models::PredictionItemStatus::Open
        };
        out.push(CustomQuestion {
            item_id,
            kind: crate::models::PredictionItemKind::SingleChoice,
            title,
            lock_at,
            reveal_at,
            sort_order,
            status,
            current_option_id,
            correct_option_id,
            correct_points,
            incorrect_points,
            options,
        });
    }
    Ok(out)
}

#[cfg(feature = "server")]
pub async fn custom_prediction_value(
    prediction_id: &str,
) -> Result<Option<CustomPredictionValue>, ServerFnError> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT prediction_id, option_id FROM custom_prediction_values WHERE prediction_id=?1",
    )
    .bind(prediction_id)
    .fetch_optional(crate::db::pool())
    .await
    .map(|v| {
        v.map(|(prediction_id, option_id)| CustomPredictionValue {
            prediction_id,
            option_id,
        })
    })
    .map_err(|e| crate::security::internal_error("custom_prediction_value", e))
}

/// A resolução é armazenada, mas não muda status nem aciona scoring nesta fase.
#[cfg(feature = "server")]
pub async fn set_correct_option(item_id: &str, option_id: &str) -> Result<(), ServerFnError> {
    let changed = sqlx::query(
        "UPDATE custom_questions SET correct_option_id=?2, updated_at=datetime('now')
         WHERE item_id=?1 AND EXISTS (
             SELECT 1 FROM custom_question_options o WHERE o.id=?2 AND o.item_id=?1
         )",
    )
    .bind(item_id)
    .bind(option_id)
    .execute(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("set_correct_option", e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error(
            "Opção correta incompatível com a pergunta.",
        ));
    }
    crate::scoring::recalculate_custom_breakdowns().await?;
    Ok(())
}
