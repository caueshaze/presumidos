//! Adaptadores HTTP compartilhados: erros, CSRF, health e fallbacks.

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::error::ServerFnError;

pub(crate) struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
}

impl From<ServerFnError> for ApiError {
    fn from(error: ServerFnError) -> Self {
        let message = error.message().to_string();
        let status = if message == "SECURITY:ADMIN_REAUTH_REQUIRED"
            || message.starts_with("Falha de seguranca da sessao")
        {
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
        Self { status, message }
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
                json!({ "error_id": error_id, "status": self.status.as_u16(), "message": self.message }),
            );
            (
                self.status,
                Json(json!({ "error": "Ocorreu um erro interno.", "errorId": error_id })),
            )
                .into_response()
        } else {
            (self.status, Json(json!({ "error": self.message }))).into_response()
        }
    }
}

pub(crate) type ApiResult<T> = Result<T, ApiError>;

pub(crate) fn csrf_header(headers: &HeaderMap) -> String {
    headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
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
            json!({ "database": report.database, "migrations": report.migrations, "assets": report.assets, "disk": report.disk }),
        );
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable" })),
        )
    }
}

pub(crate) async fn health() -> impl IntoResponse {
    health_live().await
}

pub async fn internal_metrics(headers: HeaderMap) -> impl IntoResponse {
    if !crate::config::settings().metrics_enabled
        || (crate::config::settings().app_env == "production"
            && crate::security::enforce_trusted_proxy(&headers).is_err())
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

pub(crate) async fn api_not_found() -> ApiError {
    ApiError {
        status: StatusCode::NOT_FOUND,
        message: "Rota de API não encontrada.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_errors_hide_internal_messages() {
        let response = ApiError::from(crate::security::internal_error(
            "test",
            std::io::Error::other("secret"),
        ))
        .into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn csrf_errors_map_to_forbidden() {
        assert_eq!(
            ApiError::from(crate::security::public_error(
                "Falha de seguranca da sessao: csrf"
            ))
            .status,
            StatusCode::FORBIDDEN
        );
    }
}
