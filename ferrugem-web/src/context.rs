//! Contexto de requisição com escopo de task.
//!
//! Substitui o `FullstackContext` do Dioxus. Um middleware Axum ([crate::api::context_middleware])
//! popula este task-local com os headers da requisição e o IP do peer, e fornece um acumulador de
//! headers de resposta. A lógica de negócio (em [crate::security]) lê/escreve aqui em vez de acessar
//! o contexto ambiente do Dioxus, então as funções de negócio permanecem inalteradas.

#![cfg(feature = "server")]

use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;

use axum::http::{HeaderMap, HeaderName, HeaderValue};

pub struct RequestContext {
    pub headers: HeaderMap,
    pub peer: Option<SocketAddr>,
    pub response_headers: Mutex<Vec<(HeaderName, HeaderValue)>>,
}

impl RequestContext {
    pub fn new(headers: HeaderMap, peer: Option<SocketAddr>) -> Self {
        Self {
            headers,
            peer,
            response_headers: Mutex::new(Vec::new()),
        }
    }
}

tokio::task_local! {
    pub static REQUEST: RequestContext;
}

/// Headers da requisição atual (vazio fora de um escopo de requisição).
pub fn request_headers() -> HeaderMap {
    REQUEST.try_with(|ctx| ctx.headers.clone()).unwrap_or_default()
}

/// IP do peer TCP (antes de qualquer resolução de proxy confiável).
pub fn peer_ip() -> Option<IpAddr> {
    REQUEST
        .try_with(|ctx| ctx.peer.map(|addr| addr.ip()))
        .ok()
        .flatten()
}

/// Enfileira um header para ser anexado à resposta.
pub fn push_response_header(name: &str, value: String) {
    let _ = REQUEST.try_with(|ctx| {
        if let (Ok(header_name), Ok(header_value)) =
            (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
        {
            ctx.response_headers
                .lock()
                .expect("response_headers mutex")
                .push((header_name, header_value));
        }
    });
}

/// Drena os headers de resposta acumulados durante a requisição.
pub fn take_response_headers() -> Vec<(HeaderName, HeaderValue)> {
    REQUEST
        .try_with(|ctx| {
            std::mem::take(&mut *ctx.response_headers.lock().expect("response_headers mutex"))
        })
        .unwrap_or_default()
}
