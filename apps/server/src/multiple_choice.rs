use std::collections::BTreeSet;

#[cfg(test)]
use crate::models::MultipleChoiceScoreOutcome;

#[cfg(test)]
pub fn classify(
    predicted: &BTreeSet<String>,
    official: &BTreeSet<String>,
) -> MultipleChoiceScoreOutcome {
    if predicted == official {
        MultipleChoiceScoreOutcome::Exact
    } else if !predicted.is_empty() && predicted.is_subset(official) {
        MultipleChoiceScoreOutcome::Partial
    } else {
        MultipleChoiceScoreOutcome::Incorrect
    }
}

pub fn validate_selection_count(
    selected_count: usize,
    min_selections: i64,
    max_selections: Option<i64>,
    option_count: i64,
) -> Result<(), String> {
    let effective_max = max_selections.unwrap_or(option_count);
    if min_selections < 1 || effective_max < min_selections || effective_max > option_count {
        return Err("Configuração de mínimo/máximo inválida.".into());
    }
    if selected_count < min_selections as usize || selected_count > effective_max as usize {
        return Err(format!(
            "Selecione de {min_selections} a {effective_max} opções."
        ));
    }
    Ok(())
}

#[cfg(feature = "server")]
pub async fn submit_prediction(
    token: String,
    pool_id: String,
    item_id: String,
    option_ids: Vec<String>,
    csrf_token: String,
) -> Result<(), crate::error::ServerFnError> {
    use crate::auth::require_user;
    use uuid::Uuid;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    if option_ids.is_empty() || option_ids.iter().collect::<BTreeSet<_>>().len() != option_ids.len()
    {
        return Err(crate::security::public_error("Selecione opções únicas."));
    }
    let db = crate::db::pool();
    let row: Option<(String, String, i64, Option<i64>, i64)> = sqlx::query_as(
        "SELECT pi.kind,pi.lock_at,q.min_selections,q.max_selections,COUNT(o.id)
         FROM pools p JOIN events e ON e.id=p.event_id JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
         JOIN multiple_choice_questions q ON q.item_id=pi.id
         LEFT JOIN custom_question_options o ON o.item_id=pi.id
         WHERE p.id=?1 AND pi.id=?2 AND e.status='active' GROUP BY pi.id",
    )
    .bind(&pool_id)
    .bind(&item_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("submit_multiple_choice_load", e))?;
    let Some((kind, lock_at, min, max, option_count)) = row else {
        return Err(crate::security::public_error("Bolão ou pergunta inválida."));
    };
    if kind != "multiple_choice" {
        return Err(crate::security::public_error("Pergunta incompatível."));
    }
    validate_selection_count(option_ids.len(), min, max, option_count)
        .map_err(crate::security::public_error)?;
    let member: Option<(String,)> =
        sqlx::query_as("SELECT user_id FROM pool_members WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("submit_multiple_choice_member", e))?;
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
    if crate::prediction_access::can_edit_item("multiple_choice", &lock_at, None, &session.user_id)
        .await?
        .is_none()
    {
        return Err(crate::security::public_error(
            "Esta pergunta esta travada para palpite.",
        ));
    }
    let valid_count:(i64,) = sqlx::query_as("SELECT COUNT(*) FROM custom_question_options WHERE item_id=?1 AND id IN (SELECT value FROM json_each(?2))")
        .bind(&item_id).bind(serde_json::to_string(&option_ids).unwrap()).fetch_one(db).await.map_err(|e|crate::security::internal_error("submit_multiple_choice_options",e))?;
    if valid_count.0 != option_ids.len() as i64 {
        return Err(crate::security::public_error(
            "Opção incompatível com a pergunta.",
        ));
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("submit_multiple_choice_begin", e))?;
    sqlx::query("INSERT INTO predictions(id,pool_id,user_id,item_id) VALUES(?1,?2,?3,?4) ON CONFLICT(pool_id,user_id,item_id) DO UPDATE SET submitted_at=datetime('now')")
        .bind(Uuid::new_v4().to_string()).bind(&pool_id).bind(&session.user_id).bind(&item_id).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("submit_multiple_choice_prediction",e))?;
    let prediction: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3")
            .bind(&pool_id)
            .bind(&session.user_id)
            .bind(&item_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("submit_multiple_choice_id", e))?;
    sqlx::query("DELETE FROM multiple_choice_prediction_options WHERE prediction_id=?1")
        .bind(&prediction.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("submit_multiple_choice_replace", e))?;
    for option_id in option_ids {
        sqlx::query(
            "INSERT INTO multiple_choice_prediction_options(prediction_id,option_id) VALUES(?1,?2)",
        )
        .bind(&prediction.0)
        .bind(option_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("submit_multiple_choice_insert", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("submit_multiple_choice_commit", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn set_result_authorized(
    token: String,
    item_id: String,
    option_ids: Vec<String>,
    pool_id: Option<String>,
    csrf: String,
) -> Result<(), crate::error::ServerFnError> {
    use crate::auth::require_user;
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if option_ids.is_empty() || option_ids.iter().collect::<BTreeSet<_>>().len() != option_ids.len()
    {
        return Err(crate::security::public_error("Selecione opções únicas."));
    }
    let db = crate::db::pool();
    let row: Option<(i64, Option<i64>, i64, String)> = if let Some(pool_id) = pool_id.as_deref() {
        sqlx::query_as("SELECT q.min_selections,q.max_selections,COUNT(o.id),pi.event_version_id FROM prediction_items pi JOIN multiple_choice_questions q ON q.item_id=pi.id JOIN pools p ON p.event_version_id=pi.event_version_id LEFT JOIN users u ON u.id=?2 LEFT JOIN custom_question_options o ON o.item_id=pi.id WHERE pi.id=?1 AND p.id=?3 AND (p.created_by=?2 OR u.is_admin=1) GROUP BY pi.id")
            .bind(&item_id)
            .bind(&session.user_id)
            .bind(pool_id)
            .fetch_optional(db)
            .await
    } else {
        sqlx::query_as("SELECT q.min_selections,q.max_selections,COUNT(o.id),pi.event_version_id FROM prediction_items pi JOIN multiple_choice_questions q ON q.item_id=pi.id JOIN events e ON e.id=pi.event_id LEFT JOIN users u ON u.id=?2 LEFT JOIN custom_question_options o ON o.item_id=pi.id WHERE pi.id=?1 AND (e.created_by=?2 OR u.is_admin=1) GROUP BY pi.id")
            .bind(&item_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
    }
    .map_err(|e| crate::security::internal_error("multiple_choice_result_authorization", e))?;
    let Some((min, max, option_count, version_id)) = row else {
        return Err(crate::security::public_error(
            "Somente o dono do bolão ou admin pode definir o resultado.",
        ));
    };
    validate_selection_count(option_ids.len(), min, max, option_count)
        .map_err(crate::security::public_error)?;
    let valid:(i64,)=sqlx::query_as("SELECT COUNT(*) FROM custom_question_options WHERE item_id=?1 AND id IN (SELECT value FROM json_each(?2))").bind(&item_id).bind(serde_json::to_string(&option_ids).unwrap()).fetch_one(db).await.map_err(|e|crate::security::internal_error("multiple_choice_result_options",e))?;
    if valid.0 != option_ids.len() as i64 {
        return Err(crate::security::public_error(
            "Opção incompatível com a pergunta.",
        ));
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_result_begin", e))?;
    sqlx::query("DELETE FROM multiple_choice_results WHERE item_id=?1")
        .bind(&item_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_result_replace", e))?;
    for option_id in &option_ids {
        sqlx::query("INSERT INTO multiple_choice_results(item_id,option_id) VALUES(?1,?2)")
            .bind(&item_id)
            .bind(option_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("multiple_choice_result_insert", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_result_commit", e))?;
    sqlx::query("INSERT INTO official_results(id,event_version_id,item_id,kind,state,option_ids_json,updated_at) VALUES(?1,?2,?3,'multiple_choice','resolved',?4,datetime('now')) ON CONFLICT(event_version_id,item_id) DO UPDATE SET state='resolved',option_id=NULL,option_ids_json=excluded.option_ids_json,value_scaled=NULL,reason=NULL,updated_at=datetime('now')")
        .bind(uuid::Uuid::new_v4().to_string()).bind(&version_id).bind(&item_id).bind(serde_json::to_string(&option_ids).unwrap())
        .execute(db).await.map_err(|e| crate::security::internal_error("multiple_choice_result_official", e))?;
    crate::scoring::recalculate_custom_breakdowns().await?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_official_result_changed",
        "prediction_item",
        Some(&item_id),
        None,
        serde_json::json!({"option_ids":option_ids}),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|v| v.to_string()).collect()
    }
    #[test]
    fn classifies_sets_without_order() {
        assert_eq!(
            classify(&set(&["d", "a", "c"]), &set(&["a", "c", "d"])),
            MultipleChoiceScoreOutcome::Exact
        );
        assert_eq!(
            classify(&set(&["a", "c"]), &set(&["a", "c", "d"])),
            MultipleChoiceScoreOutcome::Partial
        );
        assert_eq!(
            classify(&set(&["a", "b"]), &set(&["a", "c", "d"])),
            MultipleChoiceScoreOutcome::Incorrect
        );
    }
}
