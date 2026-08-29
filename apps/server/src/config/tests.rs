use super::{
    env::default_asset_dir,
    types::RateLimitBackendKind,
    validation::{
        has_global_cidr, parse_environment, validate_auth_email_config, validate_proxy_config,
        validate_rate_limit_config, validate_secret,
    },
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
