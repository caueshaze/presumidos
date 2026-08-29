use super::super::*;

pub(crate) async fn admin_event_manifest_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_admin("").await?;
    let (manifest, content) = crate::custom_event_manifest::export_for_event(&event_id).await?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_manifest_exported",
        "event",
        Some(&event_id),
        None,
        json!({
            "schemaVersion": manifest.schema_version,
            "slug": manifest.slug,
            "manifestFingerprint": crate::custom_event_manifest::fingerprint(&manifest).unwrap_or_default(),
            "itemCount": manifest.items.len(),
            "optionCount": manifest.items.iter().map(|item| item.options.len()).sum::<usize>(),
        }),
    )
    .await?;
    let filename = format!("{}.json", manifest.slug);
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json; charset=utf-8")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(content))
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Não foi possível preparar o download do manifesto.".into(),
        })
}

pub(crate) async fn custom_event_manifest_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_user("").await?;
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT e.id FROM events e WHERE e.id=?1 AND e.kind='custom' AND (e.created_by=?2 OR EXISTS(SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1))",
    )
    .bind(&event_id)
    .bind(&session.user_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("custom_manifest_access", e))?;
    if allowed.is_none() {
        return Err(ApiError::from(crate::security::public_error(
            "Você não pode exportar este evento.",
        )));
    }
    let (manifest, content) =
        crate::custom_event_manifest::export_for_working_event(&event_id).await?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_manifest_exported",
        "event",
        Some(&event_id),
        None,
        json!({
            "schemaVersion": manifest.schema_version,
            "slug": manifest.slug,
            "manifestFingerprint": crate::custom_event_manifest::fingerprint(&manifest).unwrap_or_default(),
            "itemCount": manifest.items.len(),
            "optionCount": manifest.items.iter().map(|item| item.options.len()).sum::<usize>(),
        }),
    )
    .await?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json; charset=utf-8")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{}.json\"", manifest.slug),
        )
        .body(Body::from(content))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível preparar o manifesto.",
            ))
        })
}

pub(crate) async fn admin_manifest_preview(
    headers: HeaderMap,
    Json(body): Json<ManifestPreviewBody>,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    if body
        .filename
        .as_deref()
        .is_some_and(|name| name.len() > 255)
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Nome de arquivo inválido.".into(),
        });
    }
    if body
        .filename
        .as_deref()
        .is_some_and(|name| !name.to_ascii_lowercase().ends_with(".json"))
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Apenas arquivos JSON são aceitos.".into(),
        });
    }
    Ok(Json(
        crate::custom_event_manifest::preview(&body.content).await?,
    ))
}

pub(crate) async fn admin_manifest_apply(
    headers: HeaderMap,
    Json(body): Json<ManifestApplyBody>,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_recent_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    if body
        .filename
        .as_deref()
        .is_some_and(|name| name.len() > 255)
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Nome de arquivo inválido.".into(),
        });
    }
    if body
        .filename
        .as_deref()
        .is_some_and(|name| !name.to_ascii_lowercase().ends_with(".json"))
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Apenas arquivos JSON são aceitos.".into(),
        });
    }
    Ok(Json(
        crate::custom_event_manifest::apply_admin(
            &body.content,
            &body.base_fingerprint,
            &session.user_id,
        )
        .await?,
    ))
}

pub(crate) async fn admin_event_package_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_admin("").await?;
    let bytes = crate::event_package::export(&event_id).await?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_package_exported",
        "event",
        Some(&event_id),
        None,
        json!({"byteSize": bytes.len()}),
    )
    .await?;
    let manifest = crate::custom_event_manifest::export_for_event(&event_id)
        .await?
        .0;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/zip")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{}.zip\"", manifest.slug),
        )
        .body(Body::from(bytes))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível preparar o pacote.",
            ))
        })
}

pub(crate) async fn custom_event_package_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_user("").await?;
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT e.id FROM events e WHERE e.id=?1 AND e.kind='custom' AND (e.created_by=?2 OR EXISTS(SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1))",
    )
    .bind(&event_id)
    .bind(&session.user_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("custom_package_access", e))?;
    if allowed.is_none() {
        return Err(ApiError::from(crate::security::public_error(
            "Você não pode exportar este evento.",
        )));
    }
    let bytes = crate::event_package::export_working(&event_id).await?;
    let manifest = crate::custom_event_manifest::export_for_working_event(&event_id)
        .await?
        .0;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_package_exported",
        "event",
        Some(&event_id),
        None,
        json!({"byteSize": bytes.len(), "slug": manifest.slug}),
    )
    .await?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/zip")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{}.zip\"", manifest.slug),
        )
        .body(Body::from(bytes))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível preparar o pacote.",
            ))
        })
}

pub(crate) async fn custom_event_package_preview(
    Path(event_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_user("").await?;
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT e.id FROM events e WHERE e.id=?1 AND e.kind='custom' AND (e.created_by=?2 OR EXISTS(SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1))",
    )
    .bind(&event_id)
    .bind(&session.user_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("custom_package_preview_access", e))?;
    if allowed.is_none() {
        return Err(ApiError::from(crate::security::public_error(
            "Você não pode exportar este evento.",
        )));
    }
    Ok(Json(
        crate::event_package_preview::for_working_event(&event_id).await?,
    ))
}

pub(crate) async fn admin_package_preview(
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    let (bytes, _) = multipart_package_parts(multipart).await?;
    Ok(Json(crate::event_package::preview(&bytes).await?))
}

pub(crate) async fn admin_package_apply(
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_recent_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    let (bytes, base_fingerprint) = multipart_package_parts(multipart).await?;
    if base_fingerprint.trim().is_empty() {
        return Err(ApiError::from(crate::security::public_error(
            "Fingerprint base é obrigatório.",
        )));
    }
    Ok(Json(
        crate::event_package::apply(&bytes, &base_fingerprint, &session.user_id).await?,
    ))
}
