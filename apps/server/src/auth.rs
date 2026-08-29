use crate::error::ServerFnError;

use crate::models::{AuthResult, SessionState, UserPublic};

#[path = "auth_impl.rs"]
mod auth_impl;
pub(crate) use auth_impl::*;

#[cfg(feature = "server")]
use axum::http::HeaderMap;

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub token: String,
    pub user_id: String,
    pub csrf_token: String,
    pub admin_reauthed_at: Option<String>,
}

#[cfg(feature = "server")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AuthCleanupSummary {
    pub expired_sessions_deleted: u64,
    pub expired_pending_registrations_deleted: u64,
    pub expired_password_reset_codes_deleted: u64,
}
