use crate::error::ServerFnError;

#[cfg(feature = "server")]
use std::net::IpAddr;
#[cfg(feature = "server")]
use sha2::Digest;
#[cfg(feature = "server")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "server")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(feature = "server")]
use std::time::{Duration, Instant};

#[cfg(feature = "server")]
use crate::config::settings;

#[cfg(feature = "server")]
use axum::http::HeaderMap;

#[cfg(feature = "server")]
#[derive(Clone, Copy)]
pub struct RateLimitRule {
    pub window: Duration,
    pub max_attempts: usize,
}

#[cfg(feature = "server")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RateLimitFailurePolicy {
    FailOpen,
    FailClosed,
}

#[cfg(feature = "server")]
pub struct RateLimitRequest {
    pub key: String,
    pub rule: RateLimitRule,
    pub blocked_event: &'static str,
    pub failure_policy: RateLimitFailurePolicy,
    pub audit_fields: serde_json::Value,
}

#[cfg(feature = "server")]
#[derive(Default)]
struct RateLimiter {
    buckets: HashMap<String, VecDeque<Instant>>,
}

#[cfg(feature = "server")]
enum RateLimitBackend {
    Memory(Arc<Mutex<RateLimiter>>),
    Redis(redis::Client),
}

#[cfg(feature = "server")]
static RATE_LIMIT_BACKEND: OnceLock<RateLimitBackend> = OnceLock::new();

#[cfg(feature = "server")]
fn rate_limit_backend() -> &'static RateLimitBackend {
    RATE_LIMIT_BACKEND.get_or_init(|| match settings().rate_limit_backend {
        crate::config::RateLimitBackendKind::Memory => {
            RateLimitBackend::Memory(Arc::new(Mutex::new(RateLimiter::default())))
        }
        crate::config::RateLimitBackendKind::Redis => {
            let client = redis::Client::open(
                settings()
                    .redis_url
                    .clone()
                    .expect("REDIS_URL precisa estar presente para o backend redis"),
            )
            .expect("falha ao inicializar cliente Redis para rate limit");
            RateLimitBackend::Redis(client)
        }
    })
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
pub fn rate_limit_identity_hash(value: &str) -> String {
    use hmac::{Hmac, Mac};

    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(settings().rate_limit_identity_secret.as_bytes())
        .expect("HMAC aceita chaves de qualquer tamanho");
    mac.update(value.trim().to_lowercase().as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(feature = "server")]
fn parse_ip_token(raw: &str) -> Option<IpAddr> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('[').trim_matches(']');
    let candidate = trimmed
        .split(':')
        .next()
        .filter(|_| trimmed.matches(':').count() < 2)
        .unwrap_or(trimmed);

    candidate.parse::<IpAddr>().ok().or_else(|| trimmed.parse::<IpAddr>().ok())
}

#[cfg(feature = "server")]
fn parse_forwarded_for_ip(raw: &str) -> Option<IpAddr> {
    raw.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix("for=")
            .and_then(parse_ip_token)
            .or_else(|| part.strip_prefix("For=").and_then(parse_ip_token))
    })
}

#[cfg(feature = "server")]
fn header_ip(headers: &HeaderMap, key: &str) -> Option<IpAddr> {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_token)
}

#[cfg(feature = "server")]
fn forwarded_chain(headers: &HeaderMap) -> Vec<IpAddr> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        let parsed: Vec<IpAddr> = value.split(',').filter_map(parse_ip_token).collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').filter_map(parse_forwarded_for_ip).collect())
        .unwrap_or_default()
}

#[cfg(feature = "server")]
fn is_trusted_proxy(ip: IpAddr, trusted_proxy_cidrs: &[ipnet::IpNet]) -> bool {
    trusted_proxy_cidrs.iter().any(|cidr| cidr.contains(&ip))
}

#[cfg(feature = "server")]
fn resolve_client_ip_from_peer_and_headers(
    peer_ip: Option<IpAddr>,
    headers: &HeaderMap,
    trusted_proxy_cidrs: &[ipnet::IpNet],
) -> Option<IpAddr> {
    let peer_ip = peer_ip?;
    if trusted_proxy_cidrs.is_empty() || !is_trusted_proxy(peer_ip, trusted_proxy_cidrs) {
        return Some(peer_ip);
    }

    let chain = forwarded_chain(headers);
    if !chain.is_empty() {
        for ip in chain.into_iter().rev() {
            if !is_trusted_proxy(ip, trusted_proxy_cidrs) {
                return Some(ip);
            }
        }
    }

    header_ip(headers, "cf-connecting-ip")
        .or_else(|| header_ip(headers, "x-real-ip"))
        .or(Some(peer_ip))
}

