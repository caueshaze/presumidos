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
    result_status: Option<String>,
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
         FROM pools p JOIN events e ON e.id = p.event_id JOIN prediction_items pi ON pi.event_version_id = p.event_version_id
         LEFT JOIN custom_question_options o ON o.id = ?3
         WHERE p.id = ?1 AND pi.id = ?2 AND e.status = 'active'",
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
    if crate::prediction_access::can_edit_item("single_choice", &lock_at, None, &session.user_id)
        .await?
        .is_none()
    {
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
                CASE WHEN orx.state IN ('resolved','not_representable') OR datetime(pi.reveal_at)<=datetime('now') THEN COALESCE(orx.option_id,q.correct_option_id) ELSE NULL END AS correct_option_id,
                n.decimal_places,n.unit_label,n.min_value_scaled AS min_scaled,n.max_value_scaled AS max_scaled,npv.value_scaled AS current_scaled,
                CASE WHEN orx.state IN ('resolved','not_representable') OR datetime(pi.reveal_at)<=datetime('now') THEN COALESCE(orx.value_scaled,n.result_value_scaled) ELSE NULL END AS result_scaled,
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

#[cfg(feature = "server")]
pub async fn set_option_media_seen(
    token: String,
    pool_id: String,
    option_id: String,
    seen: bool,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let allowed: (i64,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM pool_members pm JOIN pools p ON p.id=pm.pool_id JOIN prediction_items pi ON pi.event_version_id=p.event_version_id JOIN custom_question_options o ON o.item_id=pi.id JOIN option_links l ON l.option_id=o.id WHERE pm.pool_id=?1 AND pm.user_id=?2 AND o.id=?3)")
        .bind(&pool_id).bind(&session.user_id).bind(&option_id).fetch_one(db).await.map_err(|e| crate::security::internal_error("set_option_media_seen_allowed", e))?;
    if allowed.0 == 0 {
        return Err(crate::security::public_error(
            "Mídia não disponível neste bolão.",
        ));
    }
    if seen {
        sqlx::query("INSERT INTO option_media_progress(user_id,option_id) VALUES(?1,?2) ON CONFLICT(user_id,option_id) DO UPDATE SET seen_at=datetime('now')").bind(&session.user_id).bind(&option_id).execute(db).await.map_err(|e| crate::security::internal_error("set_option_media_seen", e))?;
    } else {
        sqlx::query("DELETE FROM option_media_progress WHERE user_id=?1 AND option_id=?2")
            .bind(&session.user_id)
            .bind(&option_id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("clear_option_media_seen", e))?;
    }
    Ok(())
}

#[cfg(feature = "server")]
pub async fn event_showcase(
    token: String,
    pool_id: String,
) -> Result<crate::models::EventShowcase, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let row: Option<(String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,String,i64,i64)> = sqlx::query_as("SELECT v.name,v.description,v.cover_url,v.external_url,v.cover_asset_id,e.starts_at,e.ends_at,e.status,(SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id=p.event_version_id),(SELECT COUNT(*) FROM predictions pr WHERE pr.pool_id=p.id AND pr.user_id=?2) FROM pools p JOIN events e ON e.id=p.event_id JOIN event_versions v ON v.id=p.event_version_id JOIN pool_members pm ON pm.pool_id=p.id AND pm.user_id=?2 WHERE p.id=?1")
        .bind(&pool_id).bind(&session.user_id).fetch_optional(db).await.map_err(|e| crate::security::internal_error("event_showcase", e))?;
    let Some((
        name,
        description,
        cover_url,
        external_url,
        cover_asset_id,
        starts_at,
        ends_at,
        status,
        item_count,
        answered_count,
    )) = row
    else {
        return Err(crate::security::public_error(
            "Bolão não encontrado ou sem acesso.",
        ));
    };
    let is_historical = status == "finished"
        || ends_at
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|value| value <= chrono::Utc::now());
    Ok(crate::models::EventShowcase {
        name,
        description,
        cover_url,
        cover_asset_url: cover_asset_id.map(|asset_id| format!("/media/assets/{asset_id}/cover")),
        external_url,
        starts_at,
        ends_at,
        item_count,
        answered_count,
        is_historical,
    })
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
    let version: Option<(String,)> = sqlx::query_as(
        "SELECT pi.event_version_id FROM prediction_items pi WHERE pi.id=?1 AND EXISTS(SELECT 1 FROM custom_question_options o WHERE o.id=?2 AND o.item_id=pi.id)",
    )
    .bind(item_id)
    .bind(option_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("set_correct_option_version", e))?;
    let Some((version_id,)) = version else {
        return Err(crate::security::public_error(
            "Opção correta incompatível com a pergunta.",
        ));
    };
    let changed = sqlx::query(
        "UPDATE custom_questions SET correct_option_id=?2, updated_at=datetime('now')
         WHERE item_id=?1",
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
    sqlx::query("INSERT INTO official_results(id,event_version_id,item_id,kind,state,option_id,updated_at) VALUES(?1,?2,?3,'single_choice','resolved',?4,datetime('now')) ON CONFLICT(event_version_id,item_id) DO UPDATE SET state='resolved',option_id=excluded.option_id,option_ids_json=NULL,value_scaled=NULL,reason=NULL,updated_at=datetime('now')")
        .bind(uuid::Uuid::new_v4().to_string()).bind(&version_id).bind(item_id).bind(option_id)
        .execute(crate::db::pool()).await
        .map_err(|e| crate::security::internal_error("set_correct_option_official", e))?;
    crate::scoring::recalculate_custom_breakdowns().await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn set_correct_option_authorized(
    token: String,
    item_id: String,
    option_id: String,
    pool_id: Option<String>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let allowed: Option<(String,)> = if let Some(pool_id) = pool_id.as_deref() {
        sqlx::query_as("SELECT p.id FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id LEFT JOIN users u ON u.id=?2 WHERE p.id=?3 AND pi.id=?1 AND (p.created_by=?2 OR u.is_admin=1)")
            .bind(&item_id)
            .bind(&session.user_id)
            .bind(pool_id)
            .fetch_optional(crate::db::pool())
            .await
    } else {
        sqlx::query_as("SELECT pi.id FROM prediction_items pi JOIN events e ON e.id=pi.event_id LEFT JOIN users u ON u.id=?2 WHERE pi.id=?1 AND (e.created_by=?2 OR u.is_admin=1)")
            .bind(&item_id)
            .bind(&session.user_id)
            .fetch_optional(crate::db::pool())
            .await
    }
    .map_err(|e| crate::security::internal_error("custom_result_authorization", e))?;
    if allowed.is_none() {
        return Err(crate::security::public_error(
            "Somente o dono do bolão ou admin pode definir o resultado.",
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

#[cfg(feature = "server")]
pub async fn mark_result_not_representable_authorized(
    token: String,
    item_id: String,
    reason: String,
    pool_id: Option<String>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let reason = crate::security::normalize_required_text("Motivo", reason, 1, 1000)?;
    let row: Option<(String, String)> = if let Some(pool_id) = pool_id.as_deref() {
        sqlx::query_as("SELECT pi.event_version_id,pi.kind FROM prediction_items pi JOIN pools p ON p.event_version_id=pi.event_version_id LEFT JOIN users u ON u.id=?2 WHERE pi.id=?1 AND p.id=?3 AND (p.created_by=?2 OR u.is_admin=1)")
            .bind(&item_id)
            .bind(&session.user_id)
            .bind(pool_id)
            .fetch_optional(crate::db::pool())
            .await
    } else {
        sqlx::query_as("SELECT pi.event_version_id,pi.kind FROM prediction_items pi JOIN events e ON e.id=pi.event_id LEFT JOIN users u ON u.id=?2 WHERE pi.id=?1 AND (e.created_by=?2 OR u.is_admin=1)")
            .bind(&item_id)
            .bind(&session.user_id)
            .fetch_optional(crate::db::pool())
            .await
    }
    .map_err(|e| crate::security::internal_error("result_not_representable_access", e))?;
    let Some((version_id, kind)) = row else {
        return Err(crate::security::public_error(
            "Somente o dono do evento ou admin pode decidir este resultado.",
        ));
    };
    match kind.as_str() {
        "single_choice" => sqlx::query("UPDATE custom_questions SET correct_option_id=NULL,updated_at=datetime('now') WHERE item_id=?1").bind(&item_id).execute(crate::db::pool()).await,
        "numeric" => sqlx::query("UPDATE numeric_questions SET result_value_scaled=NULL,updated_at=datetime('now') WHERE item_id=?1").bind(&item_id).execute(crate::db::pool()).await,
        "multiple_choice" => sqlx::query("DELETE FROM multiple_choice_results WHERE item_id=?1").bind(&item_id).execute(crate::db::pool()).await,
        _ => return Err(crate::security::public_error("Tipo de resultado não suportado.")),
    }
    .map_err(|e| crate::security::internal_error("result_not_representable_clear", e))?;
    sqlx::query("INSERT INTO official_results(id,event_version_id,item_id,kind,state,reason,updated_by,updated_at) VALUES(?1,?2,?3,?4,'not_representable',?5,?6,datetime('now')) ON CONFLICT(event_version_id,item_id) DO UPDATE SET state='not_representable',option_id=NULL,option_ids_json=NULL,value_scaled=NULL,reason=excluded.reason,updated_by=excluded.updated_by,updated_at=datetime('now')")
        .bind(uuid::Uuid::new_v4().to_string()).bind(&version_id).bind(&item_id).bind(&kind).bind(&reason).bind(&session.user_id)
        .execute(crate::db::pool()).await
        .map_err(|e| crate::security::internal_error("result_not_representable_save", e))?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_official_result_not_representable",
        "prediction_item",
        Some(&item_id),
        None,
        serde_json::json!({"eventVersionId":version_id,"reason":reason}),
    )
    .await
}
