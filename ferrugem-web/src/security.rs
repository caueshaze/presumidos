use dioxus::prelude::ServerFnError;

#[cfg(feature = "server")]
use sha2::Digest;
#[cfg(feature = "server")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "server")]
use std::sync::{Mutex, OnceLock};
#[cfg(feature = "server")]
use std::time::{Duration, Instant};

#[cfg(feature = "server")]
use crate::config::settings;

#[cfg(feature = "server")]
type HeaderMap = dioxus::prelude::dioxus_fullstack::HeaderMap;

#[cfg(feature = "server")]
#[derive(Clone, Copy)]
pub struct RateLimitRule {
    pub window: Duration,
    pub max_attempts: usize,
}

#[cfg(feature = "server")]
#[derive(Default)]
struct RateLimiter {
    buckets: HashMap<String, VecDeque<Instant>>,
}

#[cfg(feature = "server")]
static RATE_LIMITER: OnceLock<Mutex<RateLimiter>> = OnceLock::new();

#[cfg(feature = "server")]
fn limiter() -> &'static Mutex<RateLimiter> {
    RATE_LIMITER.get_or_init(|| Mutex::new(RateLimiter::default()))
}

#[cfg(feature = "server")]
pub fn log_event(kind: &str, details: serde_json::Value) {
    let line = serde_json::json!({
        "kind": kind,
        "at": chrono::Utc::now().to_rfc3339(),
        "details": details,
    });
    eprintln!("{line}");
}

#[cfg(feature = "server")]
pub fn public_error(message: impl Into<String>) -> ServerFnError {
    ServerFnError::new(message.into())
}

#[cfg(feature = "server")]
pub fn internal_error(context: &str, error: impl std::fmt::Display) -> ServerFnError {
    log_event(
        "internal_error",
        serde_json::json!({
            "context": context,
            "error": error.to_string(),
        }),
    );
    public_error("O servidor nao conseguiu concluir essa operacao agora.")
}

#[cfg(feature = "server")]
pub fn client_ip(headers: &HeaderMap) -> String {
    for key in ["x-forwarded-for", "x-real-ip", "cf-connecting-ip"] {
        if let Some(value) = headers.get(key).and_then(|v| v.to_str().ok()) {
            let ip = value.split(',').next().unwrap_or("").trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    "unknown".to_string()
}

#[cfg(feature = "server")]
pub fn normalize_required_text(
    field: &str,
    value: String,
    min_len: usize,
    max_len: usize,
) -> Result<String, ServerFnError> {
    let value = value.trim().to_string();
    if value.len() < min_len {
        return Err(public_error(format!("{field} muito curto.")));
    }
    if value.len() > max_len {
        return Err(public_error(format!("{field} muito longo.")));
    }
    Ok(value)
}

#[cfg(feature = "server")]
pub fn normalize_optional_text(value: String, max_len: usize) -> Result<String, ServerFnError> {
    let value = value.trim().to_string();
    if value.len() > max_len {
        return Err(public_error("Texto acima do tamanho permitido."));
    }
    Ok(value)
}

#[cfg(feature = "server")]
pub fn normalize_email(email: String) -> Result<String, ServerFnError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || email.len() > 120 || !email.contains('@') {
        return Err(public_error("Email invalido."));
    }
    Ok(email)
}

#[cfg(feature = "server")]
pub fn validate_uuid(field: &str, value: &str) -> Result<(), ServerFnError> {
    uuid::Uuid::parse_str(value).map_err(|_| public_error(format!("{field} invalido.")))?;
    Ok(())
}

#[cfg(feature = "server")]
pub fn enforce_rate_limit(
    scope: &str,
    key: &str,
    rule: RateLimitRule,
) -> Result<(), ServerFnError> {
    let bucket_key = format!("{scope}:{key}");
    let now = Instant::now();
    let mut guard = limiter()
        .lock()
        .map_err(|_| public_error("Nao foi possivel validar limite de acesso."))?;
    let attempts = guard.buckets.entry(bucket_key).or_default();

    while attempts
        .front()
        .is_some_and(|instant| now.duration_since(*instant) > rule.window)
    {
        attempts.pop_front();
    }

    if attempts.len() >= rule.max_attempts {
        log_event(
            "rate_limit_blocked",
            serde_json::json!({
                "scope": scope,
                "key": key,
                "attempts": attempts.len(),
            }),
        );
        return Err(public_error(
            "Muitas tentativas em pouco tempo. Aguarde um pouco e tente novamente.",
        ));
    }

    attempts.push_back(now);
    Ok(())
}

#[cfg(feature = "server")]
pub fn session_cookie_name() -> &'static str {
    "presumidos_session"
}