#[cfg(feature = "server")]
fn proxy_boundary_allowed(
    peer_ip: Option<IpAddr>,
    trusted_proxy_cidrs: &[ipnet::IpNet],
    require_trusted_proxy: bool,
) -> bool {
    if !require_trusted_proxy {
        return true;
    }

    peer_ip.is_some_and(|peer_ip| is_trusted_proxy(peer_ip, trusted_proxy_cidrs))
}

#[cfg(feature = "server")]
pub fn current_peer_ip() -> Option<IpAddr> {
    crate::context::peer_ip()
}

#[cfg(feature = "server")]
pub fn client_ip(headers: &HeaderMap) -> String {
    resolve_client_ip_from_peer_and_headers(
        current_peer_ip(),
        headers,
        &settings().trusted_proxy_cidrs,
    )
    .map(|ip| ip.to_string())
    .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(feature = "server")]
pub fn enforce_trusted_proxy(headers: &HeaderMap) -> Result<(), ServerFnError> {
    let peer_ip = current_peer_ip();
    if proxy_boundary_allowed(
        peer_ip,
        &settings().trusted_proxy_cidrs,
        settings().require_trusted_proxy,
    ) {
        return Ok(());
    }

    let Some(peer_ip) = peer_ip else {
        log_event(
            "proxy_boundary_blocked",
            serde_json::json!({
                "reason": "missing_connect_info",
            }),
        );
        return Err(public_error(
            "Este ambiente exige acesso pelo proxy configurado.",
        ));
    };

    if !is_trusted_proxy(peer_ip, &settings().trusted_proxy_cidrs) {
        log_event(
            "proxy_boundary_blocked",
            serde_json::json!({
                "reason": "untrusted_peer",
                "peer_ip": peer_ip.to_string(),
                "client_ip": resolve_client_ip_from_peer_and_headers(
                    Some(peer_ip),
                    headers,
                    &settings().trusted_proxy_cidrs,
                )
                .map(|ip| ip.to_string()),
            }),
        );
        return Err(public_error(
            "Este ambiente exige acesso pelo proxy configurado.",
        ));
    }

    Ok(())
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

/// Valida um identificador de partida. Os IDs de partidas vêm do seed (ex.: `jogo-001`),
/// não são UUIDs — então aceitamos um token curto de [A-Za-z0-9_-]. A existência real é
/// verificada na consulta seguinte ("Partida nao encontrada.").
#[cfg(feature = "server")]
pub fn validate_match_id(value: &str) -> Result<(), ServerFnError> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(public_error("Partida invalida."));
    }
    Ok(())
}

#[cfg(feature = "server")]
fn rate_limit_backend_name() -> &'static str {
    match settings().rate_limit_backend {
        crate::config::RateLimitBackendKind::Memory => "memory",
        crate::config::RateLimitBackendKind::Redis => "redis",
    }
}

#[cfg(feature = "server")]
fn rate_limit_error(policy: RateLimitFailurePolicy) -> Result<(), ServerFnError> {
    match policy {
        RateLimitFailurePolicy::FailOpen => Ok(()),
        RateLimitFailurePolicy::FailClosed => Err(public_error(
            "Nao foi possivel validar limite de acesso agora. Tente novamente em instantes.",
        )),
    }
}

#[cfg(feature = "server")]
fn enrich_rate_limit_fields(
    fields: &serde_json::Value,
    key: &str,
    rule: RateLimitRule,
) -> serde_json::Value {
    let mut fields = fields.clone();
    if let Some(object) = fields.as_object_mut() {
        object.insert("key".to_string(), serde_json::Value::String(key.to_string()));
        object.insert(
            "window_secs".to_string(),
            serde_json::Value::Number(rule.window.as_secs().into()),
        );
        object.insert(
            "max_attempts".to_string(),
            serde_json::Value::Number(rule.max_attempts.into()),
        );
        object.insert(
            "backend".to_string(),
            serde_json::Value::String(rate_limit_backend_name().to_string()),
        );
    }
    fields
}

#[cfg(feature = "server")]
fn log_rate_limit_backend_unavailable(
    key: &str,
    rule: RateLimitRule,
    policy: RateLimitFailurePolicy,
    fields: &serde_json::Value,
    error: &redis::RedisError,
) {
    let mut fields = enrich_rate_limit_fields(fields, key, rule);
    if let Some(object) = fields.as_object_mut() {
        object.insert(
            "failure_policy".to_string(),
            serde_json::Value::String(match policy {
                RateLimitFailurePolicy::FailOpen => "fail_open".to_string(),
                RateLimitFailurePolicy::FailClosed => "fail_closed".to_string(),
            }),
        );
        object.insert(
            "error".to_string(),
            serde_json::Value::String(error.to_string()),
        );
    }
    log_event("rate_limit_backend_unavailable", fields);
}

