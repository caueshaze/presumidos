use super::super::*;

pub(crate) async fn admin_overview() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::admin_overview(String::new()).await?))
}

pub(crate) async fn admin_events() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::list_events_admin(String::new()).await?))
}

pub(crate) async fn admin_event_delete(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete_admin(String::new(), event_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_event_availability(
    Path(event_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<EventAvailabilityBody>,
) -> ApiResult<StatusCode> {
    crate::admin::set_pool_creation_enabled(
        String::new(),
        event_id,
        body.enabled,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_event_version_publish(
    Path((event_id, version_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let session = crate::auth::require_recent_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    crate::custom_event_manifest::publish_working_revision(
        &event_id,
        Some(&version_id),
        &session.user_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_event_version_restore(
    Path((event_id, version_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_recent_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    Ok(Json(
        crate::custom_event_manifest::restore_published_version(
            &event_id,
            &version_id,
            &session.user_id,
        )
        .await?,
    ))
}
