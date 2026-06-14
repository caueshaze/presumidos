//! Camada HTTP/JSON (Axum) que expõe a lógica de negócio como uma API REST estável.
//!
//! Substitui a camada RPC do Dioxus (`#[server]`). Cada handler é fino: extrai corpo/headers,
//! chama a função de negócio correspondente (que lê a sessão pelo cookie e escreve headers de
//! resposta via [crate::context]) e serializa o resultado em JSON. Erros viram
//! `{ "error": "..." }` com o status HTTP apropriado.

#![cfg(feature = "server")]

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query, Request},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::context::{take_response_headers, RequestContext, REQUEST};
use crate::error::ServerFnError;
use crate::models::KnockoutEntry;

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
        } else {
            StatusCode::BAD_REQUEST
        };
        ApiError { status, message }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
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
    let ctx = RequestContext::new(headers, Some(peer));

    let (mut response, extra_headers) = REQUEST
        .scope(ctx, async move {
            // Headers de segurança em toda resposta (inclusive estáticos).
            crate::security::apply_security_headers();
            let response = next.run(request).await;
            (response, take_response_headers())
        })
        .await;

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
struct CreatePoolBody {
    name: String,
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
struct PoolIdQuery {
    pool_id: String,
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

async fn csrf() -> ApiResult<impl IntoResponse> {
    let state = crate::auth::current_user(String::new()).await?;
    Ok(Json(json!({ "csrfToken": state.csrf_token })))
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Handlers — pools
// ---------------------------------------------------------------------------

async fn list_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::pools::list_my_pools(String::new()).await?))
}

async fn create_pool(
    headers: HeaderMap,
    Json(body): Json<CreatePoolBody>,
) -> ApiResult<impl IntoResponse> {
    let pool = crate::pools::create_pool(String::new(), body.name, csrf_header(&headers)).await?;
    Ok(Json(pool))
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

async fn delete_pool(
    Path(pool_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    crate::pools::delete_pool(String::new(), pool_id, csrf_header(&headers)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Handlers — admin: gestão de membros de bolões
// ---------------------------------------------------------------------------

async fn admin_list_pools() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::pools::list_all_pools_admin(String::new()).await?))
}

async fn admin_list_users() -> ApiResult<impl IntoResponse> {
    Ok(Json(crate::auth::list_all_users(String::new()).await?))
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
    crate::pools::add_pool_member_admin(String::new(), pool_id, body.user_id, csrf_header(&headers))
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

// ---------------------------------------------------------------------------
// Handlers — admin
// ---------------------------------------------------------------------------

async fn set_match_result(
    Path(match_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MatchResultBody>,
) -> ApiResult<StatusCode> {
    crate::matches::set_match_result(
        String::new(),
        match_id,
        body.home_score,
        body.away_score,
        body.knockout,
        csrf_header(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
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
    crate::matches::set_match_finished(String::new(), match_id, body.finished, csrf_header(&headers))
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
        .route("/auth/register", post(register))
        .route("/auth/register/confirm", post(register_confirm))
        .route("/auth/password-reset", post(password_reset))
        .route("/auth/password-reset/confirm", post(password_reset_confirm))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/current-user", get(current_user))
        .route("/auth/reauth", post(reauth))
        .route("/auth/username", post(change_username))
        .route("/auth/csrf", get(csrf))
        .route("/pools", get(list_pools).post(create_pool))
        .route("/pools/join", post(join_pool))
        .route("/pools/{pool_id}/member-predictions", get(pool_member_predictions))
        .route(
            "/pools/{pool_id}/adjustments",
            get(list_pool_adjustments).post(add_point_adjustment),
        )
        .route("/pools/{pool_id}/adjustments/remove", post(remove_point_adjustment))
        .route("/pools/{pool_id}/delete", post(delete_pool))
        .route("/matches", get(list_matches))
        .route("/matches/knockout-released", get(knockout_released))
        .route("/predictions", get(my_predictions).post(submit_prediction))
        .route("/admin/matches/{id}/result", post(set_match_result))
        .route("/admin/matches/{id}/finished", post(set_match_finished))
        .route("/admin/knockout-released", post(set_knockout_released))
        .route("/admin/matches/{id}/teams", post(update_match_teams))
        .route("/admin/pools", get(admin_list_pools))
        .route("/admin/users", get(admin_list_users))
        .route(
            "/admin/pools/{pool_id}/members",
            get(admin_list_pool_members).post(admin_add_pool_member),
        )
        .route("/admin/pools/{pool_id}/members/remove", post(admin_remove_pool_member))
        .route("/leaderboard", get(leaderboard))
        .fallback(api_not_found)
}

async fn api_not_found() -> ApiError {
    ApiError {
        status: StatusCode::NOT_FOUND,
        message: "Rota de API não encontrada.".to_string(),
    }
}