#[cfg(feature = "server")]
fn memory_enforce_rate_limit(
    limiter: &Arc<Mutex<RateLimiter>>,
    key: &str,
    rule: RateLimitRule,
    blocked_event: &str,
    audit_fields: &serde_json::Value,
) -> Result<(), ServerFnError> {
    let now = Instant::now();
    let mut guard = limiter
        .lock()
        .map_err(|_| public_error("Nao foi possivel validar limite de acesso."))?;
    let attempts = guard.buckets.entry(key.to_string()).or_default();

    while attempts
        .front()
        .is_some_and(|instant| now.duration_since(*instant) > rule.window)
    {
        attempts.pop_front();
    }

    if attempts.len() >= rule.max_attempts {
        let mut fields = enrich_rate_limit_fields(audit_fields, key, rule);
        if let Some(object) = fields.as_object_mut() {
            object.insert(
                "attempts".to_string(),
                serde_json::Value::Number((attempts.len() as u64).into()),
            );
        }
        log_event(blocked_event, fields);
        return Err(public_error(
            "Muitas tentativas em pouco tempo. Aguarde um pouco e tente novamente.",
        ));
    }

    attempts.push_back(now);
    Ok(())
}

/// Incrementa o contador e garante um TTL na mesma chamada ao Redis: evita que
/// a chave fique sem expiracao caso o processo morra entre o INCR e o EXPIRE.
#[cfg(feature = "server")]
const RATE_LIMIT_INCR_SCRIPT: &str = r#"
local count = redis.call('INCR', KEYS[1])
if redis.call('TTL', KEYS[1]) < 0 then
    redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return count
"#;

#[cfg(feature = "server")]
async fn redis_enforce_rate_limit(
    client: &redis::Client,
    key: &str,
    rule: RateLimitRule,
    blocked_event: &str,
    failure_policy: RateLimitFailurePolicy,
    audit_fields: &serde_json::Value,
) -> Result<(), ServerFnError> {
    let mut connection = match client.get_multiplexed_async_connection().await {
        Ok(connection) => connection,
        Err(error) => {
            log_rate_limit_backend_unavailable(key, rule, failure_policy, audit_fields, &error);
            return rate_limit_error(failure_policy);
        }
    };

    let count: i64 = match redis::Script::new(RATE_LIMIT_INCR_SCRIPT)
        .key(key)
        .arg(rule.window.as_secs() as i64)
        .invoke_async(&mut connection)
        .await
    {
        Ok(count) => count,
        Err(error) => {
            log_rate_limit_backend_unavailable(key, rule, failure_policy, audit_fields, &error);
            return rate_limit_error(failure_policy);
        }
    };

    if count > rule.max_attempts as i64 {
        let mut fields = enrich_rate_limit_fields(audit_fields, key, rule);
        if let Some(object) = fields.as_object_mut() {
            object.insert("attempts".to_string(), serde_json::Value::Number(count.into()));
        }
        log_event(blocked_event, fields);
        return Err(public_error(
            "Muitas tentativas em pouco tempo. Aguarde um pouco e tente novamente.",
        ));
    }

    Ok(())
}

#[cfg(feature = "server")]
pub async fn enforce_rate_limit(request: RateLimitRequest) -> Result<(), ServerFnError> {
    match rate_limit_backend() {
        RateLimitBackend::Memory(limiter) => memory_enforce_rate_limit(
            limiter,
            &request.key,
            request.rule,
            request.blocked_event,
            &request.audit_fields,
        ),
        RateLimitBackend::Redis(client) => {
            redis_enforce_rate_limit(
                client,
                &request.key,
                request.rule,
                request.blocked_event,
                request.failure_policy,
                &request.audit_fields,
            )
            .await
        }
    }
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
    crate::context::push_response_header(name, value);
}

