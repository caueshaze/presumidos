#[cfg(feature = "server")]
use std::path::Path;
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
    pub backup_dir: String,
    pub public_base_url: Option<String>,
    pub listen_address: String,
    pub shutdown_timeout_secs: u64,
    pub min_free_bytes: u64,
    pub database_busy_timeout_ms: u64,
    pub max_body_bytes: usize,
    pub json_logs: bool,
    pub metrics_enabled: bool,
    pub contact_email: Option<String>,
    pub session_secret: String,
    pub admin_bootstrap_secret: String,
    pub session_ttl_hours: i64,
    pub cookie_secure: bool,
    pub admin_reauth_ttl_minutes: i64,
    pub trusted_proxy_cidrs: Vec<ipnet::IpNet>,
    pub require_trusted_proxy: bool,
    pub resend_api_key: String,
    pub resend_from_email: String,
    pub disable_auth_emails: bool,
    pub rate_limit_backend: RateLimitBackendKind,
    pub redis_url: Option<String>,
    pub rate_limit_identity_secret: String,
    pub argon2_memory_kib: u32,
    pub argon2_time_cost: u32,
    pub argon2_parallelism: u32,
    pub argon2_policy_version: String,
    pub football: FootballConfig,
    pub web_push: WebPushConfig,
    pub asset_dir: String,
    pub asset_max_upload_bytes: usize,
    pub asset_max_pixels: u64,
}

/// Configuração da integração de resultados ao vivo via provedor público de placares.
/// Tudo é opcional: se `enabled` for false, o poller nunca sobe. A API é pública
/// (sem chave), então não há cota/segredo aqui.
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
    /// Intervalo (menor) usado enquanto há jogo na janela, para a pontuação ao
    /// vivo andar mais rápido. Fora de jogo, usa `poll_interval_secs`.
    pub live_poll_interval_secs: u64,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct WebPushConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    pub vapid_public_key: Option<String>,
    pub vapid_private_key: Option<String>,
    pub contact_email: Option<String>,
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
fn default_asset_dir(database_path: &str) -> String {
    let database_path = Path::new(database_path);
    if database_path.is_absolute() {
        database_path
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .join("data/assets")
            .to_string_lossy()
            .into_owned()
    } else {
        "./data/assets".to_string()
    }
}

#[cfg(feature = "server")]
fn parse_environment(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "development" | "test" | "production" => Ok(normalized),
        _ => Err("APP_ENV deve ser development, test ou production".to_string()),
    }
}

