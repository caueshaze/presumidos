use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::{
    config::settings,
    error::ServerFnError,
    security::{log_event, public_error},
};

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
#[cfg(feature = "server")]
pub fn rate_limit_identity_hash(value: &str) -> String {
    use hmac::{Hmac, Mac};

    let mut mac =
        Hmac::<sha2::Sha256>::new_from_slice(settings().rate_limit_identity_secret.as_bytes())
            .expect("HMAC aceita chaves de qualquer tamanho");
    mac.update(value.trim().to_lowercase().as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(feature = "server")]
pub fn sensitive_value_hash(value: &str) -> String {
    rate_limit_identity_hash(value)
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
        object.insert(
            "key".to_string(),
            serde_json::Value::String(key.to_string()),
        );
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
            object.insert(
                "attempts".to_string(),
                serde_json::Value::Number(count.into()),
            );
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
