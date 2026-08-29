async fn response_for(asset_id: &str) -> Result<AssetResponse, ServerFnError> {
    let row: (String, String, i64, i64, i64) =
        sqlx::query_as("SELECT sha256,media_type,width,height,byte_size FROM assets WHERE id=?1")
            .bind(asset_id)
            .fetch_one(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_response", e))?;
    let variants = VARIANTS
        .into_iter()
        .map(|variant| {
            (
                variant.to_string(),
                format!("/media/assets/{asset_id}/{variant}"),
            )
        })
        .collect();
    Ok(AssetResponse {
        asset_id: asset_id.to_string(),
        sha256: row.0,
        media_type: row.1,
        width: row.2 as u32,
        height: row.3 as u32,
        byte_size: row.4 as usize,
        url: format!("/media/assets/{asset_id}/cover"),
        variants,
    })
}

pub async fn read_variant(
    asset_id: &str,
    variant: &str,
) -> Result<(Vec<u8>, String), ServerFnError> {
    if !matches!(variant, "master" | "thumb" | "card" | "cover") {
        return Err(crate::security::public_error("Variante de asset inválida."));
    }
    let db = crate::db::pool();
    let row: Option<(String, String)> = sqlx::query_as("SELECT av.storage_key,a.sha256 FROM asset_variants av JOIN assets a ON a.id=av.asset_id WHERE av.asset_id=?1 AND av.variant=?2 UNION ALL SELECT storage_key,sha256 FROM assets WHERE id=?1 AND ?2='master'")
        .bind(asset_id).bind(variant).fetch_optional(db).await.map_err(|e| crate::security::internal_error("asset_variant_lookup", e))?;
    let Some((storage_key, sha256)) = row else {
        return Err(crate::security::public_error("Asset não encontrado."));
    };
    let bytes = store().read(&storage_key)?;
    Ok((bytes, sha256))
}

pub async fn can_read(asset_id: &str) -> Result<bool, ServerFnError> {
    let db = crate::db::pool();
    let published: (i64,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM events e JOIN event_versions v ON v.event_id=e.id AND (v.state='published' OR (e.current_published_version_id IS NULL AND v.state='working')) WHERE e.status IN ('active','finished') AND (v.cover_asset_id=?1 OR EXISTS(SELECT 1 FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.image_asset_id=?1 AND pi.event_version_id=v.id)))")
        .bind(asset_id).fetch_one(db).await.map_err(|e| crate::security::internal_error("asset_public_access", e))?;
    if published.0 != 0 {
        return Ok(true);
    }
    let state = crate::auth::current_user(String::new()).await?;
    let Some(user) = state.user else {
        return Ok(false);
    };
    let allowed: (i64,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM events e JOIN event_versions v ON v.event_id=e.id AND v.state='working' WHERE (v.cover_asset_id=?1 OR EXISTS(SELECT 1 FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.image_asset_id=?1 AND pi.event_version_id=v.id)) AND (e.created_by=?2 OR EXISTS(SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1)))")
        .bind(asset_id).bind(user.id).fetch_one(db).await.map_err(|e| crate::security::internal_error("asset_private_access", e))?;
    Ok(allowed.0 != 0)
}

pub fn master_bytes_for_hash(sha256: &str) -> Result<Vec<u8>, ServerFnError> {
    store().read(&storage_key(sha256, PACKAGE_MASTER_VARIANT))
}

pub async fn has_complete_asset(sha256: &str) -> Result<bool, ServerFnError> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id,storage_key FROM assets WHERE sha256=?1 AND media_type='image/webp'",
    )
    .bind(sha256)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("asset_complete_lookup", e))?;
    let Some((asset_id, master_key)) = row else {
        return Ok(false);
    };
    let file_store = store();
    if !file_store.exists(&master_key) {
        return Ok(false);
    }
    let variants: Vec<(String, String)> =
        sqlx::query_as("SELECT variant,storage_key FROM asset_variants WHERE asset_id=?1")
            .bind(asset_id)
            .fetch_all(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_complete_variants", e))?;
    let variants: HashSet<_> = variants
        .into_iter()
        .filter(|(variant, storage_key)| {
            file_store.exists(storage_key) && VARIANTS.contains(&variant.as_str())
        })
        .map(|(variant, _)| variant)
        .collect();
    Ok(variants.len() == VARIANTS.len())
}

