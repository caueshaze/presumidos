//! Camada HTTP/JSON (Axum) que expõe a lógica de negócio como uma API REST estável.
//!
//! Substitui a camada RPC do Dioxus (`#[server]`). Cada handler é fino: extrai corpo/headers,
//! chama a função de negócio correspondente (que lê a sessão pelo cookie e escreve headers de
//! resposta via [crate::context]) e serializa o resultado em JSON. Erros viram
//! `{ "error": "..." }` com o status HTTP apropriado.

#![cfg(feature = "server")]

use std::net::SocketAddr;
use std::time::Instant;

use axum::{
    body::Body,
    extract::{ConnectInfo, Multipart, Path, Query, Request},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::FutureExt;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::context::{take_response_headers, RequestContext, REQUEST};
use crate::error::ServerFnError;
use crate::models::{AdminSettings, FootballScoringConfig, KnockoutEntry};

// ---------------------------------------------------------------------------
// Erro -> resposta HTTP
// ---------------------------------------------------------------------------

pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl From<ServerFnError> for ApiError {
    fn from(error: ServerFnError) -> Self {
        let message = error.message().to_string();
        let status = if message == "SECURITY:ADMIN_REAUTH_REQUIRED" {
            StatusCode::FORBIDDEN
        } else if message.starts_with("Falha de seguranca da sessao") {
            // CSRF inválido/expirado → 403, para o cliente renovar o token e tentar de novo.
            StatusCode::FORBIDDEN
        } else if message.starts_with("Sessao invalida") {
            StatusCode::UNAUTHORIZED
        } else if message.starts_with("O servidor nao conseguiu")
            || message.starts_with("O servidor não conseguiu")
        {
            StatusCode::INTERNAL_SERVER_ERROR
        } else if message.starts_with("STORAGE:") {
            StatusCode::from_u16(507).expect("507 é um status HTTP válido")
        } else {
            StatusCode::BAD_REQUEST
        };
        ApiError { status, message }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            let error_id = Uuid::new_v4().to_string();
            crate::operability::metrics()
                .responses_5xx
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            crate::security::log_event(
                "http_error",
                json!({
                    "error_id": error_id,
                    "status": self.status.as_u16(),
                    "message": self.message,
                }),
            );
            (
                self.status,
                Json(json!({
                    "error": "Ocorreu um erro interno.",
                    "errorId": error_id,
                })),
            )
                .into_response()
        } else {
            (self.status, Json(json!({ "error": self.message }))).into_response()
        }
    }
}

type ApiResult<T> = Result<T, ApiError>;

fn csrf_header(headers: &HeaderMap) -> String {
    headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

// ---------------------------------------------------------------------------
// Middleware: instala o contexto de requisição (task-local) e drena os headers
// de resposta acumulados pela lógica de negócio.
// ---------------------------------------------------------------------------

pub async fn context_middleware(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers().clone();
    let request_id = Uuid::new_v4().to_string();
    let started = Instant::now();
    let path = request.uri().path().to_string();
    let method = request.method().as_str().to_string();
    let began = crate::operability::runtime_state().begin_request();
    if began {
        crate::operability::metrics()
            .in_flight
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    let ctx = RequestContext::new(headers, Some(peer), request_id.clone());

    let scoped = REQUEST.scope(ctx, async move {
        // Headers de segurança em toda resposta (inclusive estáticos).
        crate::security::apply_security_headers();
        let response = next.run(request).await;
        (response, take_response_headers())
    });
    let (mut response, extra_headers) =
        match std::panic::AssertUnwindSafe(scoped).catch_unwind().await {
            Ok(result) => result,
            Err(_) => {
                let error_id = Uuid::new_v4().to_string();
                crate::operability::metrics()
                    .responses_5xx
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                crate::security::log_event(
                    "handler_panic",
                    json!({ "error_id": error_id, "route": path }),
                );
                (
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Ocorreu um erro interno.",
                            "errorId": error_id,
                        })),
                    )
                        .into_response(),
                    Vec::new(),
                )
            }
        };

    if began {
        crate::operability::runtime_state().end_request();
        crate::operability::metrics()
            .in_flight
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
    crate::operability::metrics()
        .requests
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    crate::security::log_event(
        "http_request",
        json!({
            "request_id": request_id,
            "method": method,
            "route": path,
            "status": response.status().as_u16(),
            "duration_ms": started.elapsed().as_secs_f64() * 1000.0,
        }),
    );
    if let Ok(value) = request_id.parse() {
        response.headers_mut().insert("x-request-id", value);
    }
    if path.starts_with("/api/admin") || path.starts_with("/api/auth") {
        response.headers_mut().insert(
            "cache-control",
            axum::http::HeaderValue::from_static("no-store"),
        );
    }

    let response_headers = response.headers_mut();
    for (name, value) in extra_headers {
        // Cookies podem se repetir; o resto sobrescreve (dedup de headers de segurança).
        if name == axum::http::header::SET_COOKIE {
            response_headers.append(name, value);
        } else {
            response_headers.insert(name, value);
        }
    }
    response
}

