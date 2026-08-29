use super::{env::required_var, types::RateLimitBackendKind};

pub(crate) fn parse_environment(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "development" | "test" | "production" => Ok(normalized),
        _ => Err("APP_ENV deve ser development, test ou production".to_string()),
    }
}

#[cfg(feature = "server")]
pub(crate) fn validate_secret(name: &str, value: &str) {
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
pub(crate) fn parse_rate_limit_backend_var(name: &str) -> RateLimitBackendKind {
    match required_var(name).trim().to_lowercase().as_str() {
        "memory" => RateLimitBackendKind::Memory,
        "redis" => RateLimitBackendKind::Redis,
        _ => panic!("variavel {name} deve ser 'memory' ou 'redis'"),
    }
}

#[cfg(feature = "server")]
pub(crate) fn has_global_cidr(cidrs: &[ipnet::IpNet]) -> bool {
    cidrs.iter().any(|cidr| match cidr {
        ipnet::IpNet::V4(net) => net.prefix_len() == 0,
        ipnet::IpNet::V6(net) => net.prefix_len() == 0,
    })
}

#[cfg(feature = "server")]
pub(crate) fn validate_proxy_config(
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
pub(crate) fn validate_rate_limit_config(
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
pub(crate) fn validate_auth_email_config(
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
