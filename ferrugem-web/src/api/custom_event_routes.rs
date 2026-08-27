use super::*;
use serde_json::json;

pub(super) async fn custom_events_mine() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::mine(String::new()).await?))
}
pub(super) async fn custom_events_available() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::available(String::new()).await?))
}
pub(super) async fn custom_event_create(
    headers: HeaderMap,
    Json(body): Json<CreateEventBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::custom_events::create(
            String::new(),
            body.name,
            body.starts_at,
            body.ends_at,
            csrf_header(&headers),
        )
        .await?,
    ))
}
pub(super) async fn custom_event_get(Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::get(String::new(), id).await?))
}
pub(super) async fn custom_event_draft(Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::draft(String::new(), id).await?))
}
pub(super) async fn custom_event_update(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UpdateEventBody>,
) -> ApiResult<StatusCode> {
    crate::custom_events::update_metadata(
        String::new(),
        id,
        body.name,
        body.starts_at,
        body.ends_at,
        body.description,
        body.cover_url,
        body.external_url,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn custom_event_delete(
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete(String::new(), id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn custom_event_add_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_item(String::new(),id,body.title,body.lock_at,body.reveal_at,csrf_header(&headers)).await?}),
    ))
}
pub(super) async fn custom_event_add_numeric_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateNumericItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_numeric_item(String::new(),id,body.title,body.lock_at,body.reveal_at,body.decimal_places,body.unit_label,body.min_value,body.max_value,csrf_header(&headers)).await?}),
    ))
}
pub(super) async fn custom_event_add_multiple_choice_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateMultipleChoiceItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_multiple_choice_item(String::new(),id,body.title,body.lock_at,body.reveal_at,body.min_selections,body.max_selections,csrf_header(&headers)).await?}),
    ))
}
pub(super) async fn custom_event_add_option(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<CreateOptionBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_option(String::new(),id,item_id,body.label,csrf_header(&headers)).await?}),
    ))
}
pub(super) async fn custom_event_update_item(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<UpdateItemBody>,
) -> ApiResult<StatusCode> {
    crate::custom_events::update_item(
        String::new(),
        id,
        item_id,
        body.title,
        body.lock_at,
        body.reveal_at,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn custom_event_delete_item(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete_item(String::new(), id, item_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn custom_event_move_item(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<MoveBody>,
) -> ApiResult<StatusCode> {
    crate::custom_events::move_item(
        String::new(),
        id,
        item_id,
        body.direction,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn custom_event_update_option(
    Path((id, item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(body): Json<CreateOptionBody>,
) -> ApiResult<StatusCode> {
    crate::custom_events::update_option(
        String::new(),
        id,
        item_id,
        option_id,
        body.label,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn custom_event_update_option_media(
    Path((id, item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(body): Json<UpdateOptionMediaBody>,
) -> ApiResult<StatusCode> {
    crate::custom_events::update_option_media(
        String::new(),
        id,
        item_id,
        option_id,
        body.image_url,
        body.links,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn multipart_bytes(mut multipart: Multipart) -> Result<Vec<u8>, ServerFnError> {
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

pub(super) async fn multipart_package_parts(
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

pub(super) async fn custom_event_cover_upload(
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

pub(super) async fn custom_event_cover_remove(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::assets::remove_cover(String::new(), event_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn custom_event_option_upload(
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

pub(super) async fn custom_event_option_remove(
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
pub(super) async fn custom_event_delete_option(
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
pub(super) async fn custom_event_move_option(
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
pub(super) async fn custom_event_publish(
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::publish(String::new(), id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
