async fn attach(
    event_id: &str,
    version_id: &str,
    option_id: Option<&str>,
    asset_id: Option<&str>,
    actor: &str,
    action: &str,
) -> Result<(), ServerFnError> {
    let db = crate::db::pool();
    if let Some(option_id) = option_id {
        sqlx::query("UPDATE custom_question_options SET image_asset_id=?2 WHERE id=?1 AND EXISTS(SELECT 1 FROM prediction_items pi WHERE pi.id=custom_question_options.item_id AND pi.event_version_id=?3)")
            .bind(option_id).bind(asset_id).bind(version_id).execute(db).await
            .map_err(|e| crate::security::internal_error("asset_option_attach", e))?;
    } else {
        sqlx::query("UPDATE event_versions SET cover_asset_id=?2,updated_at=datetime('now') WHERE id=?1 AND state='working'")
            .bind(version_id)
            .bind(asset_id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("asset_cover_attach", e))?;
        // Mantém a coluna legada como projeção de compatibilidade para
        // fixtures e integrações antigas. A EventVersion continua sendo a
        // fonte canônica usada por Pools e convites.
        sqlx::query("UPDATE events SET cover_asset_id=?2 WHERE id=?1")
            .bind(event_id)
            .bind(asset_id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("asset_event_cover_projection", e))?;
    }
    crate::security::append_audit_log(
        db,
        Some(actor),
        action,
        "event",
        Some(event_id),
        None,
        serde_json::json!({"assetId": asset_id}),
    )
    .await
}

pub async fn upload_cover(
    token: String,
    event_id: String,
    bytes: Vec<u8>,
    csrf: String,
) -> Result<AssetResponse, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    let normalized = normalize_image(&bytes)?;
    let asset_id = persist_asset(&normalized, &session.user_id).await?;
    attach(
        &event_id,
        &version_id,
        None,
        Some(&asset_id),
        &session.user_id,
        "event_cover_asset_changed",
    )
    .await?;
    response_for(&asset_id).await
}

pub async fn upload_option(
    token: String,
    event_id: String,
    option_id: String,
    bytes: Vec<u8>,
    csrf: String,
) -> Result<AssetResponse, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    let owns: Option<(String,)> = sqlx::query_as("SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.id=?1 AND pi.event_version_id=?2")
        .bind(&option_id).bind(&version_id).fetch_optional(crate::db::pool()).await
        .map_err(|e| crate::security::internal_error("asset_option_access", e))?;
    if owns.is_none() {
        return Err(crate::security::public_error("Opção não encontrada."));
    }
    let normalized = normalize_image(&bytes)?;
    let asset_id = persist_asset(&normalized, &session.user_id).await?;
    attach(
        &event_id,
        &version_id,
        Some(&option_id),
        Some(&asset_id),
        &session.user_id,
        "event_option_asset_changed",
    )
    .await?;
    response_for(&asset_id).await
}

pub async fn remove_cover(
    token: String,
    event_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    attach(
        &event_id,
        &version_id,
        None,
        None,
        &session.user_id,
        "event_cover_asset_removed",
    )
    .await
}

pub async fn remove_option(
    token: String,
    event_id: String,
    option_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    let owns: Option<(String,)> = sqlx::query_as(
        "SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.id=?1 AND pi.event_version_id=?2",
    )
    .bind(&option_id)
    .bind(&version_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("asset_option_remove_access", e))?;
    if owns.is_none() {
        return Err(crate::security::public_error("Opção não encontrada."));
    }
    attach(
        &event_id,
        &version_id,
        Some(&option_id),
        None,
        &session.user_id,
        "event_option_asset_removed",
    )
    .await
}

