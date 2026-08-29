use super::{env::*, types::*, validation::*};
use std::path::Path;

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
