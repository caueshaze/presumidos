use super::*;
use serde_json::json;

pub(crate) async fn custom_events_mine() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::mine(String::new()).await?))
}
pub(crate) async fn custom_events_available() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::available(String::new()).await?))
}
pub(crate) async fn custom_event_create(
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
pub(crate) async fn custom_event_get(Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::get(String::new(), id).await?))
}
pub(crate) async fn custom_event_draft(Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::draft(String::new(), id).await?))
}
pub(crate) async fn custom_event_update(
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
pub(crate) async fn custom_event_delete(
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete(String::new(), id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn custom_event_add_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_item(String::new(),id,body.title,body.lock_at,body.reveal_at,csrf_header(&headers)).await?}),
    ))
}
pub(crate) async fn custom_event_add_numeric_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateNumericItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_numeric_item(String::new(),id,body.title,body.lock_at,body.reveal_at,body.decimal_places,body.unit_label,body.min_value,body.max_value,csrf_header(&headers)).await?}),
    ))
}
pub(crate) async fn custom_event_add_multiple_choice_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateMultipleChoiceItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_multiple_choice_item(String::new(),id,body.title,body.lock_at,body.reveal_at,body.min_selections,body.max_selections,csrf_header(&headers)).await?}),
    ))
}
pub(crate) async fn custom_event_add_option(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<CreateOptionBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_option(String::new(),id,item_id,body.label,csrf_header(&headers)).await?}),
    ))
}
pub(crate) async fn custom_event_update_item(
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
pub(crate) async fn custom_event_delete_item(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete_item(String::new(), id, item_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn custom_event_move_item(
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
pub(crate) async fn custom_event_update_option(
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
pub(crate) async fn custom_event_update_option_media(
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
