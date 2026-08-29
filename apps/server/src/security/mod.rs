#[cfg(feature = "server")]
mod audit;
#[cfg(feature = "server")]
mod cookies;
#[cfg(feature = "server")]
mod csrf;
#[cfg(feature = "server")]
mod headers;
#[cfg(feature = "server")]
mod rate_limit;
#[cfg(feature = "server")]
mod validation;

#[cfg(feature = "server")]
pub use audit::{append_audit_log, internal_error, log_event, public_error};
#[cfg(feature = "server")]
pub use cookies::{
    clear_session_cookie, current_headers, parse_cookie, session_cookie_name, set_response_header,
    set_session_cookie,
};
#[cfg(feature = "server")]
pub use csrf::{csrf_token, hash_code, require_csrf, verification_code};
#[cfg(feature = "server")]
pub use headers::{apply_security_headers, client_ip, current_peer_ip, enforce_trusted_proxy};
#[cfg(feature = "server")]
pub use rate_limit::{
    enforce_rate_limit, rate_limit_identity_hash, sensitive_value_hash, RateLimitFailurePolicy,
    RateLimitRequest, RateLimitRule,
};
#[cfg(feature = "server")]
pub use validation::{
    normalize_email, normalize_optional_text, normalize_required_text, validate_match_id,
    validate_uuid,
};
