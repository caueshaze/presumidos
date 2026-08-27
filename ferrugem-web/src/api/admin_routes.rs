use super::*;

// ---------------------------------------------------------------------------
// Handlers — admin
// ---------------------------------------------------------------------------

pub(super) async fn set_match_result(
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

pub(super) async fn set_knockout_released(
    headers: HeaderMap,
    Json(body): Json<KnockoutReleasedBody>,
) -> ApiResult<StatusCode> {
    crate::matches::set_knockout_released(String::new(), body.released, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn set_match_finished(
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

pub(super) async fn update_match_teams(
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

pub(super) async fn create_match(
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

pub(super) async fn update_match_schedule(
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

pub(super) async fn set_match_fixture(
    Path(match_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MatchFixtureBody>,
) -> ApiResult<impl IntoResponse> {
    let updated = crate::matches::set_match_fixture(
        String::new(),
        match_id,
        body.external_fixture_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(Json(updated))
}

pub(super) async fn check_fixture(
    headers: HeaderMap,
    Json(body): Json<FixtureCheckBody>,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    let checked = crate::football::check_fixture_id(body.external_fixture_id).await?;
    Ok(Json(checked))
}

pub(super) async fn delete_match(
    Path(match_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::matches::delete_match(String::new(), match_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_overview() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::admin_overview(String::new()).await?))
}

pub(super) async fn admin_events() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::list_events_admin(String::new()).await?))
}

pub(super) async fn admin_event_delete(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete_admin(String::new(), event_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_event_availability(
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

pub(super) async fn admin_event_version_publish(
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

pub(super) async fn admin_event_version_restore(
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

pub(super) async fn admin_event_manifest_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_admin("").await?;
    let (manifest, content) = crate::custom_event_manifest::export_for_event(&event_id).await?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_manifest_exported",
        "event",
        Some(&event_id),
        None,
        json!({
            "schemaVersion": manifest.schema_version,
            "slug": manifest.slug,
            "manifestFingerprint": crate::custom_event_manifest::fingerprint(&manifest).unwrap_or_default(),
            "itemCount": manifest.items.len(),
            "optionCount": manifest.items.iter().map(|item| item.options.len()).sum::<usize>(),
        }),
    )
    .await?;
    let filename = format!("{}.json", manifest.slug);
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json; charset=utf-8")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(content))
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Não foi possível preparar o download do manifesto.".into(),
        })
}

pub(super) async fn custom_event_manifest_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_user("").await?;
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT e.id FROM events e WHERE e.id=?1 AND e.kind='custom' AND (e.created_by=?2 OR EXISTS(SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1))",
    )
    .bind(&event_id)
    .bind(&session.user_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("custom_manifest_access", e))?;
    if allowed.is_none() {
        return Err(ApiError::from(crate::security::public_error(
            "Você não pode exportar este evento.",
        )));
    }
    let (manifest, content) = crate::custom_event_manifest::export_for_event(&event_id).await?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_manifest_exported",
        "event",
        Some(&event_id),
        None,
        json!({
            "schemaVersion": manifest.schema_version,
            "slug": manifest.slug,
            "manifestFingerprint": crate::custom_event_manifest::fingerprint(&manifest).unwrap_or_default(),
            "itemCount": manifest.items.len(),
            "optionCount": manifest.items.iter().map(|item| item.options.len()).sum::<usize>(),
        }),
    )
    .await?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json; charset=utf-8")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{}.json\"", manifest.slug),
        )
        .body(Body::from(content))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível preparar o manifesto.",
            ))
        })
}

pub(super) async fn admin_manifest_preview(
    headers: HeaderMap,
    Json(body): Json<ManifestPreviewBody>,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    if body
        .filename
        .as_deref()
        .is_some_and(|name| name.len() > 255)
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Nome de arquivo inválido.".into(),
        });
    }
    if body
        .filename
        .as_deref()
        .is_some_and(|name| !name.to_ascii_lowercase().ends_with(".json"))
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Apenas arquivos JSON são aceitos.".into(),
        });
    }
    Ok(Json(
        crate::custom_event_manifest::preview(&body.content).await?,
    ))
}

pub(super) async fn admin_manifest_apply(
    headers: HeaderMap,
    Json(body): Json<ManifestApplyBody>,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_recent_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    if body
        .filename
        .as_deref()
        .is_some_and(|name| name.len() > 255)
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Nome de arquivo inválido.".into(),
        });
    }
    if body
        .filename
        .as_deref()
        .is_some_and(|name| !name.to_ascii_lowercase().ends_with(".json"))
    {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Apenas arquivos JSON são aceitos.".into(),
        });
    }
    Ok(Json(
        crate::custom_event_manifest::apply_admin(
            &body.content,
            &body.base_fingerprint,
            &session.user_id,
        )
        .await?,
    ))
}

