use super::super::*;

// Handlers — admin
// ---------------------------------------------------------------------------

pub(crate) async fn set_match_result(
    Path(match_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MatchResultBody>,
) -> ApiResult<impl IntoResponse> {
    let updated = crate::matches::set_match_result(
        String::new(),
        match_id,
        body.home_score,
        body.away_score,
        body.knockout,
        csrf_header(&headers),
    )
    .await?;
    Ok(Json(updated))
}

pub(crate) async fn set_knockout_released(
    headers: HeaderMap,
    Json(body): Json<KnockoutReleasedBody>,
) -> ApiResult<StatusCode> {
    crate::matches::set_knockout_released(String::new(), body.released, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn set_match_finished(
    Path(match_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MatchFinishedBody>,
) -> ApiResult<StatusCode> {
    crate::matches::set_match_finished(
        String::new(),
        match_id,
        body.finished,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn update_match_teams(
    Path(match_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UpdateTeamsBody>,
) -> ApiResult<StatusCode> {
    crate::matches::update_match_teams(
        String::new(),
        match_id,
        body.home_team,
        body.away_team,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn create_match(
    headers: HeaderMap,
    Json(body): Json<MatchScheduleBody>,
) -> ApiResult<impl IntoResponse> {
    let created = crate::matches::create_match(
        String::new(),
        body.home_team,
        body.away_team,
        body.phase,
        body.kickoff,
        csrf_header(&headers),
    )
    .await?;
    Ok(Json(created))
}

pub(crate) async fn update_match_schedule(
    Path(match_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MatchScheduleBody>,
) -> ApiResult<impl IntoResponse> {
    let updated = crate::matches::update_match_schedule(
        String::new(),
        match_id,
        body.home_team,
        body.away_team,
        body.phase,
        body.kickoff,
        csrf_header(&headers),
    )
    .await?;
    Ok(Json(updated))
}

pub(crate) async fn delete_match(
    Path(match_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::matches::delete_match(String::new(), match_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
