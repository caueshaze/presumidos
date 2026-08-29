use super::*;

pub(crate) async fn submit_single_choice_prediction(
    headers: HeaderMap,
    Json(body): Json<SingleChoicePredictionBody>,
) -> ApiResult<StatusCode> {
    crate::custom_questions::submit_single_choice_prediction(
        String::new(),
        body.pool_id,
        body.item_id,
        body.option_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn submit_numeric_prediction(
    headers: HeaderMap,
    Json(body): Json<NumericPredictionBody>,
) -> ApiResult<StatusCode> {
    crate::numeric::submit_prediction(
        String::new(),
        body.pool_id,
        body.item_id,
        body.value,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn submit_multiple_choice_prediction(
    headers: HeaderMap,
    Json(body): Json<MultipleChoicePredictionBody>,
) -> ApiResult<StatusCode> {
    crate::multiple_choice::submit_prediction(
        String::new(),
        body.pool_id,
        body.item_id,
        body.option_ids,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn set_multiple_choice_result(
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MultipleChoiceResultBody>,
) -> ApiResult<StatusCode> {
    crate::multiple_choice::set_result_authorized(
        String::new(),
        item_id,
        body.option_ids,
        body.pool_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn mark_custom_result_not_representable(
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ResultNotRepresentableBody>,
) -> ApiResult<StatusCode> {
    crate::custom_questions::mark_result_not_representable_authorized(
        String::new(),
        item_id,
        body.reason,
        body.pool_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn custom_questions(
    Query(query): Query<CustomQuestionsQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::custom_questions::list_custom_questions(String::new(), query.pool_id).await?,
    ))
}

pub(crate) async fn custom_event_showcase(
    Query(query): Query<PoolIdQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::custom_questions::event_showcase(String::new(), query.pool_id).await?,
    ))
}

pub(crate) async fn update_option_media_progress(
    headers: HeaderMap,
    Json(body): Json<OptionMediaProgressBody>,
) -> ApiResult<StatusCode> {
    crate::custom_questions::set_option_media_seen(
        String::new(),
        body.pool_id,
        body.option_id,
        body.seen,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn custom_member_predictions(
    Path(pool_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::custom_questions::list_custom_member_predictions(String::new(), pool_id).await?,
    ))
}

pub(crate) async fn pool_football_scoring(
    Path(pool_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::football_scoring_config(String::new(), pool_id).await?,
    ))
}

pub(crate) async fn update_pool_football_scoring(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<FootballScoringBody>,
) -> ApiResult<StatusCode> {
    crate::pools::update_football_scoring_config(
        String::new(),
        pool_id,
        FootballScoringConfig {
            exact_score_points: body.exact_score_points,
            correct_result_exact_side_points: body.correct_result_exact_side_points,
            correct_result_points: body.correct_result_points,
            incorrect_result_points: body.incorrect_result_points,
            knockout_bonus_points: body.knockout_bonus_points,
        },
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn pool_custom_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::custom_item_scoring_config(String::new(), pool_id, item_id).await?,
    ))
}
pub(crate) async fn update_pool_custom_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<CustomScoringBody>,
) -> ApiResult<StatusCode> {
    crate::pools::update_custom_item_scoring_config(
        String::new(),
        pool_id,
        item_id,
        body.correct_points,
        body.incorrect_points,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn set_custom_question_result(
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CustomResultBody>,
) -> ApiResult<StatusCode> {
    crate::custom_questions::set_correct_option_authorized(
        String::new(),
        item_id,
        body.option_id,
        body.pool_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn set_numeric_question_result(
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<NumericResultBody>,
) -> ApiResult<StatusCode> {
    crate::numeric::set_result_authorized(
        String::new(),
        item_id,
        body.value,
        body.pool_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn pool_numeric_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::numeric_item_scoring_config(String::new(), pool_id, item_id).await?,
    ))
}
pub(crate) async fn pool_multiple_choice_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::multiple_choice_item_scoring_config(String::new(), pool_id, item_id).await?,
    ))
}
pub(crate) async fn update_pool_multiple_choice_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<MultipleChoiceScoringBody>,
) -> ApiResult<StatusCode> {
    crate::pools::update_multiple_choice_item_scoring_config(
        String::new(),
        pool_id,
        item_id,
        body.exact_points,
        body.partial_points,
        body.incorrect_points,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(crate) async fn update_pool_numeric_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<NumericScoringBody>,
) -> ApiResult<StatusCode> {
    crate::pools::update_numeric_item_scoring_config(
        String::new(),
        pool_id,
        item_id,
        body.exact_points,
        body.tolerance,
        body.within_tolerance_points,
        body.incorrect_points,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
