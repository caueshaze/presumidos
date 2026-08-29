use super::*;

// ---------------------------------------------------------------------------
// Handlers — pools
// ---------------------------------------------------------------------------

pub(super) async fn list_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::pools::list_my_pools(String::new()).await?))
}

pub(super) async fn dashboard_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::pools::dashboard_pools(String::new()).await?))
}

pub(super) async fn create_pool(
    headers: HeaderMap,
    Json(body): Json<CreatePoolBody>,
) -> ApiResult<impl IntoResponse> {
    let pool = crate::pools::create_pool_for_event(
        String::new(),
        body.name,
        body.event_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(Json(pool))
}

pub(super) async fn join_pool(
    headers: HeaderMap,
    Json(body): Json<JoinPoolBody>,
) -> ApiResult<impl IntoResponse> {
    let result =
        crate::pools::join_pool(String::new(), body.invite_code, csrf_header(&headers)).await;
    let pool = match result {
        Ok(pool) => pool,
        Err(error) => {
            crate::operability::metrics()
                .invite_join_failure
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(error.into());
        }
    };
    Ok(Json(pool))
}

pub(super) async fn leave_pool(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::pools::leave_pool(String::new(), pool_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn close_predictions(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::close_predictions(String::new(), pool_id, csrf_header(&headers)).await?,
    ))
}

pub(super) async fn close_pool(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::close_pool(String::new(), pool_id, csrf_header(&headers)).await?,
    ))
}

pub(super) async fn create_pool_report(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PoolReportBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::create_pool_report(
            String::new(),
            pool_id,
            body.category,
            body.details,
            csrf_header(&headers),
        )
        .await?,
    ))
}

pub(super) async fn public_pool_invite_preview(
    Path(token): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let preview = crate::pools::public_invite_preview(token).await?;
    Ok((
        [(axum::http::header::CACHE_CONTROL, "private, no-store")],
        Json(preview),
    ))
}

pub(super) async fn pool_member_predictions(
    Path(pool_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::get_pool_member_predictions(String::new(), pool_id).await?,
    ))
}

pub(super) async fn react_to_prediction(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PredictionReactionBody>,
) -> ApiResult<StatusCode> {
    crate::pools::react_to_prediction(
        String::new(),
        pool_id,
        body.target_user_id,
        body.prediction_id,
        body.match_id,
        body.emoji,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn mark_prediction_reactions_seen(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::pools::mark_prediction_reactions_seen(String::new(), pool_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn pool_breakdowns(Path(pool_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::scoring::list_pool_breakdowns(&pool_id).await?))
}

pub(super) async fn list_pool_adjustments(
    Path(pool_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::list_pool_adjustments(String::new(), pool_id).await?,
    ))
}

pub(super) async fn add_point_adjustment(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AdjustmentBody>,
) -> ApiResult<StatusCode> {
    crate::pools::add_point_adjustment(
        String::new(),
        pool_id,
        body.user_id,
        body.delta,
        body.reason,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn remove_point_adjustment(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RemoveAdjustmentBody>,
) -> ApiResult<StatusCode> {
    crate::pools::remove_point_adjustment(
        String::new(),
        pool_id,
        body.adjustment_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn delete_pool(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::pools::delete_pool(String::new(), pool_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Handlers — admin: gestão de membros de bolões
// ---------------------------------------------------------------------------

pub(super) async fn admin_list_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::list_all_pools_admin(String::new()).await?,
    ))
}

pub(super) async fn admin_list_users() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::list_admin_users(String::new()).await?))
}

pub(super) async fn admin_list_pool_members(
    Path(pool_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::list_pool_members_admin(String::new(), pool_id).await?,
    ))
}

pub(super) async fn admin_add_pool_member(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PoolMemberBody>,
) -> ApiResult<StatusCode> {
    crate::pools::add_pool_member_admin(
        String::new(),
        pool_id,
        body.user_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_remove_pool_member(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PoolMemberBody>,
) -> ApiResult<StatusCode> {
    crate::pools::remove_pool_member_admin(
        String::new(),
        pool_id,
        body.user_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_list_pool_reports(
    Query(query): Query<PoolReportQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::list_pool_reports_admin(String::new(), query.status).await?,
    ))
}

pub(super) async fn admin_update_pool_report_status(
    Path(report_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PoolReportStatusBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::update_pool_report_status_admin(
            String::new(),
            report_id,
            body.status,
            csrf_header(&headers),
        )
        .await?,
    ))
}
