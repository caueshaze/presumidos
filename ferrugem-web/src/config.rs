#[cfg(feature = "server")]
use std::sync::OnceLock;

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitBackendKind {
    Memory,
    Redis,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub database_path: String,
    pub session_secret: String,
    pub admin_bootstrap_secret: String,
    pub session_ttl_hours: i64,
    pub cookie_secure: bool,
    pub admin_reauth_ttl_minutes: i64,
    pub trusted_proxy_cidrs: Vec<ipnet::IpNet>,
    pub require_trusted_proxy: bool,
    pub resend_api_key: String,
    pub resend_from_email: String,
    pub rate_limit_backend: RateLimitBackendKind,
    pub redis_url: Option<String>,
    pub rate_limit_identity_secret: String,
    pub argon2_memory_kib: u32,
    pub argon2_time_cost: u32,
    pub argon2_parallelism: u32,
    pub argon2_policy_version: String,
    pub football: FootballConfig,
}

/// Configuração da integração de resultados ao vivo (API worldcup26.ir).
/// Tudo é opcional: se `enabled` for false, o poller nunca sobe. A API é
/// pública (sem chave) e gratuita, então não há cota/segredo aqui.
#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct FootballConfig {
    /// Liga a integração (sync + leitura). Sem isso, nada de chamadas externas.
    pub enabled: bool,
    /// Sobe o poller em background nesta instância. Mantenha `true` em apenas
    /// uma réplica para não duplicar requisições à API pública.
    pub poller_enabled: bool,
    pub base_url: String,
    pub poll_interval_secs: u64,
}

#[cfg(feature = "server")]
static CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[cfg(feature = "server")]
fn required_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("variavel {name} ausente no .env"))
}

#[cfg(feature = "server")]
fn parse_bool_var(name: &str) -> bool {
    match required_var(name).trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => panic!("variavel {name} deve ser booleana"),
    }
}

#[cfg(feature = "server")]
fn parse_i64_var(name: &str) -> i64 {
    required_var(name)
        .trim()
        .parse::<i64>()
        .unwrap_or_else(|_| panic!("variavel {name} deve ser numerica"))
}

#[cfg(feature = "server")]
fn parse_cidr_list_var(name: &str) -> Vec<ipnet::IpNet> {
    required_var(name)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<ipnet::IpNet>()
                .unwrap_or_else(|_| panic!("variavel {name} contem CIDR invalido: {value}"))
        })
        .collect()
}

#[cfg(feature = "server")]
fn optional_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(feature = "server")]
fn optional_u32_var(name: &str, default: u32) -> u32 {
    match optional_var(name) {
        Some(value) => value
            .parse::<u32>()
            .unwrap_or_else(|_| panic!("variavel {name} deve ser numerica")),
        None => default,
    }
}

#[cfg(feature = "server")]
fn optional_u64_var(name: &str, default: u64) -> u64 {
    match optional_var(name) {
        Some(value) => value
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("variavel {name} deve ser numerica")),
        None => default,
    }
}

#[cfg(feature = "server")]
fn optional_bool_var(name: &str, default: bool) -> bool {
    match optional_var(name) {
        Some(value) => match value.trim().to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => panic!("variavel {name} deve ser booleana"),
        },
        None => default,
    }
}

#[cfg(feature = "server")]
fn parse_rate_limit_backend_var(name: &str) -> RateLimitBackendKind {
    match required_var(name).trim().to_lowercase().as_str() {
        "memory" => RateLimitBackendKind::Memory,
        "redis" => RateLimitBackendKind::Redis,
        _ => panic!("variavel {name} deve ser 'memory' ou 'redis'"),
    }
}

#[cfg(feature = "server")]
fn has_global_cidr(cidrs: &[ipnet::IpNet]) -> bool {
    cidrs.iter().any(|cidr| match cidr {
        ipnet::IpNet::V4(net) => net.prefix_len() == 0,
        ipnet::IpNet::V6(net) => net.prefix_len() == 0,
    })
}

