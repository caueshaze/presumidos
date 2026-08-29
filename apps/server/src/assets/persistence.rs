fn storage_key(sha256: &str, variant: &str) -> String {
    format!("{sha256}/{variant}.webp")
}

async fn authorize_editor(
    session: &crate::auth::AuthSession,
    event_id: &str,
) -> Result<(), ServerFnError> {
    let allowed: Option<(String, String)> =
        sqlx::query_as("SELECT status,created_by FROM events WHERE id=?1 AND kind='custom'")
            .bind(event_id)
            .fetch_optional(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_event_access", e))?;
    let Some((_status, owner)) = allowed else {
        return Err(crate::security::public_error("Evento não encontrado."));
    };
    let is_admin: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id=?1")
        .bind(&session.user_id)
        .fetch_one(crate::db::pool())
        .await
        .map_err(|e| crate::security::internal_error("asset_event_admin", e))?;
    if is_admin.0 || owner == session.user_id {
        let status: (String,) = sqlx::query_as("SELECT status FROM events WHERE id=?1")
            .bind(event_id)
            .fetch_one(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_event_status", e))?;
        if status.0 != "draft" && !is_admin.0 {
            return Err(crate::security::public_error(
                "A mídia de evento publicado só pode ser alterada por um administrador.",
            ));
        }
        Ok(())
    } else {
        Err(crate::security::public_error(
            "Você não pode editar a mídia deste evento.",
        ))
    }
}

async fn persist_asset(normalized: &NormalizedAsset, actor: &str) -> Result<String, ServerFnError> {
    let db = crate::db::pool();
    let asset: Option<(String, String)> =
        sqlx::query_as("SELECT id,storage_key FROM assets WHERE sha256=?1")
            .bind(&normalized.sha256)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("asset_lookup_hash", e))?;
    let file_store = store();
    if let Some((id, master_key)) = asset {
        if !file_store.exists(&master_key) {
            return Err(crate::security::internal_error(
                "asset_missing_master",
                "asset metadata points to missing file",
            ));
        }
        let variants: Vec<(String,)> =
            sqlx::query_as("SELECT storage_key FROM asset_variants WHERE asset_id=?1")
                .bind(&id)
                .fetch_all(db)
                .await
                .map_err(|e| crate::security::internal_error("asset_lookup_variants", e))?;
        if variants.len() != VARIANTS.len()
            || variants
                .iter()
                .any(|(storage_key,)| !file_store.exists(storage_key))
        {
            return Err(crate::security::internal_error(
                "asset_missing_variant",
                "asset metadata points to missing variant file",
            ));
        }
        return Ok(id);
    }
    let staged = file_store.store(normalized)?;
    file_store.promote(&staged)?;
    let asset_id = Uuid::new_v4().to_string();
    let result: Result<String, ServerFnError> = async {
        // Serialize the hash lookup and insert. This gives rollback
        // compensation a stable database lock before it removes a failed
        // filesystem promotion.
        let mut tx = db
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| crate::security::internal_error("asset_insert_begin", e))?;
        if let Some((id, _)) =
            sqlx::query_as::<_, (String, String)>("SELECT id,storage_key FROM assets WHERE sha256=?1")
                .bind(&normalized.sha256)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| crate::security::internal_error("asset_insert_race", e))?
        {
            tx.rollback().await.ok();
            return Ok(id);
        }
        sqlx::query("INSERT INTO assets(id,storage_key,sha256,media_type,width,height,byte_size,uploaded_by) VALUES(?1,?2,?3,'image/webp',?4,?5,?6,?7)")
            .bind(&asset_id).bind(storage_key(&normalized.sha256, PACKAGE_MASTER_VARIANT)).bind(&normalized.sha256)
            .bind(i64::from(normalized.width)).bind(i64::from(normalized.height)).bind(normalized.master.len() as i64).bind(actor)
            .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("asset_insert", e))?;
        for (variant, data) in &normalized.variants {
            sqlx::query("INSERT INTO asset_variants(asset_id,variant,storage_key,width,height,byte_size) VALUES(?1,?2,?3,?4,?5,?6)")
                .bind(&asset_id).bind(variant).bind(storage_key(&normalized.sha256, variant)).bind(i64::from(data.width)).bind(i64::from(data.height)).bind(data.bytes.len() as i64)
                .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("asset_variant_insert", e))?;
        }
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("asset_insert_commit", e))?;
        Ok(asset_id)
    }
    .await;
    if result.is_err() {
        // SQLite rollback and filesystem cleanup are separate operations. The
        // helper checks references while holding BEGIN IMMEDIATE so an asset
        // cannot become referenced between the check and physical deletion.
        let _ = remove_unreferenced_asset(&normalized.sha256).await;
    }
    result
}

/// Compensates a failed asset/database operation without becoming a general
/// garbage collector. Only assets with no Event/Option reference are removed,
/// and the reference check plus row deletion are serialized by SQLite.
pub async fn remove_unreferenced_asset(sha256: &str) -> Result<bool, ServerFnError> {
    if sha256.len() != 64 || !sha256.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err(crate::security::public_error("Hash de asset inválido."));
    }
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_begin", e))?;
    let asset: Option<(String,)> = sqlx::query_as("SELECT id FROM assets WHERE sha256=?1")
        .bind(sha256)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_lookup", e))?;
    let Some(asset_id) = asset else {
        store().delete_hash_directory(sha256)?;
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("asset_cleanup_commit", e))?;
        return Ok(true);
    };
    let referenced: (i64,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM events WHERE cover_asset_id=?1) OR EXISTS(SELECT 1 FROM event_versions WHERE cover_asset_id=?1) OR EXISTS(SELECT 1 FROM custom_question_options WHERE image_asset_id=?1)",
    )
    .bind(&asset_id.0)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("asset_cleanup_reference", e))?;
    if referenced.0 != 0 {
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("asset_cleanup_keep_commit", e))?;
        return Ok(false);
    }
    store().delete_hash_directory(sha256)?;
    sqlx::query("DELETE FROM asset_variants WHERE asset_id=?1")
        .bind(&asset_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_variants", e))?;
    sqlx::query("DELETE FROM assets WHERE id=?1")
        .bind(&asset_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_row", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_delete_commit", e))?;
    Ok(true)
}

