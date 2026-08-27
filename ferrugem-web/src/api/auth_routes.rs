use super::*;

// ---------------------------------------------------------------------------
// Handlers — auth
// ---------------------------------------------------------------------------

pub(crate) async fn register(Json(body): Json<RegisterBody>) -> ApiResult<StatusCode> {
    crate::auth::request_registration(body.username, body.email, body.password).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn register_confirm(
    Json(body): Json<RegisterConfirmBody>,
) -> ApiResult<impl IntoResponse> {
    let result = crate::auth::confirm_registration(body.email, body.code).await?;
    Ok(Json(result))
}

pub(crate) async fn password_reset(Json(body): Json<PasswordResetBody>) -> ApiResult<StatusCode> {
    crate::auth::request_password_reset(body.email).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn password_reset_confirm(
    Json(body): Json<PasswordResetConfirmBody>,
) -> ApiResult<StatusCode> {
    crate::auth::confirm_password_reset(body.email, body.code, body.new_password).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn login(Json(body): Json<LoginBody>) -> ApiResult<impl IntoResponse> {
    let result = crate::auth::login(body.username, body.password).await?;
    Ok(Json(result))
}

pub(crate) async fn logout(headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::auth::logout(String::new(), csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn current_user() -> ApiResult<impl IntoResponse> {
    let state = crate::auth::current_user(String::new()).await?;
    Ok(Json(state))
}

pub(crate) async fn contact_info() -> ApiResult<impl IntoResponse> {
    crate::security::apply_security_headers();
    let email = crate::config::settings()
        .contact_email
        .clone()
        .unwrap_or_default();
    Ok(Json(ContactInfoResponse { email }))
}

pub(crate) async fn reauth(
    headers: HeaderMap,
    Json(body): Json<ReauthBody>,
) -> ApiResult<StatusCode> {
    crate::auth::confirm_admin_password(body.password, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn change_username(
    headers: HeaderMap,
    Json(body): Json<ChangeUsernameBody>,
) -> ApiResult<impl IntoResponse> {
    let user =
        crate::auth::change_username(String::new(), body.username, csrf_header(&headers)).await?;
    Ok(Json(user))
}

pub(crate) async fn delete_account(headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::auth::delete_account(String::new(), csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn csrf() -> ApiResult<impl IntoResponse> {
    let state = crate::auth::current_user(String::new()).await?;
    Ok(Json(json!({ "csrfToken": state.csrf_token })))
}

pub(crate) async fn notification_status() -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::get_notification_status(String::new()).await?,
    ))
}

pub(crate) async fn update_notification_preference_handler(
    headers: HeaderMap,
    Json(body): Json<NotificationPreferenceBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::update_notification_preference(
            String::new(),
            body.enabled,
            body.lead_time_minutes,
            body.reaction_enabled,
            csrf_header(&headers),
        )
        .await?,
    ))
}

pub(crate) async fn upsert_push_subscription_handler(
    headers: HeaderMap,
    Json(body): Json<crate::models::WebPushSubscriptionInput>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::upsert_push_subscription(String::new(), body, csrf_header(&headers)).await?,
    ))
}

pub(crate) async fn remove_push_subscription_handler(
    headers: HeaderMap,
    Json(body): Json<SubscriptionRemoveBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::deactivate_push_subscription(
            String::new(),
            body.endpoint,
            csrf_header(&headers),
        )
        .await?,
    ))
}