pub(super) async fn admin_event_package_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_admin("").await?;
    let bytes = crate::event_package::export(&event_id).await?;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_package_exported",
        "event",
        Some(&event_id),
        None,
        json!({"byteSize": bytes.len()}),
    )
    .await?;
    let manifest = crate::custom_event_manifest::export_for_event(&event_id)
        .await?
        .0;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/zip")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{}.zip\"", manifest.slug),
        )
        .body(Body::from(bytes))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível preparar o pacote.",
            ))
        })
}

pub(super) async fn custom_event_package_export(
    Path(event_id): Path<String>,
) -> ApiResult<Response> {
    let session = crate::auth::require_user("").await?;
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT e.id FROM events e WHERE e.id=?1 AND e.kind='custom' AND (e.created_by=?2 OR EXISTS(SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1))",
    )
    .bind(&event_id)
    .bind(&session.user_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("custom_package_access", e))?;
    if allowed.is_none() {
        return Err(ApiError::from(crate::security::public_error(
            "Você não pode exportar este evento.",
        )));
    }
    let bytes = crate::event_package::export(&event_id).await?;
    let manifest = crate::custom_event_manifest::export_for_event(&event_id)
        .await?
        .0;
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(&session.user_id),
        "event_package_exported",
        "event",
        Some(&event_id),
        None,
        json!({"byteSize": bytes.len(), "slug": manifest.slug}),
    )
    .await?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/zip")
        .header(
            "content-disposition",
            format!("attachment; filename=\"{}.zip\"", manifest.slug),
        )
        .body(Body::from(bytes))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível preparar o pacote.",
            ))
        })
}

pub(super) async fn admin_package_preview(
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    let (bytes, _) = multipart_package_parts(multipart).await?;
    Ok(Json(crate::event_package::preview(&bytes).await?))
}

pub(super) async fn admin_package_apply(
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_recent_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    let (bytes, base_fingerprint) = multipart_package_parts(multipart).await?;
    if base_fingerprint.trim().is_empty() {
        return Err(ApiError::from(crate::security::public_error(
            "Fingerprint base é obrigatório.",
        )));
    }
    Ok(Json(
        crate::event_package::apply(&bytes, &base_fingerprint, &session.user_id).await?,
    ))
}

pub(super) async fn admin_finish_event(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::finish_event(String::new(), event_id, csrf_header(&headers)).await?,
    ))
}

pub(super) async fn admin_matches(
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

pub(super) async fn admin_match_audit(
    Path(match_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_match_audit(String::new(), match_id).await?,
    ))
}

pub(super) async fn admin_sync_status() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::latest_sync_status().await?))
}

pub(super) async fn admin_sync_run_now(headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::run_sync_now(String::new(), csrf_header(&headers)).await?,
    ))
}

pub(super) async fn admin_sync_backfill(headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::run_backfill_now(String::new(), csrf_header(&headers)).await?,
    ))
}

pub(super) async fn admin_predictions(
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

pub(super) async fn admin_prediction_reopen(
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

pub(super) async fn admin_prediction_reopen_revoke(
    headers: HeaderMap,
    Json(body): Json<RevokePredictionOverrideBody>,
) -> ApiResult<StatusCode> {
    crate::admin::revoke_prediction_reopen(String::new(), body.override_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_recalculate_match(
    headers: HeaderMap,
    Json(body): Json<RecalculateMatchBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::admin_recalculate_match(String::new(), body.match_id, csrf_header(&headers))
            .await?,
    ))
}

pub(super) async fn admin_recalculate_all(headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::admin_recalculate_all(String::new(), csrf_header(&headers)).await?,
    ))
}

pub(super) async fn admin_user_breakdown(
    Path(user_id): Path<String>,
    Query(query): Query<PoolIdQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::scoring::list_user_breakdowns(&user_id, &query.pool_id).await?,
    ))
}

pub(super) async fn admin_user_pools(Path(user_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_user_pools(String::new(), user_id).await?,
    ))
}

pub(super) async fn admin_block_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<BlockUserBody>,
) -> ApiResult<StatusCode> {
    crate::admin::block_user(String::new(), user_id, body.reason, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_unblock_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::unblock_user(String::new(), user_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_invalidate_user_sessions(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::invalidate_user_sessions_admin(String::new(), user_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_trigger_user_password_reset(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::trigger_user_password_reset(String::new(), user_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn admin_send_push_to_user(
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

pub(super) async fn admin_send_push_broadcast(
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

pub(super) async fn admin_audit(
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

pub(super) async fn admin_get_settings() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::load_admin_settings().await?))
}

pub(super) async fn public_settings() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::load_admin_settings().await?))
}

pub(super) async fn admin_save_settings(
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

pub(super) async fn leaderboard(Query(query): Query<PoolIdQuery>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::scoring::get_leaderboard(String::new(), query.pool_id).await?,
    ))
}