#[cfg(feature = "server")]
pub fn current_headers() -> HeaderMap {
    crate::context::request_headers()
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
    set_response_header(
        "content-security-policy",
        // SPA React servida estaticamente: scripts e estilos próprios, fontes do Google,
        // e fetch da API no mesmo host. Sem 'unsafe-inline'/'wasm-unsafe-eval' (não há mais
        // SSR/WASM do Dioxus). 'style-src' mantém 'unsafe-inline' para estilos utilitários
        // injetados em runtime (Tailwind/shadcn) e variáveis de tema.
        "default-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; script-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".to_string(),
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

/// Gera um codigo numerico de 6 digitos para verificacao por email.
#[cfg(feature = "server")]
pub fn verification_code() -> String {
    use rand_core::{OsRng, RngCore};
    format!("{:06}", OsRng.next_u32() % 1_000_000)
}

/// Hash de um codigo de verificacao para armazenamento (nunca guardar em texto puro).
#[cfg(feature = "server")]
pub fn hash_code(code: &str) -> String {
    let seed = format!("{}:{}", settings().session_secret, code.trim());
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

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{
        forwarded_chain, memory_enforce_rate_limit, parse_forwarded_for_ip, parse_ip_token,
        proxy_boundary_allowed, rate_limit_identity_hash, redis_enforce_rate_limit,
        resolve_client_ip_from_peer_and_headers, RateLimitFailurePolicy, RateLimitRule,
        RateLimiter,
    };
    use axum::http::HeaderMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::net::{IpAddr, Ipv4Addr};

    fn trusted_cidrs() -> Vec<ipnet::IpNet> {
        vec![
            "10.0.0.0/8".parse().expect("cidr"),
            "127.0.0.0/8".parse().expect("cidr"),
        ]
    }

    fn seed_security_env() {
        std::env::set_var("APP_ENV", "test");
        std::env::set_var("DATABASE_PATH", "test.db");
        std::env::set_var(
            "SESSION_SECRET",
            "0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        std::env::set_var(
            "ADMIN_BOOTSTRAP_SECRET",
            "bootstrap-secret-super-seguro-0123456789abcdef",
        );
        std::env::set_var("SESSION_TTL_HOURS", "12");
        std::env::set_var("COOKIE_SECURE", "false");
        std::env::set_var("ADMIN_REAUTH_TTL_MINUTES", "10");
        std::env::set_var("TRUSTED_PROXY_CIDRS", "");
        std::env::set_var("REQUIRE_TRUSTED_PROXY", "false");
        std::env::set_var("RESEND_API_KEY", "test-key");
        std::env::set_var("RESEND_FROM_EMAIL", "teste@presumidos.dev");
        std::env::set_var("RATE_LIMIT_BACKEND", "memory");
        std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                name.parse::<axum::http::header::HeaderName>()
                    .expect("header name"),
                value.parse().expect("header value"),
            );
        }
        headers
    }

    #[test]
    fn parses_ip_tokens_and_forwarded_values() {
        assert_eq!(
            parse_ip_token("203.0.113.5"),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)))
        );
        assert_eq!(
            parse_forwarded_for_ip("for=203.0.113.5;proto=https"),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)))
        );
        assert_eq!(
            parse_forwarded_for_ip("for=\"[2001:db8::1]\""),
            "2001:db8::1".parse().ok()
        );
    }

    #[test]
    fn ignores_spoofed_x_forwarded_for_without_trusted_proxy_peer() {
        let peer_ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7));
        let headers = headers(&[("x-forwarded-for", "1.2.3.4, 5.6.7.8")]);

        let client_ip =
            resolve_client_ip_from_peer_and_headers(Some(peer_ip), &headers, &trusted_cidrs())
                .unwrap();

        assert_eq!(client_ip, peer_ip);
    }

    #[test]
    fn resolves_client_from_rightmost_non_trusted_forwarded_ip() {
        let peer = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10)));
        let headers = headers(&[(
            "x-forwarded-for",
            "1.2.3.4, 203.0.113.77, 10.0.0.10",
        )]);

        let client_ip =
            resolve_client_ip_from_peer_and_headers(peer, &headers, &trusted_cidrs()).unwrap();

        assert_eq!(client_ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 77)));
        assert_eq!(forwarded_chain(&headers).len(), 3);
    }

    #[test]
    fn falls_back_to_single_proxy_headers_when_proxy_is_trusted() {
        let peer = Some(IpAddr::V4(Ipv4Addr::new(10, 2, 0, 5)));
        let headers = headers(&[("cf-connecting-ip", "198.51.100.42")]);

        let client_ip =
            resolve_client_ip_from_peer_and_headers(peer, &headers, &trusted_cidrs()).unwrap();

        assert_eq!(client_ip, IpAddr::V4(Ipv4Addr::new(198, 51, 100, 42)));
    }

    #[test]
    fn proxy_boundary_requires_trusted_peer_when_enabled() {
        assert!(!proxy_boundary_allowed(None, &trusted_cidrs(), true));
        assert!(!proxy_boundary_allowed(
            Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7))),
            &trusted_cidrs(),
            true,
        ));
        assert!(proxy_boundary_allowed(
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10))),
            &trusted_cidrs(),
            true,
        ));
    }

    fn rate_limit_fields(extra: serde_json::Value) -> serde_json::Value {
        let mut fields = serde_json::Map::new();
        fields.insert("scope".to_string(), serde_json::Value::String("test".to_string()));
        if let Some(object) = extra.as_object() {
            for (key, value) in object {
                fields.insert(key.clone(), value.clone());
            }
        }
        serde_json::Value::Object(fields)
    }

    #[test]
    fn memory_rate_limit_blocks_after_threshold_without_cross_key_leak() {
        seed_security_env();
        let limiter = Arc::new(Mutex::new(RateLimiter::default()));
        let rule = RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 2,
        };

        assert!(memory_enforce_rate_limit(
            &limiter,
            "rl:login:ip:1.2.3.4",
            rule,
            "rate_limit_triggered_login_ip",
            &rate_limit_fields(serde_json::json!({"client_ip": "1.2.3.4"})),
        )
        .is_ok());
        assert!(memory_enforce_rate_limit(
            &limiter,
            "rl:login:ip:1.2.3.4",
            rule,
            "rate_limit_triggered_login_ip",
            &rate_limit_fields(serde_json::json!({"client_ip": "1.2.3.4"})),
        )
        .is_ok());
        assert!(memory_enforce_rate_limit(
            &limiter,
            "rl:login:ip:1.2.3.4",
            rule,
            "rate_limit_triggered_login_ip",
            &rate_limit_fields(serde_json::json!({"client_ip": "1.2.3.4"})),
        )
        .is_err());

        assert!(memory_enforce_rate_limit(
            &limiter,
            "rl:login:ip:5.6.7.8",
            rule,
            "rate_limit_triggered_login_ip",
            &rate_limit_fields(serde_json::json!({"client_ip": "5.6.7.8"})),
        )
        .is_ok());
    }

    #[test]
    fn identity_hash_does_not_echo_raw_login() {
        seed_security_env();
        let hash = rate_limit_identity_hash("admin@presumidos.dev");
        assert_ne!(hash, "admin@presumidos.dev");
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn redis_rate_limit_failure_policy_is_respected() {
        seed_security_env();
        let client = redis::Client::open("redis://127.0.0.1:1").expect("redis client");
        let rule = RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 1,
        };

        let fail_open = redis_enforce_rate_limit(
            &client,
            "rl:login:ip:203.0.113.5",
            rule,
            "rate_limit_triggered_login_ip",
            RateLimitFailurePolicy::FailOpen,
            &rate_limit_fields(serde_json::json!({"client_ip": "203.0.113.5"})),
        )
        .await;
        assert!(fail_open.is_ok());

        let fail_closed = redis_enforce_rate_limit(
            &client,
            "rl:login:ip:203.0.113.6",
            rule,
            "rate_limit_triggered_login_ip",
            RateLimitFailurePolicy::FailClosed,
            &rate_limit_fields(serde_json::json!({"client_ip": "203.0.113.6"})),
        )
        .await;
        assert!(fail_closed.is_err());
    }

    #[tokio::test]
    async fn redis_rate_limit_persists_across_clients() {
        seed_security_env();
        let Some(redis_url) = std::env::var("REDIS_TEST_URL").ok() else {
            return;
        };
        let rule = RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 2,
        };

        let client_a = redis::Client::open(redis_url.clone()).expect("client a");
        let client_b = redis::Client::open(redis_url).expect("client b");
        let key = format!("rl:login:identity:test-{}", uuid::Uuid::new_v4());

        assert!(redis_enforce_rate_limit(
            &client_a,
            &key,
            rule,
            "rate_limit_triggered_login_identity",
            RateLimitFailurePolicy::FailClosed,
            &rate_limit_fields(serde_json::json!({"identity_hash": "abc123"})),
        )
        .await
        .is_ok());
        assert!(redis_enforce_rate_limit(
            &client_b,
            &key,
            rule,
            "rate_limit_triggered_login_identity",
            RateLimitFailurePolicy::FailClosed,
            &rate_limit_fields(serde_json::json!({"identity_hash": "abc123"})),
        )
        .await
        .is_ok());
        assert!(redis_enforce_rate_limit(
            &client_b,
            &key,
            rule,
            "rate_limit_triggered_login_identity",
            RateLimitFailurePolicy::FailClosed,
            &rate_limit_fields(serde_json::json!({"identity_hash": "abc123"})),
        )
        .await
        .is_err());
    }
}
