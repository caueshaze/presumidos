//! Middleware de contexto, observabilidade e headers de resposta.

use std::{net::SocketAddr, time::Instant};

use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::FutureExt;
use serde_json::json;
use uuid::Uuid;

use crate::context::{take_response_headers, RequestContext, REQUEST};

pub(crate) fn redacted_route(path: &str) -> String {
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
    let scoped = REQUEST.scope(
        RequestContext::new(headers, Some(peer), request_id.clone()),
        async move {
            crate::security::apply_security_headers();
            let response = next.run(request).await;
            (response, take_response_headers())
        },
    );
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
                        Json(json!({ "error": "Ocorreu um erro interno.", "errorId": error_id })),
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
        json!({ "request_id": request_id, "method": method, "route": log_path, "status": response.status().as_u16(), "duration_ms": started.elapsed().as_secs_f64() * 1000.0 }),
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
    for (name, value) in extra_headers {
        if name == axum::http::header::SET_COOKIE {
            response.headers_mut().append(name, value);
        } else {
            response.headers_mut().insert(name, value);
        }
    }
    response
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
