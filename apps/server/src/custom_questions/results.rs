use crate::error::ServerFnError;

/// A resolução é armazenada, mas não muda status nem aciona scoring nesta fase.
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