pub fn validate_package_master(
    bytes: &[u8],
    expected_sha: &str,
) -> Result<NormalizedAsset, ServerFnError> {
    if bytes.len() > settings().asset_max_upload_bytes {
        return Err(crate::security::public_error(
            "Asset do pacote excede o limite permitido.",
        ));
    }
    let raw_sha = hex::encode(Sha256::digest(bytes));
    if raw_sha != expected_sha {
        return Err(crate::security::public_error(format!(
            "Hash do asset {expected_sha} não confere."
        )));
    }
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| crate::security::public_error("Master de asset inválido."))?;
    if reader.format() != Some(ImageFormat::WebP) {
        return Err(crate::security::public_error(
            "Masters de pacote devem ser WebP.",
        ));
    }
    let (header_width, header_height) = reader
        .into_dimensions()
        .map_err(|_| crate::security::public_error("Não foi possível ler as dimensões."))?;
    if u64::from(header_width) * u64::from(header_height) > settings().asset_max_pixels {
        return Err(crate::security::public_error(
            "Master de asset excede o limite de pixels.",
        ));
    }
    let image = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| crate::security::public_error("Master de asset inválido."))?
        .decode()
        .map_err(|_| crate::security::public_error("Não foi possível decodificar o master."))?;
    let mut variants = BTreeMap::new();
    for (name, width) in [("thumb", 320u32), ("card", 640u32), ("cover", 1280u32)] {
        let resized = image.thumbnail(width, width);
        variants.insert(
            name.to_string(),
            VariantBytes {
                bytes: encode_webp(&resized)?,
                width: resized.width(),
                height: resized.height(),
            },
        );
    }
    Ok(NormalizedAsset {
        master: bytes.to_vec(),
        sha256: expected_sha.to_string(),
        width: image.width(),
        height: image.height(),
        variants,
    })
}

pub fn read_limited<R: Read>(reader: &mut R, limit: usize) -> Result<Vec<u8>, ServerFnError> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| crate::security::public_error(format!("falha ao ler pacote: {e}")))?;
    if bytes.len() > limit {
        return Err(crate::security::public_error(
            "Pacote ou asset excede o limite permitido.",
        ));
    }
    Ok(bytes)
}

pub async fn ensure_manifest_assets(
    manifest: &crate::custom_event_manifest::CustomEventManifest,
) -> Result<(), ServerFnError> {
    ensure_manifest_assets_available(manifest, &std::collections::HashSet::new()).await
}

pub async fn ensure_manifest_assets_available(
    manifest: &crate::custom_event_manifest::CustomEventManifest,
    available: &std::collections::HashSet<String>,
) -> Result<(), ServerFnError> {
    let mut hashes = Vec::new();
    if let Some(asset) = &manifest.cover_asset {
        hashes.push(asset.sha256.as_str());
    }
    for item in &manifest.items {
        for option in &item.options {
            if let Some(asset) = &option.image_asset {
                hashes.push(asset.sha256.as_str());
            }
        }
    }
    for sha256 in hashes {
        if available.contains(sha256) {
            continue;
        }
        if !has_complete_asset(sha256).await? {
            return Err(crate::security::public_error(format!(
                "Asset {sha256} referenciado pelo manifesto não foi encontrado."
            )));
        }
    }
    Ok(())
}

pub async fn ingest_package_master(
    bytes: &[u8],
    expected_sha256: &str,
    actor: &str,
) -> Result<String, ServerFnError> {
    let normalized = validate_package_master(bytes, expected_sha256)?;
    persist_asset(&normalized, actor).await
}