#[cfg(feature = "server")]
fn validate_secret(name: &str, value: &str) {
    assert!(
        value.trim().len() >= 32,
        "{name} precisa ter pelo menos 32 caracteres"
    );
    assert!(
        !value.to_lowercase().contains("troque-este")
            && !value.to_lowercase().contains("change-me")
            && !value.to_lowercase().contains("example"),
        "{name} nao pode usar um valor de exemplo"
    );
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
                .unwrap_or_else(|| {
                    panic!("REDIS_URL precisa ser configurado quando RATE_LIMIT_BACKEND=redis")
                });
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
fn validate_auth_email_config(
    app_env: &str,
    disable_auth_emails: bool,
    resend_api_key: Option<&str>,
    resend_from_email: Option<&str>,
) {
    if disable_auth_emails {
        assert!(
            app_env == "development" || app_env == "test",
            "DEV_DISABLE_AUTH_EMAILS so pode ser habilitado em development ou test"
        );
        return;
    }

    let resend_api_key = match resend_api_key {
        Some(value) if !value.trim().is_empty() => value,
        _ => panic!("RESEND_API_KEY precisa ser configurado quando emails estao ativos"),
    };
    assert!(
        !resend_api_key.trim().is_empty(),
        "RESEND_API_KEY nao pode ser vazio"
    );

    let resend_from_email = match resend_from_email {
        Some(value) if !value.trim().is_empty() => value,
        _ => panic!("RESEND_FROM_EMAIL precisa ser configurado quando emails estao ativos"),
    };
    assert!(
        resend_from_email.contains('@'),
        "RESEND_FROM_EMAIL precisa ser um remetente valido"
    );
}

#[cfg(feature = "server")]
pub fn settings() -> &'static AppConfig {
    CONFIG.get_or_init(|| {
        let _ = dotenvy::dotenv();

        let app_env =
            parse_environment(&required_var("APP_ENV")).unwrap_or_else(|error| panic!("{error}"));
        let database_path = required_var("DATABASE_PATH");
        let backup_dir =
            optional_var("PRESUMIDOS_BACKUP_DIR").unwrap_or_else(|| "./backups".to_string());
        let public_base_url = optional_var("PUBLIC_BASE_URL");
        let listen_address = optional_var("LISTEN_ADDRESS").unwrap_or_else(|| {
            let ip = optional_var("IP").unwrap_or_else(|| "127.0.0.1".to_string());
            let port = optional_var("PORT").unwrap_or_else(|| "8080".to_string());
            format!("{ip}:{port}")
        });
        let shutdown_timeout_secs = optional_u64_var("SHUTDOWN_TIMEOUT_SECS", 30);
        let min_free_bytes = optional_u64_var("PRESUMIDOS_MIN_FREE_BYTES", 100 * 1024 * 1024);
        let database_busy_timeout_ms =
            optional_u64_var("PRESUMIDOS_DATABASE_BUSY_TIMEOUT_MS", 5_000);
        let max_body_bytes = optional_u64_var("PRESUMIDOS_MAX_BODY_BYTES", 128 * 1024 * 1024)
            .try_into()
            .unwrap_or_else(|_| panic!("PRESUMIDOS_MAX_BODY_BYTES excede o limite suportado"));
        let json_logs = optional_bool_var("PRESUMIDOS_JSON_LOGS", false);
        let metrics_enabled = optional_bool_var("PRESUMIDOS_METRICS_ENABLED", false);
        let contact_email = optional_var("CONTACT_EMAIL")
            .or_else(|| optional_var("VITE_CONTACT_EMAIL"))
            .or_else(|| optional_var("WEB_PUSH_CONTACT_EMAIL"));
        let session_secret = required_var("SESSION_SECRET");
        let admin_bootstrap_secret = required_var("ADMIN_BOOTSTRAP_SECRET");
        let session_ttl_hours = parse_i64_var("SESSION_TTL_HOURS");
        let cookie_secure = parse_bool_var("COOKIE_SECURE");
        let admin_reauth_ttl_minutes = parse_i64_var("ADMIN_REAUTH_TTL_MINUTES");
        let trusted_proxy_cidrs = parse_cidr_list_var("TRUSTED_PROXY_CIDRS");
        let require_trusted_proxy = parse_bool_var("REQUIRE_TRUSTED_PROXY");
        let disable_auth_emails = optional_bool_var("DEV_DISABLE_AUTH_EMAILS", false);
        let resend_api_key = optional_var("RESEND_API_KEY");
        let resend_from_email = optional_var("RESEND_FROM_EMAIL");
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
            base_url: optional_var("FOOTBALL_API_BASE_URL").unwrap_or_default(),
            poll_interval_secs: optional_u64_var("FOOTBALL_POLL_INTERVAL_SECS", 600),
            live_poll_interval_secs: optional_u64_var("FOOTBALL_LIVE_POLL_INTERVAL_SECS", 300),
        };
        if football_enabled {
            assert!(
                !football.base_url.trim().is_empty(),
                "FOOTBALL_API_BASE_URL precisa ser configurada quando FOOTBALL_API_ENABLED=true"
            );
            assert!(
                football.poll_interval_secs >= 60,
                "FOOTBALL_POLL_INTERVAL_SECS deve ser >= 60"
            );
            assert!(
                football.live_poll_interval_secs >= 60,
                "FOOTBALL_LIVE_POLL_INTERVAL_SECS deve ser >= 60"
            );
        }

        let web_push_enabled = optional_bool_var("WEB_PUSH_ENABLED", false);
        let web_push = WebPushConfig {
            enabled: web_push_enabled,
            poll_interval_secs: optional_u64_var("WEB_PUSH_POLL_INTERVAL_SECS", 60),
            vapid_public_key: optional_var("WEB_PUSH_VAPID_PUBLIC_KEY"),
            vapid_private_key: optional_var("WEB_PUSH_VAPID_PRIVATE_KEY"),
            contact_email: optional_var("WEB_PUSH_CONTACT_EMAIL"),
        };
        let asset_dir = optional_var("PRESUMIDOS_ASSET_DIR")
            .unwrap_or_else(|| default_asset_dir(&database_path));
        let asset_max_upload_bytes =
            optional_u32_var("PRESUMIDOS_ASSET_MAX_UPLOAD_BYTES", 10 * 1024 * 1024) as usize;
        let asset_max_pixels = optional_u64_var("PRESUMIDOS_ASSET_MAX_PIXELS", 25_000_000);
        assert!(
            asset_max_upload_bytes > 0,
            "PRESUMIDOS_ASSET_MAX_UPLOAD_BYTES deve ser > 0"
        );
        assert!(
            asset_max_pixels > 0,
            "PRESUMIDOS_ASSET_MAX_PIXELS deve ser > 0"
        );
        if web_push_enabled {
            assert!(
                web_push.poll_interval_secs >= 30,
                "WEB_PUSH_POLL_INTERVAL_SECS deve ser >= 30"
            );
            assert!(
                web_push
                    .vapid_public_key
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()),
                "WEB_PUSH_VAPID_PUBLIC_KEY precisa ser configurada quando WEB_PUSH_ENABLED=true"
            );
            assert!(
                web_push
                    .vapid_private_key
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()),
                "WEB_PUSH_VAPID_PRIVATE_KEY precisa ser configurada quando WEB_PUSH_ENABLED=true"
            );
            assert!(
                web_push
                    .contact_email
                    .as_deref()
                    .is_some_and(|value| value.contains('@')),
                "WEB_PUSH_CONTACT_EMAIL precisa ser um email valido quando WEB_PUSH_ENABLED=true"
            );
        }

        validate_secret("SESSION_SECRET", &session_secret);
        validate_secret("ADMIN_BOOTSTRAP_SECRET", &admin_bootstrap_secret);
        assert!(
            session_secret != admin_bootstrap_secret,
            "SESSION_SECRET e ADMIN_BOOTSTRAP_SECRET devem ser diferentes"
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
        assert!(argon2_parallelism >= 1, "ARGON2_PARALLELISM deve ser >= 1");

        if app_env == "production" {
            assert!(
                Path::new(&database_path).is_absolute(),
                "DATABASE_PATH precisa ser absoluto em producao"
            );
            assert!(
                Path::new(&backup_dir).is_absolute(),
                "PRESUMIDOS_BACKUP_DIR precisa ser absoluto em producao"
            );
            assert!(
                Path::new(&asset_dir).is_absolute(),
                "PRESUMIDOS_ASSET_DIR precisa ser absoluto em producao"
            );
            assert!(
                cookie_secure,
                "COOKIE_SECURE precisa estar habilitado em producao"
            );
            assert!(
                public_base_url.is_some(),
                "PUBLIC_BASE_URL precisa ser configurada em producao"
            );
        }
        assert!(
            shutdown_timeout_secs > 0,
            "SHUTDOWN_TIMEOUT_SECS deve ser > 0"
        );
        assert!(min_free_bytes > 0, "PRESUMIDOS_MIN_FREE_BYTES deve ser > 0");
        assert!(
            database_busy_timeout_ms > 0,
            "PRESUMIDOS_DATABASE_BUSY_TIMEOUT_MS deve ser > 0"
        );
        assert!(max_body_bytes > 0, "PRESUMIDOS_MAX_BODY_BYTES deve ser > 0");
        let _: std::net::SocketAddr = listen_address
            .parse()
            .unwrap_or_else(|_| panic!("LISTEN_ADDRESS invalido"));
        if let Some(url) = public_base_url.as_deref() {
            assert!(
                url.starts_with("http://") || url.starts_with("https://"),
                "PUBLIC_BASE_URL precisa usar http:// ou https://"
            );
        }
        validate_auth_email_config(
            &app_env,
            disable_auth_emails,
            resend_api_key.as_deref(),
            resend_from_email.as_deref(),
        );
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
            backup_dir,
            public_base_url,
            listen_address,
            shutdown_timeout_secs,
            min_free_bytes,
            database_busy_timeout_ms,
            max_body_bytes,
            json_logs,
            metrics_enabled,
            contact_email,
            session_secret,
            admin_bootstrap_secret,
            session_ttl_hours,
            cookie_secure,
            admin_reauth_ttl_minutes,
            trusted_proxy_cidrs,
            require_trusted_proxy,
            resend_api_key: resend_api_key.unwrap_or_default(),
            resend_from_email: resend_from_email.unwrap_or_default(),
            disable_auth_emails,
            rate_limit_backend,
            redis_url,
            rate_limit_identity_secret,
            argon2_memory_kib,
            argon2_time_cost,
            argon2_parallelism,
            argon2_policy_version,
            football,
            web_push,
            asset_dir,
            asset_max_upload_bytes,
            asset_max_pixels,
        }
    })
}