#[cfg(feature = "server")]
pub fn parse_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| {
            raw.split(';').find_map(|part| {
                let mut pieces = part.trim().splitn(2, '=');
                let key = pieces.next()?.trim();
                let value = pieces.next()?.trim();
                (key == name && !value.is_empty()).then(|| value.to_string())
            })
        })
}

#[cfg(feature = "server")]
pub fn set_response_header(name: &'static str, value: String) {
    if let Some(ctx) = dioxus::prelude::dioxus_fullstack::FullstackContext::current() {
        if let (Ok(header_name), Ok(header_value)) = (
            name.parse::<dioxus::prelude::dioxus_fullstack::http::header::HeaderName>(),
            value.parse::<dioxus::prelude::dioxus_fullstack::HeaderValue>(),
        ) {
            ctx.add_response_header(header_name, header_value);
        }
    }
}

#[cfg(feature = "server")]
pub fn current_headers() -> HeaderMap {
    dioxus::prelude::dioxus_fullstack::FullstackContext::current()
        .map(|ctx| ctx.parts_mut().headers.clone())
        .unwrap_or_default()
}

#[cfg(feature = "server")]
pub fn set_session_cookie(token: &str) {
    let max_age = settings().session_ttl_hours * 60 * 60;
    let secure = if settings().cookie_secure {
        "; Secure"
    } else {
        ""
    };
    set_response_header(
        "Set-Cookie",
        format!(
            "{}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}",
            session_cookie_name()
        ),
    );
}

#[cfg(feature = "server")]
pub fn clear_session_cookie() {
    let secure = if settings().cookie_secure {
        "; Secure"
    } else {
        ""
    };
    set_response_header(
        "Set-Cookie",
        format!(
            "{}=deleted; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
            session_cookie_name(),
            secure,
        ),
    );
}

#[cfg(feature = "server")]
pub fn apply_security_headers() {
    if dioxus::prelude::dioxus_fullstack::FullstackContext::current().is_some() {
        set_response_header(
            "content-security-policy",
            "default-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".to_string(),
        );
        set_response_header(
            "referrer-policy",
            "strict-origin-when-cross-origin".to_string(),
        );
        set_response_header("x-content-type-options", "nosniff".to_string());
        set_response_header("x-frame-options", "DENY".to_string());
        if settings().app_env == "production" && settings().cookie_secure {
            set_response_header(
                "strict-transport-security",
                "max-age=31536000; includeSubDomains".to_string(),
            );
        }
    }
}

#[cfg(feature = "server")]
pub fn csrf_token() -> String {
    let seed = format!(
        "{}:{}:{}",
        settings().session_secret,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        uuid::Uuid::new_v4()
    );
    let digest = sha2::Sha256::digest(seed.as_bytes());
    hex::encode(digest)
}

#[cfg(feature = "server")]
pub fn require_csrf(expected: &str, provided: &str) -> Result<(), ServerFnError> {
    if expected.is_empty() || provided.trim().is_empty() || expected != provided.trim() {
        return Err(public_error("Falha de seguranca da sessao. Atualize a pagina e tente novamente."));
    }
    Ok(())
}

#[cfg(feature = "server")]
pub async fn append_audit_log(
    db: &sqlx::SqlitePool,
    actor_user_id: Option<&str>,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    ip_address: Option<&str>,
    details: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip_address)
    .bind(details.to_string())
    .execute(db)
    .await
    .map_err(|e| internal_error("append_audit_log", e))?;

    Ok(())
}