// ---------------------------------------------------------------------------
// Corpos de requisição
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RegisterBody {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterConfirmBody {
    email: String,
    code: String,
}

#[derive(Deserialize)]
struct PasswordResetBody {
    email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PasswordResetConfirmBody {
    email: String,
    code: String,
    new_password: String,
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct ChangeUsernameBody {
    username: String,
}

#[derive(Deserialize)]
struct ReauthBody {
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePoolBody {
    name: String,
    #[serde(default)]
    event_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateEventBody {
    name: String,
    starts_at: Option<String>,
    ends_at: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateItemBody {
    title: String,
    lock_at: String,
    reveal_at: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNumericItemBody {
    title: String,
    lock_at: String,
    reveal_at: String,
    decimal_places: i64,
    unit_label: Option<String>,
    min_value: Option<String>,
    max_value: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateMultipleChoiceItemBody {
    title: String,
    lock_at: String,
    reveal_at: String,
    min_selections: i64,
    max_selections: Option<i64>,
}
#[derive(Deserialize)]
struct CreateOptionBody {
    label: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateOptionMediaBody {
    image_url: Option<String>,
    #[serde(default)]
    links: Vec<crate::custom_events::BuilderOptionLink>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateEventBody {
    name: String,
    starts_at: Option<String>,
    ends_at: Option<String>,
    description: Option<String>,
    cover_url: Option<String>,
    external_url: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateItemBody {
    title: String,
    lock_at: String,
    reveal_at: String,
}
#[derive(Deserialize)]
struct MoveBody {
    direction: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestPreviewBody {
    content: String,
    #[serde(default)]
    filename: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestApplyBody {
    content: String,
    base_fingerprint: String,
    #[serde(default)]
    filename: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinPoolBody {
    invite_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PredictionBody {
    match_id: String,
    home_score: i64,
    away_score: i64,
    #[serde(default)]
    knockout: KnockoutEntry,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SingleChoicePredictionBody {
    pool_id: String,
    item_id: String,
    option_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NumericPredictionBody {
    pool_id: String,
    item_id: String,
    value: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultipleChoicePredictionBody {
    pool_id: String,
    item_id: String,
    option_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultipleChoiceResultBody {
    option_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomQuestionsQuery {
    pool_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FootballScoringBody {
    exact_score_points: i64,
    correct_result_exact_side_points: i64,
    correct_result_points: i64,
    incorrect_result_points: i64,
    knockout_bonus_points: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomScoringBody {
    correct_points: i64,
    incorrect_points: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomResultBody {
    option_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NumericScoringBody {
    exact_points: i64,
    tolerance: String,
    within_tolerance_points: i64,
    incorrect_points: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultipleChoiceScoringBody {
    exact_points: i64,
    partial_points: i64,
    incorrect_points: i64,
}
#[derive(Deserialize)]
struct NumericResultBody {
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchResultBody {
    home_score: i64,
    away_score: i64,
    #[serde(default)]
    knockout: KnockoutEntry,
}

#[derive(Deserialize)]
struct KnockoutReleasedBody {
    released: bool,
}

#[derive(Deserialize)]
struct MatchFinishedBody {
    finished: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTeamsBody {
    home_team: String,
    away_team: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchFixtureBody {
    /// `None` limpa o mapeamento; um id positivo aponta o jogo ao evento externo.
    external_fixture_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCheckBody {
    external_fixture_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchScheduleBody {
    home_team: String,
    away_team: String,
    phase: String,
    kickoff: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolMemberBody {
    user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdjustmentBody {
    user_id: String,
    delta: i64,
    #[serde(default)]
    reason: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveAdjustmentBody {
    adjustment_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminMatchListQuery {
    phase: Option<String>,
    group_name: Option<String>,
    date: Option<String>,
    status: Option<String>,
    origin: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminPredictionsQuery {
    match_id: Option<String>,
    user_id: Option<String>,
    pool_id: Option<String>,
    missing_only: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminAuditQuery {
    action: Option<String>,
    actor_user_id: Option<String>,
    target_type: Option<String>,
    target_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReopenPredictionBody {
    match_id: String,
    user_id: String,
    reason: String,
    expires_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevokePredictionOverrideBody {
    override_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecalculateMatchBody {
    match_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockUserBody {
    reason: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminPushBody {
    title: String,
    body: String,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolIdQuery {
    pool_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationPreferenceBody {
    enabled: bool,
    lead_time_minutes: i64,
    reaction_enabled: bool,
}

#[derive(Deserialize)]
struct SubscriptionRemoveBody {
    endpoint: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PredictionReactionBody {
    target_user_id: String,
    prediction_id: Option<String>,
    match_id: Option<String>,
    emoji: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OptionMediaProgressBody {
    pool_id: String,
    option_id: String,
    seen: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ContactInfoResponse {
    email: String,
}

// ---------------------------------------------------------------------------
// Handlers — auth
// ---------------------------------------------------------------------------

async fn register(Json(body): Json<RegisterBody>) -> ApiResult<StatusCode> {
    crate::auth::request_registration(body.username, body.email, body.password).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn register_confirm(Json(body): Json<RegisterConfirmBody>) -> ApiResult<impl IntoResponse> {
    let result = crate::auth::confirm_registration(body.email, body.code).await?;
    Ok(Json(result))
}

async fn password_reset(Json(body): Json<PasswordResetBody>) -> ApiResult<StatusCode> {
    crate::auth::request_password_reset(body.email).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn password_reset_confirm(
    Json(body): Json<PasswordResetConfirmBody>,
) -> ApiResult<StatusCode> {
    crate::auth::confirm_password_reset(body.email, body.code, body.new_password).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn login(Json(body): Json<LoginBody>) -> ApiResult<impl IntoResponse> {
    let result = crate::auth::login(body.username, body.password).await?;
    Ok(Json(result))
}

async fn logout(headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::auth::logout(String::new(), csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn current_user() -> ApiResult<impl IntoResponse> {
    let state = crate::auth::current_user(String::new()).await?;
    Ok(Json(state))
}

async fn contact_info() -> ApiResult<impl IntoResponse> {
    crate::security::apply_security_headers();
    let email = crate::config::settings()
        .contact_email
        .clone()
        .unwrap_or_default();
    Ok(Json(ContactInfoResponse { email }))
}

async fn reauth(headers: HeaderMap, Json(body): Json<ReauthBody>) -> ApiResult<StatusCode> {
    crate::auth::confirm_admin_password(body.password, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn change_username(
    headers: HeaderMap,
    Json(body): Json<ChangeUsernameBody>,
) -> ApiResult<impl IntoResponse> {
    let user =
        crate::auth::change_username(String::new(), body.username, csrf_header(&headers)).await?;
    Ok(Json(user))
}

async fn delete_account(headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::auth::delete_account(String::new(), csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn csrf() -> ApiResult<impl IntoResponse> {
    let state = crate::auth::current_user(String::new()).await?;
    Ok(Json(json!({ "csrfToken": state.csrf_token })))
}

async fn notification_status() -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::get_notification_status(String::new()).await?,
    ))
}

async fn update_notification_preference_handler(
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

async fn upsert_push_subscription_handler(
    headers: HeaderMap,
    Json(body): Json<crate::models::WebPushSubscriptionInput>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::push::upsert_push_subscription(String::new(), body, csrf_header(&headers)).await?,
    ))
}

async fn remove_push_subscription_handler(
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

pub async fn health_live() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub async fn health_ready() -> impl IntoResponse {
    if !crate::operability::runtime_state().is_accepting() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable" })),
        );
    }
    let report = crate::operability::readiness_report().await;
    if report.state == crate::operability::ReadinessState::Ready {
        (StatusCode::OK, Json(json!({ "status": "ok" })))
    } else {
        crate::security::log_event(
            "readiness_failed",
            json!({
                "database": report.database,
                "migrations": report.migrations,
                "assets": report.assets,
                "disk": report.disk,
            }),
        );
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable" })),
        )
    }
}

async fn health() -> impl IntoResponse {
    health_live().await
}

pub async fn internal_metrics(headers: HeaderMap) -> impl IntoResponse {
    if !crate::config::settings().metrics_enabled {
        return (StatusCode::NOT_FOUND, "not found".to_string()).into_response();
    }
    if crate::config::settings().app_env == "production"
        && crate::security::enforce_trusted_proxy(&headers).is_err()
    {
        return (StatusCode::NOT_FOUND, "not found".to_string()).into_response();
    }
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        crate::operability::metrics_text(),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Handlers — pools
// ---------------------------------------------------------------------------

async fn list_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::pools::list_my_pools(String::new()).await?))
}

async fn dashboard_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::pools::dashboard_pools(String::new()).await?))
}

async fn create_pool(
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

async fn custom_events_mine() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::mine(String::new()).await?))
}
async fn custom_events_available() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::available(String::new()).await?))
}
async fn custom_event_create(
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
async fn custom_event_get(Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::get(String::new(), id).await?))
}
async fn custom_event_draft(Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::custom_events::draft(String::new(), id).await?))
}
async fn custom_event_update(
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
async fn custom_event_delete(Path(id): Path<String>, headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::custom_events::delete(String::new(), id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn custom_event_add_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_item(String::new(),id,body.title,body.lock_at,body.reveal_at,csrf_header(&headers)).await?}),
    ))
}
async fn custom_event_add_numeric_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateNumericItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_numeric_item(String::new(),id,body.title,body.lock_at,body.reveal_at,body.decimal_places,body.unit_label,body.min_value,body.max_value,csrf_header(&headers)).await?}),
    ))
}
async fn custom_event_add_multiple_choice_item(
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateMultipleChoiceItemBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_multiple_choice_item(String::new(),id,body.title,body.lock_at,body.reveal_at,body.min_selections,body.max_selections,csrf_header(&headers)).await?}),
    ))
}
async fn custom_event_add_option(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<CreateOptionBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        json!({"id":crate::custom_events::add_option(String::new(),id,item_id,body.label,csrf_header(&headers)).await?}),
    ))
}
async fn custom_event_update_item(
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
async fn custom_event_delete_item(
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete_item(String::new(), id, item_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn custom_event_move_item(
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
async fn custom_event_update_option(
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
async fn custom_event_update_option_media(
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

async fn multipart_bytes(mut multipart: Multipart) -> Result<Vec<u8>, ServerFnError> {
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| crate::security::public_error("Upload inválido."))?
    {
        if field.name() != Some("file") && field.name() != Some("image") {
            continue;
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| crate::security::public_error("Não foi possível ler a imagem."))?
        {
            if bytes.len() + chunk.len() > crate::config::settings().asset_max_upload_bytes {
                return Err(crate::security::public_error(format!(
                    "A imagem excede {} MB.",
                    crate::config::settings().asset_max_upload_bytes / (1024 * 1024)
                )));
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(crate::security::public_error("Selecione uma imagem."));
        }
        return Ok(bytes);
    }
    Err(crate::security::public_error("Selecione uma imagem."))
}

async fn multipart_package_parts(
    mut multipart: Multipart,
) -> Result<(Vec<u8>, String), ServerFnError> {
    let mut bytes = None;
    let mut base_fingerprint = String::new();
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| crate::security::public_error("Upload de pacote inválido."))?
    {
        match field.name() {
            Some("file") | Some("package") => {
                let mut content = Vec::new();
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|_| crate::security::public_error("Não foi possível ler o pacote."))?
                {
                    if content.len() + chunk.len() > 128 * 1024 * 1024 {
                        return Err(crate::security::public_error(
                            "O pacote excede o limite permitido.",
                        ));
                    }
                    content.extend_from_slice(&chunk);
                }
                bytes = Some(content);
            }
            Some("baseFingerprint") => {
                base_fingerprint = field
                    .text()
                    .await
                    .map_err(|_| crate::security::public_error("Fingerprint inválido."))?;
            }
            _ => {}
        }
    }
    Ok((
        bytes.ok_or_else(|| crate::security::public_error("Selecione um pacote."))?,
        base_fingerprint,
    ))
}

async fn custom_event_cover_upload(
    Path(event_id): Path<String>,
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::assets::upload_cover(
            String::new(),
            event_id,
            multipart_bytes(multipart).await?,
            csrf_header(&headers),
        )
        .await?,
    ))
}

async fn custom_event_cover_remove(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::assets::remove_cover(String::new(), event_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn custom_event_option_upload(
    Path((event_id, _item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::assets::upload_option(
            String::new(),
            event_id,
            option_id,
            multipart_bytes(multipart).await?,
            csrf_header(&headers),
        )
        .await?,
    ))
}

async fn custom_event_option_remove(
    Path((event_id, _item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::assets::remove_option(String::new(), event_id, option_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn media_asset(Path((asset_id, variant)): Path<(String, String)>) -> ApiResult<Response> {
    if !crate::assets::can_read(&asset_id).await? {
        return Err(ApiError::from(crate::security::public_error(
            "Asset não encontrado.",
        )));
    }
    let (bytes, sha256) = crate::assets::read_variant(&asset_id, &variant).await?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "image/webp")
        .header("cache-control", "public, max-age=31536000, immutable")
        .header("etag", format!("\"{sha256}-{variant}\""))
        .body(Body::from(bytes))
        .map_err(|_| {
            ApiError::from(crate::security::public_error(
                "Não foi possível servir o asset.",
            ))
        })
}
async fn custom_event_delete_option(
    Path((id, item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::custom_events::delete_option(
        String::new(),
        id,
        item_id,
        option_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn custom_event_move_option(
    Path((id, item_id, option_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(body): Json<MoveBody>,
) -> ApiResult<StatusCode> {
    crate::custom_events::move_option(
        String::new(),
        id,
        item_id,
        option_id,
        body.direction,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn custom_event_publish(Path(id): Path<String>, headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::custom_events::publish(String::new(), id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn join_pool(
    headers: HeaderMap,
    Json(body): Json<JoinPoolBody>,
) -> ApiResult<impl IntoResponse> {
    let pool =
        crate::pools::join_pool(String::new(), body.invite_code, csrf_header(&headers)).await?;
    Ok(Json(pool))
}

async fn pool_member_predictions(Path(pool_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::get_pool_member_predictions(String::new(), pool_id).await?,
    ))
}

async fn react_to_prediction(
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

async fn mark_prediction_reactions_seen(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::pools::mark_prediction_reactions_seen(String::new(), pool_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn pool_breakdowns(Path(pool_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::scoring::list_pool_breakdowns(&pool_id).await?))
}

async fn list_pool_adjustments(Path(pool_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::list_pool_adjustments(String::new(), pool_id).await?,
    ))
}

async fn add_point_adjustment(
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

async fn remove_point_adjustment(
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

async fn delete_pool(Path(pool_id): Path<String>, headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::pools::delete_pool(String::new(), pool_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Handlers — admin: gestão de membros de bolões
// ---------------------------------------------------------------------------

async fn admin_list_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::list_all_pools_admin(String::new()).await?,
    ))
}

async fn admin_list_users() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::list_admin_users(String::new()).await?))
}

async fn admin_list_pool_members(Path(pool_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::list_pool_members_admin(String::new(), pool_id).await?,
    ))
}

async fn admin_add_pool_member(
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

async fn admin_remove_pool_member(
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

// ---------------------------------------------------------------------------
// Handlers — matches / predictions
// ---------------------------------------------------------------------------

async fn list_matches() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::matches::list_matches(String::new()).await?))
}

async fn knockout_released() -> ApiResult<impl IntoResponse> {
    let released = crate::matches::is_knockout_released().await?;
    Ok(Json(json!({ "released": released })))
}

async fn my_predictions() -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::matches::get_my_predictions(String::new()).await?,
    ))
}

async fn my_prediction_overrides() -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_my_prediction_overrides(String::new()).await?,
    ))
}

async fn my_match_points() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::scoring::list_my_match_points().await?))
}

async fn submit_prediction(
    headers: HeaderMap,
    Json(body): Json<PredictionBody>,
) -> ApiResult<StatusCode> {
    crate::matches::submit_prediction(
        String::new(),
        body.match_id,
        body.home_score,
        body.away_score,
        body.knockout,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn submit_single_choice_prediction(
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
async fn submit_numeric_prediction(
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
async fn submit_multiple_choice_prediction(
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
async fn set_multiple_choice_result(
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MultipleChoiceResultBody>,
) -> ApiResult<StatusCode> {
    crate::multiple_choice::set_result_authorized(
        String::new(),
        item_id,
        body.option_ids,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn custom_questions(
    Query(query): Query<CustomQuestionsQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::custom_questions::list_custom_questions(String::new(), query.pool_id).await?,
    ))
}

async fn custom_event_showcase(Query(query): Query<PoolIdQuery>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::custom_questions::event_showcase(String::new(), query.pool_id).await?,
    ))
}

async fn update_option_media_progress(
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

async fn custom_member_predictions(Path(pool_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::custom_questions::list_custom_member_predictions(String::new(), pool_id).await?,
    ))
}

async fn pool_football_scoring(Path(pool_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::football_scoring_config(String::new(), pool_id).await?,
    ))
}

async fn update_pool_football_scoring(
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

async fn pool_custom_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::custom_item_scoring_config(String::new(), pool_id, item_id).await?,
    ))
}
async fn update_pool_custom_scoring(
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

async fn set_custom_question_result(
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CustomResultBody>,
) -> ApiResult<StatusCode> {
    crate::custom_questions::set_correct_option_authorized(
        String::new(),
        item_id,
        body.option_id,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn set_numeric_question_result(
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<NumericResultBody>,
) -> ApiResult<StatusCode> {
    crate::numeric::set_result_authorized(
        String::new(),
        item_id,
        body.value,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn pool_numeric_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::numeric_item_scoring_config(String::new(), pool_id, item_id).await?,
    ))
}
async fn pool_multiple_choice_scoring(
    Path((pool_id, item_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::pools::multiple_choice_item_scoring_config(String::new(), pool_id, item_id).await?,
    ))
}
async fn update_pool_multiple_choice_scoring(
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
async fn update_pool_numeric_scoring(
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

// ---------------------------------------------------------------------------
// Handlers — admin
// ---------------------------------------------------------------------------

async fn set_match_result(
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

async fn set_knockout_released(
    headers: HeaderMap,
    Json(body): Json<KnockoutReleasedBody>,
) -> ApiResult<StatusCode> {
    crate::matches::set_knockout_released(String::new(), body.released, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_match_finished(
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

async fn update_match_teams(
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

async fn create_match(
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

async fn update_match_schedule(
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

async fn set_match_fixture(
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

async fn check_fixture(
    headers: HeaderMap,
    Json(body): Json<FixtureCheckBody>,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    let checked = crate::football::check_fixture_id(body.external_fixture_id).await?;
    Ok(Json(checked))
}

async fn delete_match(Path(match_id): Path<String>, headers: HeaderMap) -> ApiResult<StatusCode> {
    crate::matches::delete_match(String::new(), match_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_overview() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::admin_overview(String::new()).await?))
}

async fn admin_events() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::list_events_admin(String::new()).await?))
}

async fn admin_event_manifest_export(Path(event_id): Path<String>) -> ApiResult<Response> {
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

async fn custom_event_manifest_export(Path(event_id): Path<String>) -> ApiResult<Response> {
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

async fn admin_manifest_preview(
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

async fn admin_manifest_apply(
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

async fn admin_event_package_export(Path(event_id): Path<String>) -> ApiResult<Response> {
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

async fn custom_event_package_export(Path(event_id): Path<String>) -> ApiResult<Response> {
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

async fn admin_package_preview(
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let session = crate::auth::require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_header(&headers))?;
    let (bytes, _) = multipart_package_parts(multipart).await?;
    Ok(Json(crate::event_package::preview(&bytes).await?))
}

async fn admin_package_apply(
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

async fn admin_finish_event(
    Path(event_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::finish_event(String::new(), event_id, csrf_header(&headers)).await?,
    ))
}

async fn admin_matches(Query(query): Query<AdminMatchListQuery>) -> ApiResult<impl IntoResponse> {
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

async fn admin_match_audit(Path(match_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_match_audit(String::new(), match_id).await?,
    ))
}

async fn admin_sync_status() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::latest_sync_status().await?))
}

async fn admin_sync_run_now(headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::run_sync_now(String::new(), csrf_header(&headers)).await?,
    ))
}

async fn admin_sync_backfill(headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::run_backfill_now(String::new(), csrf_header(&headers)).await?,
    ))
}

async fn admin_predictions(
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

async fn admin_prediction_reopen(
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

async fn admin_prediction_reopen_revoke(
    headers: HeaderMap,
    Json(body): Json<RevokePredictionOverrideBody>,
) -> ApiResult<StatusCode> {
    crate::admin::revoke_prediction_reopen(String::new(), body.override_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_recalculate_match(
    headers: HeaderMap,
    Json(body): Json<RecalculateMatchBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::admin_recalculate_match(String::new(), body.match_id, csrf_header(&headers))
            .await?,
    ))
}

async fn admin_recalculate_all(headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::admin_recalculate_all(String::new(), csrf_header(&headers)).await?,
    ))
}

async fn admin_user_breakdown(
    Path(user_id): Path<String>,
    Query(query): Query<PoolIdQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::scoring::list_user_breakdowns(&user_id, &query.pool_id).await?,
    ))
}

async fn admin_user_pools(Path(user_id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::admin::list_user_pools(String::new(), user_id).await?,
    ))
}

async fn admin_block_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<BlockUserBody>,
) -> ApiResult<StatusCode> {
    crate::admin::block_user(String::new(), user_id, body.reason, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_unblock_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::unblock_user(String::new(), user_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_invalidate_user_sessions(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::invalidate_user_sessions_admin(String::new(), user_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_trigger_user_password_reset(
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::admin::trigger_user_password_reset(String::new(), user_id, csrf_header(&headers))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_send_push_to_user(
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

async fn admin_send_push_broadcast(
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

async fn admin_audit(Query(query): Query<AdminAuditQuery>) -> ApiResult<impl IntoResponse> {
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

async fn admin_get_settings() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::load_admin_settings().await?))
}

async fn public_settings() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::admin::load_admin_settings().await?))
}

async fn admin_save_settings(
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

async fn leaderboard(Query(query): Query<PoolIdQuery>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        crate::scoring::get_leaderboard(String::new(), query.pool_id).await?,
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/internal/metrics", get(internal_metrics))
        .route("/contact", get(contact_info))
        .route("/settings/public", get(public_settings))
        .route("/auth/register", post(register))
        .route("/auth/register/confirm", post(register_confirm))
        .route("/auth/password-reset", post(password_reset))
        .route("/auth/password-reset/confirm", post(password_reset_confirm))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/current-user", get(current_user))
        .route("/auth/reauth", post(reauth))
        .route("/auth/username", post(change_username))
        .route("/auth/delete", post(delete_account))
        .route("/auth/csrf", get(csrf))
        .route("/notifications/status", get(notification_status))
        .route(
            "/notifications/preferences",
            post(update_notification_preference_handler),
        )
        .route(
            "/notifications/subscriptions",
            post(upsert_push_subscription_handler),
        )
        .route(
            "/notifications/subscriptions/remove",
            post(remove_push_subscription_handler),
        )
        .route("/pools", get(list_pools).post(create_pool))
        .route("/pools/dashboard", get(dashboard_pools))
        .route("/custom/events/mine", get(custom_events_mine))
        .route("/custom/events/available", get(custom_events_available))
        .route("/custom/events", post(custom_event_create))
        .route("/custom/events/{id}", get(custom_event_get))
        .route("/custom/events/{id}/draft", get(custom_event_draft))
        .route("/custom/events/{id}/update", post(custom_event_update))
        .route(
            "/custom/events/{id}/manifest",
            get(custom_event_manifest_export),
        )
        .route(
            "/custom/events/{id}/package",
            get(custom_event_package_export),
        )
        .route("/custom/events/{id}/cover", post(custom_event_cover_upload))
        .route(
            "/custom/events/{id}/cover/remove",
            post(custom_event_cover_remove),
        )
        .route("/custom/events/{id}/delete", post(custom_event_delete))
        .route("/custom/events/{id}/items", post(custom_event_add_item))
        .route(
            "/custom/events/{id}/items/numeric",
            post(custom_event_add_numeric_item),
        )
        .route(
            "/custom/events/{id}/items/multiple-choice",
            post(custom_event_add_multiple_choice_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/update",
            post(custom_event_update_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/delete",
            post(custom_event_delete_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/move",
            post(custom_event_move_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options",
            post(custom_event_add_option),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/update",
            post(custom_event_update_option),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/media",
            post(custom_event_update_option_media),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/image",
            post(custom_event_option_upload),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/image/remove",
            post(custom_event_option_remove),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/delete",
            post(custom_event_delete_option),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/move",
            post(custom_event_move_option),
        )
        .route("/custom/events/{id}/publish", post(custom_event_publish))
        .route("/pools/join", post(join_pool))
        .route(
            "/pools/{pool_id}/member-predictions",
            get(pool_member_predictions),
        )
        .route(
            "/pools/{pool_id}/prediction-reactions",
            post(react_to_prediction),
        )
        .route(
            "/pools/{pool_id}/prediction-reactions/mark-seen",
            post(mark_prediction_reactions_seen),
        )
        .route("/pools/{pool_id}/breakdowns", get(pool_breakdowns))
        .route(
            "/pools/{pool_id}/adjustments",
            get(list_pool_adjustments).post(add_point_adjustment),
        )
        .route(
            "/pools/{pool_id}/adjustments/remove",
            post(remove_point_adjustment),
        )
        .route("/pools/{pool_id}/delete", post(delete_pool))
        .route("/matches", get(list_matches))
        .route("/matches/knockout-released", get(knockout_released))
        .route("/predictions", get(my_predictions).post(submit_prediction))
        .route("/custom/questions", get(custom_questions))
        .route("/custom/event-showcase", get(custom_event_showcase))
        .route("/custom/media-progress", post(update_option_media_progress))
        .route(
            "/pools/{pool_id}/custom-member-predictions",
            get(custom_member_predictions),
        )
        .route("/custom/predictions", post(submit_single_choice_prediction))
        .route(
            "/custom/numeric-predictions",
            post(submit_numeric_prediction),
        )
        .route(
            "/custom/multiple-choice-predictions",
            post(submit_multiple_choice_prediction),
        )
        .route(
            "/admin/custom/questions/{item_id}/result",
            post(set_custom_question_result),
        )
        .route(
            "/admin/custom/numeric/{item_id}/result",
            post(set_numeric_question_result),
        )
        .route(
            "/admin/custom/multiple-choice/{item_id}/result",
            post(set_multiple_choice_result),
        )
        .route(
            "/pools/{pool_id}/scoring/football",
            get(pool_football_scoring).post(update_pool_football_scoring),
        )
        .route(
            "/pools/{pool_id}/scoring/items/{item_id}",
            get(pool_custom_scoring).post(update_pool_custom_scoring),
        )
        .route(
            "/pools/{pool_id}/scoring/numeric/{item_id}",
            get(pool_numeric_scoring).post(update_pool_numeric_scoring),
        )
        .route(
            "/pools/{pool_id}/scoring/multiple-choice/{item_id}",
            get(pool_multiple_choice_scoring).post(update_pool_multiple_choice_scoring),
        )
        .route("/predictions/reopened", get(my_prediction_overrides))
        .route("/scoring/my-points", get(my_match_points))
        .route("/admin/overview", get(admin_overview))
        .route("/admin/events", get(admin_events))
        .route(
            "/admin/events/{event_id}/manifest",
            get(admin_event_manifest_export),
        )
        .route(
            "/admin/events/{event_id}/package",
            get(admin_event_package_export),
        )
        .route("/admin/events/import/preview", post(admin_manifest_preview))
        .route("/admin/events/import/apply", post(admin_manifest_apply))
        .route(
            "/admin/events/import/package/preview",
            post(admin_package_preview),
        )
        .route(
            "/admin/events/import/package/apply",
            post(admin_package_apply),
        )
        .route("/admin/events/{event_id}/finish", post(admin_finish_event))
        .route("/admin/matches", get(admin_matches).post(create_match))
        .route("/admin/matches/{id}/audit", get(admin_match_audit))
        .route("/admin/matches/{id}/result", post(set_match_result))
        .route("/admin/matches/{id}/finished", post(set_match_finished))
        .route("/admin/matches/{id}/schedule", post(update_match_schedule))
        .route("/admin/matches/{id}/fixture", post(set_match_fixture))
        .route("/admin/fixtures/check", post(check_fixture))
        .route("/admin/matches/{id}/delete", post(delete_match))
        .route("/admin/knockout-released", post(set_knockout_released))
        .route("/admin/matches/{id}/teams", post(update_match_teams))
        .route("/admin/sync/status", get(admin_sync_status))
        .route("/admin/sync/run-now", post(admin_sync_run_now))
        .route("/admin/sync/backfill", post(admin_sync_backfill))
        .route("/admin/predictions", get(admin_predictions))
        .route("/admin/predictions/reopen", post(admin_prediction_reopen))
        .route(
            "/admin/predictions/reopen/revoke",
            post(admin_prediction_reopen_revoke),
        )
        .route(
            "/admin/scoring/recalculate-match",
            post(admin_recalculate_match),
        )
        .route(
            "/admin/scoring/recalculate-all",
            post(admin_recalculate_all),
        )
        .route(
            "/admin/scoring/users/{id}/breakdown",
            get(admin_user_breakdown),
        )
        .route("/admin/pools", get(admin_list_pools))
        .route("/admin/users", get(admin_list_users))
        .route("/admin/users/{id}/pools", get(admin_user_pools))
        .route("/admin/users/{id}/block", post(admin_block_user))
        .route("/admin/users/{id}/unblock", post(admin_unblock_user))
        .route(
            "/admin/users/{id}/invalidate-sessions",
            post(admin_invalidate_user_sessions),
        )
        .route(
            "/admin/users/{id}/password-reset",
            post(admin_trigger_user_password_reset),
        )
        .route("/admin/users/{id}/push", post(admin_send_push_to_user))
        .route("/admin/push/broadcast", post(admin_send_push_broadcast))
        .route(
            "/admin/pools/{pool_id}/members",
            get(admin_list_pool_members).post(admin_add_pool_member),
        )
        .route(
            "/admin/pools/{pool_id}/members/remove",
            post(admin_remove_pool_member),
        )
        .route("/admin/audit", get(admin_audit))
        .route(
            "/admin/settings",
            get(admin_get_settings).post(admin_save_settings),
        )
        .route("/leaderboard", get(leaderboard))
        .fallback(api_not_found)
}

async fn api_not_found() -> ApiError {
    ApiError {
        status: StatusCode::NOT_FOUND,
        message: "Rota de API não encontrada.".to_string(),
    }
}
