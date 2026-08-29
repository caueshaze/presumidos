//! Fachada HTTP/JSON da API REST.
//!
//! A composição de rotas e os contratos permanecem estáveis; middleware,
//! DTOs e adaptações HTTP vivem em módulos internos por responsabilidade.

#![cfg(feature = "server")]

mod adapters;
mod admin_routes;
mod auth_routes;
mod custom_event_routes;
mod dto;
mod middleware;
mod pool_routes;
mod prediction_routes;
mod routes;

pub use adapters::{health_live, health_ready, internal_metrics};
pub use middleware::context_middleware;
pub use routes::router;

pub(crate) use adapters::*;
use admin_routes::*;
pub(crate) use auth_routes::*;
pub(crate) use custom_event_routes::*;
pub(crate) use dto::*;
use pool_routes::*;
use prediction_routes::*;

// Compatibilidade para os adaptadores existentes que usam `use super::*`.
pub(crate) use crate::error::ServerFnError;
pub(crate) use crate::models::{AdminSettings, FootballScoringConfig};
pub(crate) use axum::{
    body::Body,
    extract::{Multipart, Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
pub(crate) use serde_json::json;