#[cfg(feature = "server")]
fn validate_proxy_config(
    app_env: &str,
    trusted_proxy_cidrs: &[ipnet::IpNet],
    require_trusted_proxy: bool,
) {
    if app_env == "production" {
        assert!(
            !has_global_cidr(trusted_proxy_cidrs),
            "TRUSTED_PROXY_CIDRS nao pode conter 0.0.0.0/0 nem ::/0 em producao"
        );
        if require_trusted_proxy {
            assert!(
                !trusted_proxy_cidrs.is_empty(),
                "TRUSTED_PROXY_CIDRS precisa ser configurado quando REQUIRE_TRUSTED_PROXY=true"
            );
        }
    }
}

#[cfg(feature = "server")]
fn validate_rate_limit_config(
    app_env: &str,
    rate_limit_backend: RateLimitBackendKind,
    redis_url: Option<&str>,
    rate_limit_identity_secret_provided: bool,
) {
    match rate_limit_backend {
        RateLimitBackendKind::Memory => {}
        RateLimitBackendKind::Redis => {
            let redis_url = redis_url
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| panic!("REDIS_URL precisa ser configurado quando RATE_LIMIT_BACKEND=redis"));
            assert!(
                redis_url.starts_with("redis://") || redis_url.starts_with("rediss://"),
                "REDIS_URL precisa usar esquema redis:// ou rediss://"
            );
        }
    }

    if app_env == "production" {
        assert!(
            rate_limit_backend == RateLimitBackendKind::Redis,
            "RATE_LIMIT_BACKEND deve ser redis em producao"
        );
        assert!(
            rate_limit_identity_secret_provided,
            "RATE_LIMIT_IDENTITY_SECRET precisa ser configurado explicitamente em producao"
        );
    }
}