#[cfg(feature = "server")]
pub fn check_config() -> Result<(), String> {
    let _ = dotenvy::dotenv();
    match std::panic::catch_unwind(settings) {
        Ok(config) => {
            if config.app_env == "production" {
                for (name, configured_path) in [
                    ("DATABASE_PATH", config.database_path.as_str()),
                    ("PRESUMIDOS_ASSET_DIR", config.asset_dir.as_str()),
                    ("PRESUMIDOS_BACKUP_DIR", config.backup_dir.as_str()),
                ] {
                    if let Some(parent) = Path::new(configured_path).parent() {
                        if !parent.as_os_str().is_empty() && !parent.exists() {
                            return Err(format!("o diretório pai de {name} não existe"));
                        }
                    }
                }
            }
            Ok(())
        }
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|value| value.to_string())
                })
                .unwrap_or_else(|| "configuração inválida".to_string());
            Err(message)
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{
        default_asset_dir, has_global_cidr, parse_environment, validate_auth_email_config,
        validate_proxy_config, validate_rate_limit_config, validate_secret, RateLimitBackendKind,
    };

    #[test]
    fn derives_asset_dir_from_absolute_database_path() {
        assert_eq!(
            default_asset_dir("/srv/presumidos/bolao.db"),
            "/srv/presumidos/data/assets"
        );
    }

    #[test]
    fn keeps_relative_asset_dir_for_relative_database_path() {
        assert_eq!(default_asset_dir("./data/bolao.db"), "./data/assets");
    }

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
    fn environment_is_explicitly_bounded() {
        assert_eq!(
            parse_environment("production").expect("production"),
            "production"
        );
        assert!(parse_environment("staging").is_err());
    }

    #[test]
    fn example_secret_is_rejected() {
        let result = std::panic::catch_unwind(|| {
            validate_secret(
                "SESSION_SECRET",
                "troque-este-segredo-por-um-valor-seguro-0123456789",
            );
        });
        assert!(result.is_err());
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
    fn development_can_disable_auth_emails_without_resend_creds() {
        validate_auth_email_config("development", true, None, None);
    }

    #[test]
    fn production_requires_resend_credentials_when_auth_emails_are_enabled() {
        let result = std::panic::catch_unwind(|| {
            validate_auth_email_config(
                "production",
                false,
                Some("re_test"),
                Some("Presumidos <no-reply@example.com>"),
            );
        });
        assert!(result.is_ok());

        let missing = std::panic::catch_unwind(|| {
            validate_auth_email_config("production", false, None, None);
        });
        assert!(missing.is_err());
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
