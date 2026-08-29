use crate::error::ServerFnError;

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
    if !crate::pool_access::can_write_predictions(&pool_id).await? {
        return Err(crate::security::public_error(
            "Os palpites deste bolão estão encerrados.",
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
    let early_reveal = crate::pool_access::can_reveal_early(&pool_id).await?;
    let members: Vec<(String, String)> = sqlx::query_as("SELECT u.id,u.username FROM pool_members pm JOIN users u ON u.id=pm.user_id WHERE pm.pool_id=?1 ORDER BY u.username COLLATE NOCASE")
        .bind(&pool_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("custom_member_predictions_members", e))?;
    let mut rows: Vec<(String, String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT pr.user_id,pi.id,pi.title,o.label,b.total_points FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id JOIN custom_prediction_values cpv ON cpv.prediction_id=pr.id JOIN custom_question_options o ON o.id=cpv.option_id LEFT JOIN custom_prediction_score_breakdowns b ON b.pool_id=pr.pool_id AND b.user_id=pr.user_id AND b.item_id=pr.item_id WHERE pr.pool_id=?1 AND (datetime(pi.reveal_at)<=datetime('now') OR ?2=1)
         ORDER BY pi.sort_order",
    )
    .bind(&pool_id)
    .bind(early_reveal)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("custom_member_predictions_load", e))?;
    let numeric: Vec<(String,String,String,i64,i64,Option<String>,Option<i64>)> = sqlx::query_as(
        "SELECT pr.user_id,pi.id,pi.title,v.value_scaled,n.decimal_places,n.unit_label,b.total_points FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id JOIN numeric_prediction_values v ON v.prediction_id=pr.id JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN numeric_prediction_score_breakdowns b ON b.pool_id=pr.pool_id AND b.user_id=pr.user_id AND b.item_id=pr.item_id WHERE pr.pool_id=?1 AND (datetime(pi.reveal_at)<=datetime('now') OR ?2=1) ORDER BY pi.sort_order"
    ).bind(&pool_id).bind(early_reveal).fetch_all(db).await.map_err(|e|crate::security::internal_error("numeric_member_predictions_load",e))?;
    rows.extend(
        numeric
            .into_iter()
            .map(|(user, item, title, value, places, unit, points)| {
                (
                    user,
                    item,
                    title,
                    format!(
                        "{}{}",
                        crate::numeric::display_scaled(value, places as u8),
                        unit.map(|v| format!(" {v}")).unwrap_or_default()
                    ),
                    points,
                )
            }),
    );
    let multiple: Vec<(String,String,String,String,Option<i64>)> = sqlx::query_as(
        "SELECT p.user_id,p.item_id,p.title,GROUP_CONCAT(p.label, ' • '),MAX(p.total_points) FROM (
           SELECT pr.user_id,pi.id AS item_id,pi.title,o.label,b.total_points,pi.sort_order,o.sort_order AS option_sort
           FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id
           JOIN multiple_choice_prediction_options v ON v.prediction_id=pr.id
           JOIN custom_question_options o ON o.id=v.option_id
           LEFT JOIN multiple_choice_prediction_score_breakdowns b ON b.pool_id=pr.pool_id AND b.user_id=pr.user_id AND b.item_id=pr.item_id
           WHERE pr.pool_id=?1 AND (datetime(pi.reveal_at)<=datetime('now') OR ?2=1) AND pi.kind='multiple_choice'
           ORDER BY pi.sort_order,o.sort_order
         ) p GROUP BY p.user_id,p.item_id,p.title ORDER BY MIN(p.sort_order)"
    ).bind(&pool_id).bind(early_reveal).fetch_all(db).await.map_err(|e|crate::security::internal_error("multiple_choice_member_predictions_load",e))?;
    rows.extend(multiple);
    let mut by_user: BTreeMap<String, Vec<crate::models::CustomMemberPrediction>> = BTreeMap::new();
    for (user_id, item_id, title, option_label, points) in rows {
        by_user
            .entry(user_id)
            .or_default()
            .push(crate::models::CustomMemberPrediction {
                item_id,
                title,
                option_label,
                points,
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
