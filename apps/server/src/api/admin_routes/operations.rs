use super::super::*;

pub(crate) async fn admin_finish_event(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::finish_event(String::new(), event_id, csrf_header(&headers)).await?,
    ))
}

pub(crate) async fn admin_matches(
    Query(query): Query<AdminMatchListQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_admin_matches(
            String::new(),
            query.phase,
            query.group_name,
            query.date,
            query.status,
            query.origin,
        )
        .await?,
    ))
}

pub(crate) async fn admin_match_audit(
    Path(match_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_match_audit(String::new(), match_id).await?,
    ))
}

pub(crate) async fn admin_predictions(
    Query(query): Query<AdminPredictionsQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_admin_predictions(
            String::new(),
            query.match_id,
            query.user_id,
            query.pool_id,
            query.missing_only.unwrap_or(false),
        )
        .await?,
    ))
}

pub(crate) async fn admin_prediction_reopen(
    headers: HeaderMap,
    Json(body): Json<ReopenPredictionBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::reopen_prediction(
            String::new(),
            body.match_id,
            body.user_id,
            body.reason,
            body.expires_at,
            csrf_header(&headers),
        )
        .await?,
    ))
}

pub(crate) async fn admin_prediction_reopen_revoke(
    headers: HeaderMap,
    Json(body): Json<RevokePredictionOverrideBody>,
) -> ApiResult<StatusCode> {
    crate::admin::revoke_prediction_reopen(String::new(), body.override_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_recalculate_match(
    headers: HeaderMap,
    Json(body): Json<RecalculateMatchBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::admin_recalculate_match(String::new(), body.match_id, csrf_header(&headers))
            .await?,
    ))
}

pub(crate) async fn admin_recalculate_all(headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::admin_recalculate_all(String::new(), csrf_header(&headers)).await?,
    ))
}

pub(crate) async fn admin_user_breakdown(
    Path(user_id): Path<String>,
    Query(query): Query<PoolIdQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::scoring::list_user_breakdowns(&user_id, &query.pool_id).await?,
    ))
}

pub(crate) async fn admin_user_pools(Path(user_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_user_pools(String::new(), user_id).await?,
    ))
}

pub(crate) async fn admin_block_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<BlockUserBody>,
) -> ApiResult<StatusCode> {
    crate::admin::block_user(String::new(), user_id, body.reason, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_unblock_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::unblock_user(String::new(), user_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_invalidate_user_sessions(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::invalidate_user_sessions_admin(String::new(), user_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_trigger_user_password_reset(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::trigger_user_password_reset(String::new(), user_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_send_push_to_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AdminPushBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::send_admin_push_to_user(
            String::new(),
            user_id,
            body.title,
            body.body,
            body.url,
            csrf_header(&headers),
        )
        .await?,
    ))
}

pub(crate) async fn admin_send_push_broadcast(
    headers: HeaderMap,
    Json(body): Json<AdminPushBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::send_admin_push_broadcast(
            String::new(),
            body.title,
            body.body,
            body.url,
            csrf_header(&headers),
        )
        .await?,
    ))
}

pub(crate) async fn admin_audit(
    Query(query): Query<AdminAuditQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_audit(
            String::new(),
            query.action,
            query.actor_user_id,
            query.target_type,
            query.target_id,
        )
        .await?,
    ))
}

pub(crate) async fn admin_get_settings() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::load_admin_settings().await?))
}

pub(crate) async fn public_settings() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::load_admin_settings().await?))
}

pub(crate) async fn admin_save_settings(
    headers: HeaderMap,
    Json(body): Json<AdminSettings>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::save_admin_settings(String::new(), body, csrf_header(&headers)).await?,
    ))
}

// ---------------------------------------------------------------------------
// Handlers — leaderboard
// ---------------------------------------------------------------------------

pub(crate) async fn leaderboard(Query(query): Query<PoolIdQuery>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::scoring::get_leaderboard(String::new(), query.pool_id).await?,
    ))
}
