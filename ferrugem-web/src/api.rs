//! Camada HTTP/JSON (Axum) que expõe a lógica de negócio como uma API REST estável.
//!
//! Substitui a camada RPC do Dioxus (`#[server]`). Cada handler é fino: extrai corpo/headers,
//! chama a função de negócio correspondente (que lê a sessão pelo cookie e escreve headers de
//! resposta via [crate::context]) e serializa o resultado em JSON. Erros viram
//! `{ "error": "..." }` com o status HTTP apropriado.

#![cfg(feature = "server")]

mod admin_routes;
mod auth_routes;
mod custom_event_routes;
mod pool_routes;
mod prediction_routes;
mod routes;
use admin_routes::*;
pub(crate) use auth_routes::*;
pub(crate) use custom_event_routes::*;
use pool_routes::*;
use prediction_routes::*;

pub use routes::router;

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

fn redacted_route(path: &str) -> String {
    for prefix in [
        "/api/public/pools/invite/",
        "/api/pools/invite/",
        "/pools/join/",
    ] {
        if path.starts_with(prefix) {
            return format!("{prefix}[redacted]");
        }
    }
    path.to_string()
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
    let log_path = redacted_route(&path);
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
                    json!({ "error_id": error_id, "route": log_path }),
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
            "route": log_path,
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
pub(crate) struct RegisterBody {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub(crate) struct RegisterConfirmBody {
    email: String,
    code: String,
}

#[derive(Deserialize)]
pub(crate) struct PasswordResetBody {
    email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PasswordResetConfirmBody {
    email: String,
    code: String,
    new_password: String,
}

#[derive(Deserialize)]
pub(crate) struct LoginBody {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub(crate) struct ChangeUsernameBody {
    username: String,
}

#[derive(Deserialize)]
pub(crate) struct ReauthBody {
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
struct EventAvailabilityBody {
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinPoolBody {
    invite_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PredictionBody {
    pool_id: String,
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
    #[serde(default)]
    pool_id: Option<String>,
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
    #[serde(default)]
    pool_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResultNotRepresentableBody {
    reason: String,
    #[serde(default)]
    pool_id: Option<String>,
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
#[serde(rename_all = "camelCase")]
struct NumericResultBody {
    value: String,
    #[serde(default)]
    pool_id: Option<String>,
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
struct PoolReportBody {
    category: String,
    details: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolReportStatusBody {
    status: String,
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
struct PoolReportQuery {
    status: Option<String>,
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
pub(crate) struct NotificationPreferenceBody {
    enabled: bool,
    lead_time_minutes: i64,
    reaction_enabled: bool,
}

#[derive(Deserialize)]
pub(crate) struct SubscriptionRemoveBody {
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

async fn api_not_found() -> ApiError {
    ApiError {
        status: StatusCode::NOT_FOUND,
        message: "Rota de API não encontrada.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::redacted_route;

    #[test]
    fn invite_tokens_are_not_written_to_route_logs() {
        assert_eq!(
            redacted_route("/api/public/pools/invite/SECRET123"),
            "/api/public/pools/invite/[redacted]"
        );
        assert_eq!(
            redacted_route("/pools/join/SECRET123"),
            "/pools/join/[redacted]"
        );
        assert_eq!(redacted_route("/api/pools/join"), "/api/pools/join");
    }
}
