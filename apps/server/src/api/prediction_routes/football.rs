use super::*;

// ---------------------------------------------------------------------------
// Handlers — matches / predictions
// ---------------------------------------------------------------------------

pub(crate) async fn list_matches() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::matches::list_matches(String::new()).await?))
}

pub(crate) async fn knockout_released() -> ApiResult<impl IntoResponse> {
    let released = crate::matches::is_knockout_released().await?;
    Ok(Json(json!({ "released": released })))
}

pub(crate) async fn my_predictions(
    Query(query): Query<PoolIdQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::matches::get_my_predictions(String::new(), query.pool_id).await?,
    ))
}

pub(crate) async fn my_prediction_overrides() -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_my_prediction_overrides(String::new()).await?,
    ))
}

pub(crate) async fn my_match_points() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::scoring::list_my_match_points().await?))
}

pub(crate) async fn submit_prediction(
    headers: HeaderMap,
    Json(body): Json<PredictionBody>,
) -> ApiResult<StatusCode> {
    crate::matches::submit_prediction(
        String::new(),
        body.pool_id,
        body.match_id,
        body.home_score,
        body.away_score,
        body.knockout,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn prediction_reuse_suggestion(
    Path(pool_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::prediction_reuse::suggestion(String::new(), pool_id).await?,
    ))
}

pub(crate) async fn prediction_reuse_copy(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::prediction_reuse::copy(String::new(), pool_id, csrf_header(&headers)).await?,
    ))
}

pub(crate) async fn prediction_reuse_start_empty(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::prediction_reuse::start_empty(String::new(), pool_id, csrf_header(&headers)).await?,
    ))
}
