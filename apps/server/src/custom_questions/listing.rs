use super::types::QuestionRow;
use crate::error::ServerFnError;
use crate::models::{CustomQuestion, CustomQuestionOption};

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
                CASE WHEN (orx.state IN ('resolved','not_representable') OR datetime(pi.reveal_at)<=datetime('now') OR p.predictions_closed_at IS NOT NULL) THEN COALESCE(orx.option_id,q.correct_option_id) ELSE NULL END AS correct_option_id,
                n.decimal_places,n.unit_label,n.min_value_scaled AS min_scaled,n.max_value_scaled AS max_scaled,npv.value_scaled AS current_scaled,
                CASE WHEN (orx.state IN ('resolved','not_representable') OR datetime(pi.reveal_at)<=datetime('now') OR p.predictions_closed_at IS NOT NULL) THEN COALESCE(orx.value_scaled,n.result_value_scaled) ELSE NULL END AS result_scaled,
                orx.state AS result_status,
                s.correct_points,s.incorrect_points,ns.exact_points,ns.tolerance_scaled,ns.within_tolerance_points,
                mq.min_selections,mq.max_selections,ms.exact_points AS multiple_exact_points,ms.partial_points,ms.incorrect_points AS multiple_incorrect_points,
                (EXISTS(SELECT 1 FROM multiple_choice_results mr WHERE mr.item_id=pi.id) OR (orx.state='resolved' AND orx.kind='multiple_choice')) AS multiple_resolved
         FROM prediction_items pi JOIN pools p ON p.event_version_id=pi.event_version_id
         LEFT JOIN custom_questions q ON q.item_id=pi.id LEFT JOIN custom_pool_item_scoring s ON s.pool_id=p.id AND s.item_id=pi.id
         LEFT JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN numeric_pool_item_scoring ns ON ns.pool_id=p.id AND ns.item_id=pi.id
         LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id LEFT JOIN multiple_choice_pool_item_scoring ms ON ms.pool_id=p.id AND ms.item_id=pi.id
         LEFT JOIN official_results orx ON orx.event_version_id=pi.event_version_id AND orx.item_id=pi.id
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
            result_status,
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
            let option_rows = sqlx::query_as::<_,(String,String,i64,Option<String>,Option<String>)>("SELECT o.id,o.label,o.sort_order,o.image_url,a.id FROM custom_question_options o LEFT JOIN assets a ON a.id=o.image_asset_id WHERE o.item_id=?1 ORDER BY sort_order")
                .bind(&item_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("list_custom_options", e))?;
            let mut options = Vec::with_capacity(option_rows.len());
            for (id, label, sort_order, image_url, image_asset_id) in option_rows {
                let links = sqlx::query_as::<_, (String,String,String,i64)>("SELECT kind,label,url,sort_order FROM option_links WHERE option_id=?1 ORDER BY sort_order,id")
                    .bind(&id).fetch_all(db).await.map_err(|e| crate::security::internal_error("list_option_links", e))?
                    .into_iter().map(|(kind,label,url,sort_order)| crate::models::OptionLink {kind,label,url,sort_order}).collect();
                let media_seen: (i64,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM option_media_progress WHERE user_id=?1 AND option_id=?2)").bind(&session.user_id).bind(&id).fetch_one(db).await.map_err(|e| crate::security::internal_error("list_option_seen", e))?;
                options.push(CustomQuestionOption {
                    id,
                    label,
                    sort_order,
                    image_url,
                    image_asset_url: image_asset_id
                        .map(|asset_id| format!("/media/assets/{asset_id}/card")),
                    links,
                    media_seen: media_seen.0 != 0,
                });
            }
            options
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
            result_status,
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
