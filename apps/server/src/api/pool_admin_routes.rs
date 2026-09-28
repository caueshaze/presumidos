use super::*;

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