#[cfg(feature = "server")]
pub fn settings() -> &'static AppConfig {
    CONFIG.get_or_init(|| {
        let _ = dotenvy::dotenv();

        let app_env = required_var("APP_ENV").trim().to_lowercase();
        let database_path = required_var("DATABASE_PATH");
        let session_secret = required_var("SESSION_SECRET");
        let admin_bootstrap_secret = required_var("ADMIN_BOOTSTRAP_SECRET");
        let session_ttl_hours = parse_i64_var("SESSION_TTL_HOURS");
        let cookie_secure = parse_bool_var("COOKIE_SECURE");
        let admin_reauth_ttl_minutes = parse_i64_var("ADMIN_REAUTH_TTL_MINUTES");
        let trusted_proxy_cidrs = parse_cidr_list_var("TRUSTED_PROXY_CIDRS");
        let require_trusted_proxy = parse_bool_var("REQUIRE_TRUSTED_PROXY");
        let resend_api_key = required_var("RESEND_API_KEY");
        let resend_from_email = required_var("RESEND_FROM_EMAIL");
        let rate_limit_backend = parse_rate_limit_backend_var("RATE_LIMIT_BACKEND");
        let redis_url = optional_var("REDIS_URL");
        let rate_limit_identity_secret_var = optional_var("RATE_LIMIT_IDENTITY_SECRET");
        let rate_limit_identity_secret = rate_limit_identity_secret_var
            .clone()
            .unwrap_or_else(|| session_secret.clone());
        let argon2_memory_kib = optional_u32_var("ARGON2_MEMORY_KIB", 19456);
        let argon2_time_cost = optional_u32_var("ARGON2_TIME_COST", 2);
        let argon2_parallelism = optional_u32_var("ARGON2_PARALLELISM", 1);
        let argon2_policy_version =
            optional_var("ARGON2_POLICY_VERSION").unwrap_or_else(|| "v1".to_string());

        let football_enabled = optional_bool_var("FOOTBALL_API_ENABLED", false);
        let football = FootballConfig {
            enabled: football_enabled,
            poller_enabled: optional_bool_var("FOOTBALL_POLLER_ENABLED", false),
            base_url: optional_var("FOOTBALL_API_BASE_URL")
                .unwrap_or_else(|| "https://worldcup26.ir".to_string()),
            poll_interval_secs: optional_u64_var("FOOTBALL_POLL_INTERVAL_SECS", 900),
        };
        if football_enabled {
            assert!(
                football.poll_interval_secs >= 60,
                "FOOTBALL_POLL_INTERVAL_SECS deve ser >= 60"
            );
        }

        assert!(
            !resend_api_key.trim().is_empty(),
            "RESEND_API_KEY nao pode ser vazio"
        );
        assert!(
            resend_from_email.contains('@'),
            "RESEND_FROM_EMAIL precisa ser um remetente valido"
        );
        assert!(
            session_secret.trim().len() >= 32,
            "SESSION_SECRET precisa ter pelo menos 32 caracteres"
        );
        assert!(
            rate_limit_identity_secret.trim().len() >= 32,
            "RATE_LIMIT_IDENTITY_SECRET precisa ter pelo menos 32 caracteres"
        );
        assert!(session_ttl_hours > 0, "SESSION_TTL_HOURS deve ser > 0");
        assert!(
            admin_reauth_ttl_minutes > 0,
            "ADMIN_REAUTH_TTL_MINUTES deve ser > 0"
        );
        assert!(
            argon2_memory_kib >= 19456,
            "ARGON2_MEMORY_KIB deve ser >= 19456"
        );
        assert!(argon2_time_cost >= 2, "ARGON2_TIME_COST deve ser >= 2");
        assert!(
            argon2_parallelism >= 1,
            "ARGON2_PARALLELISM deve ser >= 1"
        );

        if app_env == "production" {
            assert!(
                cookie_secure,
                "COOKIE_SECURE precisa estar habilitado em producao"
            );
        }
        validate_proxy_config(&app_env, &trusted_proxy_cidrs, require_trusted_proxy);
        validate_rate_limit_config(
            &app_env,
            rate_limit_backend,
            redis_url.as_deref(),
            rate_limit_identity_secret_var.is_some(),
        );

        AppConfig {
            app_env,
            database_path,
            session_secret,
            admin_bootstrap_secret,
            session_ttl_hours,
            cookie_secure,
            admin_reauth_ttl_minutes,
            trusted_proxy_cidrs,
            require_trusted_proxy,
            resend_api_key,
            resend_from_email,
            rate_limit_backend,
            redis_url,
            rate_limit_identity_secret,
            argon2_memory_kib,
            argon2_time_cost,
            argon2_parallelism,
            argon2_policy_version,
            football,
        }
    })
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{has_global_cidr, validate_proxy_config, validate_rate_limit_config, RateLimitBackendKind};

    #[test]
    fn detects_global_proxy_cidrs() {
        let cidrs = vec![
            "0.0.0.0/0".parse().expect("ipv4 cidr"),
            "::/0".parse().expect("ipv6 cidr"),
        ];
        assert!(has_global_cidr(&cidrs));
        assert!(!has_global_cidr(&["10.0.0.0/8".parse().expect("cidr")]));
    }

    #[test]
    fn production_proxy_validation_rejects_global_cidrs() {
        let result = std::panic::catch_unwind(|| {
            validate_proxy_config("production", &["0.0.0.0/0".parse().expect("cidr")], true);
        });
        assert!(result.is_err());
    }

    #[test]
    fn production_proxy_validation_requires_cidrs_when_boundary_enabled() {
        let result = std::panic::catch_unwind(|| {
            validate_proxy_config("production", &[], true);
        });
        assert!(result.is_err());
    }

    #[test]
    fn production_rate_limit_requires_redis_backend() {
        let result = std::panic::catch_unwind(|| {
            validate_rate_limit_config("production", RateLimitBackendKind::Memory, None, true);
        });
        assert!(result.is_err());
    }

    #[test]
    fn redis_backend_requires_valid_url() {
        let missing = std::panic::catch_unwind(|| {
            validate_rate_limit_config("development", RateLimitBackendKind::Redis, None, true);
        });
        assert!(missing.is_err());

        let invalid = std::panic::catch_unwind(|| {
            validate_rate_limit_config(
                "development",
                RateLimitBackendKind::Redis,
                Some("http://localhost:6379"),
                true,
            );
        });
        assert!(invalid.is_err());
    }

    #[test]
    fn production_requires_explicit_rate_limit_identity_secret() {
        let missing = std::panic::catch_unwind(|| {
            validate_rate_limit_config(
                "production",
                RateLimitBackendKind::Redis,
                Some("redis://redis:6379"),
                false,
            );
        });
        assert!(missing.is_err());

        let provided = std::panic::catch_unwind(|| {
            validate_rate_limit_config(
                "production",
                RateLimitBackendKind::Redis,
                Some("redis://redis:6379"),
                true,
            );
        });
        assert!(provided.is_ok());
    }
}
