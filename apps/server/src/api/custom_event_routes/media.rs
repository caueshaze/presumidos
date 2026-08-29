use super::*;

pub(crate) async fn multipart_bytes(mut multipart: Multipart) -> Result<Vec<u8>, ServerFnError> {
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| crate::security::public_error("Upload inválido."))?
    {
        if field.name() != Some("file") && field.name() != Some("image") {
            continue;
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| crate::security::public_error("Não foi possível ler a imagem."))?
        {
            if bytes.len() + chunk.len() > crate::config::settings().asset_max_upload_bytes {
                return Err(crate::security::public_error(format!(
                    "A imagem excede {} MB.",
                    crate::config::settings().asset_max_upload_bytes / (1024 * 1024)
                )));
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(crate::security::public_error("Selecione uma imagem."));
        }
        return Ok(bytes);
    }
    Err(crate::security::public_error("Selecione uma imagem."))
}

pub(crate) async fn multipart_package_parts(
    mut multipart: Multipart,
) -> Result<(Vec<u8>, String), ServerFnError> {
    let mut bytes = None;
    let mut base_fingerprint = String::new();
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| crate::security::public_error("Upload de pacote inválido."))?
    {
        match field.name() {
            Some("file") | Some("package") => {
                let mut content = Vec::new();
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|_| crate::security::public_error("Não foi possível ler o pacote."))?
                {
                    if content.len() + chunk.len() > 128 * 1024 * 1024 {
                        return Err(crate::security::public_error(
                            "O pacote excede o limite permitido.",
                        ));
                    }
                    content.extend_from_slice(&chunk);
                }
                bytes = Some(content);
            }
            Some("baseFingerprint") => {
                base_fingerprint = field
                    .text()
                    .await
                    .map_err(|_| crate::security::public_error("Fingerprint inválido."))?;
            }
            _ => {}
        }
    }
    Ok((
        bytes.ok_or_else(|| crate::security::public_error("Selecione um pacote."))?,
        base_fingerprint,
    ))
}

pub(crate) async fn custom_event_cover_upload(
    Path(event_id): Path<String>,
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::assets::upload_cover(
            String::new(),
            event_id,
            multipart_bytes(multipart).await?,
            csrf_header(&headers),
        )
        .await?,
    ))
}

pub(crate) async fn custom_event_cover_remove(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::assets::remove_cover(String::new(), event_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn custom_event_option_upload(
    Path((event_id, _item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::assets::upload_option(
            String::new(),
            event_id,
            option_id,
            multipart_bytes(multipart).await?,
            csrf_header(&headers),
        )
        .await?,
    ))
}

pub(crate) async fn custom_event_option_remove(
    Path((event_id, _item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::assets::remove_option(String::new(), event_id, option_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn media_asset(Path((asset_id, variant)): Path<(String, String)>) -> ApiResult<Response> {
    if !crate::assets::can_read(&asset_id).await? {
        return Err(ApiError::from(crate::security::public_error(
            "Asset não encontrado.",
        )));
    }
    let (bytes, sha256) = crate::assets::read_variant(&asset_id, &variant).await?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "image/webp")
        .header("cache-control", "public, max-age=31536000, immutable")
        .header("etag", format!("\"{sha256}-{variant}\""))
        .body(Body::from(bytes))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível servir o asset.",
            ))
        })
}
pub(crate) async fn custom_event_delete_option(
    Path((id, item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete_option(
        String::new(),
        id,
        item_id,
        option_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn custom_event_move_option(
    Path((id, item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(body): Json<MoveBody>,
) -> ApiResult<StatusCode> {
    crate::custom_events::move_option(
        String::new(),
        id,
        item_id,
        option_id,
        body.direction,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn custom_event_publish(
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::publish(String::new(), id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
