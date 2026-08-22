use crate::error::ServerFnError;
use crate::models::{CustomPredictionValue, CustomQuestion, CustomQuestionOption};

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct QuestionRow {
    item_id: String,
    kind: String,
    title: String,
    lock_at: String,
    reveal_at: String,
    sort_order: i64,
    stored_status: String,
    current_option_id: Option<String>,
    correct_option_id: Option<String>,
    decimal_places: Option<i64>,
    unit_label: Option<String>,
    min_scaled: Option<i64>,
    max_scaled: Option<i64>,
    current_scaled: Option<i64>,
    result_scaled: Option<i64>,
    correct_points: Option<i64>,
    incorrect_points: Option<i64>,
    exact_points: Option<i64>,
    tolerance_scaled: Option<i64>,
    within_tolerance_points: Option<i64>,
    min_selections: Option<i64>,
    max_selections: Option<i64>,
    multiple_exact_points: Option<i64>,
    partial_points: Option<i64>,
    multiple_incorrect_points: Option<i64>,
    multiple_resolved: i64,
}

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
    let mut rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT pr.user_id,pi.id,pi.title,o.label FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id JOIN custom_prediction_values cpv ON cpv.prediction_id=pr.id JOIN custom_question_options o ON o.id=cpv.option_id WHERE pr.pool_id=?1 AND datetime(pi.reveal_at)<=datetime('now')
         ORDER BY pi.sort_order",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("custom_member_predictions_load", e))?;
    let numeric: Vec<(String,String,String,i64,i64,Option<String>)> = sqlx::query_as(
        "SELECT pr.user_id,pi.id,pi.title,v.value_scaled,n.decimal_places,n.unit_label FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id JOIN numeric_prediction_values v ON v.prediction_id=pr.id JOIN numeric_questions n ON n.item_id=pi.id WHERE pr.pool_id=?1 AND datetime(pi.reveal_at)<=datetime('now') ORDER BY pi.sort_order"
    ).bind(&pool_id).fetch_all(db).await.map_err(|e|crate::security::internal_error("numeric_member_predictions_load",e))?;
    rows.extend(
        numeric
            .into_iter()
            .map(|(user, item, title, value, places, unit)| {
                (
                    user,
                    item,
                    title,
                    format!(
                        "{}{}",
                        crate::numeric::display_scaled(value, places as u8),
                        unit.map(|v| format!(" {v}")).unwrap_or_default()
                    ),
                )
            }),
    );
    let multiple: Vec<(String,String,String,String)> = sqlx::query_as(
        "SELECT p.user_id,p.item_id,p.title,GROUP_CONCAT(p.label, ' • ') FROM (
           SELECT pr.user_id,pi.id AS item_id,pi.title,o.label,pi.sort_order,o.sort_order AS option_sort
           FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id
           JOIN multiple_choice_prediction_options v ON v.prediction_id=pr.id
           JOIN custom_question_options o ON o.id=v.option_id
           WHERE pr.pool_id=?1 AND datetime(pi.reveal_at)<=datetime('now') AND pi.kind='multiple_choice'
           ORDER BY pi.sort_order,o.sort_order
         ) p GROUP BY p.user_id,p.item_id,p.title ORDER BY MIN(p.sort_order)"
    ).bind(&pool_id).fetch_all(db).await.map_err(|e|crate::security::internal_error("multiple_choice_member_predictions_load",e))?;
    rows.extend(multiple);
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
    let rows: Vec<QuestionRow> = sqlx::query_as(
        "SELECT pi.id AS item_id,pi.kind,pi.title,pi.lock_at,pi.reveal_at,pi.sort_order,pi.status AS stored_status,cpv.option_id AS current_option_id,
                CASE WHEN datetime(pi.reveal_at)<=datetime('now') THEN q.correct_option_id ELSE NULL END AS correct_option_id,
                n.decimal_places,n.unit_label,n.min_value_scaled AS min_scaled,n.max_value_scaled AS max_scaled,npv.value_scaled AS current_scaled,
                CASE WHEN datetime(pi.reveal_at)<=datetime('now') THEN n.result_value_scaled ELSE NULL END AS result_scaled,
                s.correct_points,s.incorrect_points,ns.exact_points,ns.tolerance_scaled,ns.within_tolerance_points,
                mq.min_selections,mq.max_selections,ms.exact_points AS multiple_exact_points,ms.partial_points,ms.incorrect_points AS multiple_incorrect_points,
                EXISTS(SELECT 1 FROM multiple_choice_results mr WHERE mr.item_id=pi.id) AS multiple_resolved
         FROM prediction_items pi JOIN pools p ON p.event_id=pi.event_id
         LEFT JOIN custom_questions q ON q.item_id=pi.id LEFT JOIN custom_pool_item_scoring s ON s.pool_id=p.id AND s.item_id=pi.id
         LEFT JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN numeric_pool_item_scoring ns ON ns.pool_id=p.id AND ns.item_id=pi.id
         LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id LEFT JOIN multiple_choice_pool_item_scoring ms ON ms.pool_id=p.id AND ms.item_id=pi.id
         LEFT JOIN predictions pr ON pr.pool_id=p.id AND pr.user_id=?2 AND pr.item_id=pi.id
         LEFT JOIN custom_prediction_values cpv ON cpv.prediction_id=pr.id LEFT JOIN numeric_prediction_values npv ON npv.prediction_id=pr.id
         WHERE p.id=?1 AND pi.kind IN ('single_choice','numeric','multiple_choice') ORDER BY pi.sort_order")
        .bind(&pool_id).bind(&session.user_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("list_custom_questions", e))?;
    let mut out = Vec::new();
    for row in rows {
        let QuestionRow {
            item_id,
            kind,
            title,
            lock_at,
            reveal_at,
            sort_order,
            stored_status,
            current_option_id,
            correct_option_id,
            decimal_places,
            unit_label,
            min_scaled,
            max_scaled,
            current_scaled,
            result_scaled,
            correct_points,
            incorrect_points,
            exact_points,
            tolerance_scaled,
            within_tolerance_points,
            min_selections,
            max_selections,
            multiple_exact_points,
            partial_points,
            multiple_incorrect_points,
            multiple_resolved,
        } = row;
        let options = if kind == "single_choice" || kind == "multiple_choice" {
            sqlx::query_as::<_,(String,String,i64)>("SELECT id,label,sort_order FROM custom_question_options WHERE item_id=?1 ORDER BY sort_order")
            .bind(&item_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("list_custom_options", e))?
            .into_iter().map(|(id,label,sort_order)| CustomQuestionOption{id,label,sort_order}).collect()
        } else {
            Vec::new()
        };
        let current_option_ids = if kind == "multiple_choice" {
            sqlx::query_as::<_, (String,)>("SELECT o.id FROM multiple_choice_prediction_options v JOIN custom_question_options o ON o.id=v.option_id JOIN predictions p ON p.id=v.prediction_id WHERE p.pool_id=?1 AND p.user_id=?2 AND p.item_id=?3 ORDER BY o.sort_order")
                .bind(&pool_id).bind(&session.user_id).bind(&item_id).fetch_all(db).await.map_err(|e|crate::security::internal_error("list_multiple_choice_current",e))?.into_iter().map(|(id,)|id).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let correct_option_ids = if kind == "multiple_choice" && multiple_resolved != 0 {
            sqlx::query_as::<_, (String,)>("SELECT o.id FROM multiple_choice_results r JOIN custom_question_options o ON o.id=r.option_id WHERE r.item_id=?1 ORDER BY o.sort_order")
                .bind(&item_id).fetch_all(db).await.map_err(|e|crate::security::internal_error("list_multiple_choice_result",e))?.into_iter().map(|(id,)|id).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let status =
            if correct_option_id.is_some() || result_scaled.is_some() || multiple_resolved != 0 {
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
            kind: match kind.as_str() {
                "numeric" => crate::models::PredictionItemKind::Numeric,
                "multiple_choice" => crate::models::PredictionItemKind::MultipleChoice,
                _ => crate::models::PredictionItemKind::SingleChoice,
            },
            title,
            lock_at,
            reveal_at,
            sort_order,
            status,
            current_option_id,
            correct_option_id,
            correct_points: if kind == "multiple_choice" {
                multiple_exact_points.unwrap_or(0)
            } else {
                correct_points.unwrap_or(0)
            },
            incorrect_points: if kind == "multiple_choice" {
                multiple_incorrect_points.unwrap_or(0)
            } else {
                incorrect_points.unwrap_or(0)
            },
            options,
            decimal_places,
            unit_label,
            min_value: min_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            max_value: max_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            current_value: current_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            result_value: result_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            exact_points: if kind == "multiple_choice" {
                multiple_exact_points
            } else {
                exact_points
            },
            tolerance: tolerance_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            within_tolerance_points,
            min_selections,
            max_selections,
            current_option_ids: (kind == "multiple_choice").then_some(current_option_ids),
            correct_option_ids: (kind == "multiple_choice" && multiple_resolved != 0)
                .then_some(correct_option_ids),
            partial_points,
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

#[cfg(feature = "server")]
pub async fn set_correct_option_authorized(
    token: String,
    item_id: String,
    option_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let allowed:Option<(String,)>=sqlx::query_as("SELECT pi.id FROM prediction_items pi JOIN events e ON e.id=pi.event_id LEFT JOIN users u ON u.id=?2 WHERE pi.id=?1 AND (e.created_by=?2 OR u.is_admin=1)")
        .bind(&item_id).bind(&session.user_id).fetch_optional(crate::db::pool()).await.map_err(|e|crate::security::internal_error("custom_result_authorization",e))?;
    if allowed.is_none() {
        return Err(crate::security::public_error(
            "Somente o dono do evento ou admin pode definir o resultado.",
        ));
    }
    set_correct_option(&item_id, &option_id).await?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_official_result_changed",
        "prediction_item",
        Some(&item_id),
        None,
        serde_json::json!({ "option_id": option_id }),
    )
    .await
}
