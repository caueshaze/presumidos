use super::super::{
    admin_reauth_is_fresh, argon2_policy, can_bootstrap_admin, cleanup_expired_auth_data,
    hash_password, needs_rehash, sqlite_utc_after_hours, sqlite_utc_now,
};
use super::{seed_security_env, test_db};

#[test]
fn sqlite_time_helpers_match_lexicographic_order() {
    let now = sqlite_utc_now();
    let future = sqlite_utc_after_hours(30);

    assert!(future > now);
    assert_eq!(now.len(), "2026-06-12 18:30:45".len());
    assert!(!now.contains('T'));
}

#[test]
fn admin_reauth_window_respects_recent_timestamps() {
    seed_security_env();

    let recent = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    assert!(admin_reauth_is_fresh(Some(&recent)));
    assert!(!admin_reauth_is_fresh(Some("1999-01-01 00:00:00")));
    assert!(!admin_reauth_is_fresh(None));
}

#[test]
fn bootstrap_requires_exact_secret_and_empty_admin_set() {
    seed_security_env();

    assert!(can_bootstrap_admin(
        false,
        "bootstrap-secret-super-seguro-0123456789abcdef",
        "bootstrap-secret-super-seguro-0123456789abcdef"
    ));
    assert!(!can_bootstrap_admin(
        false,
        "errado",
        "bootstrap-secret-super-seguro-0123456789abcdef"
    ));
    assert!(!can_bootstrap_admin(
        true,
        "bootstrap-secret-super-seguro-0123456789abcdef",
        "bootstrap-secret-super-seguro-0123456789abcdef"
    ));
}

#[test]
fn argon2_policy_matches_configured_parameters() {
    seed_security_env();

    let cfg = crate::config::settings();
    let policy = argon2_policy();
    assert_eq!(policy.params().m_cost(), cfg.argon2_memory_kib);
    assert_eq!(policy.params().t_cost(), cfg.argon2_time_cost);
    assert_eq!(policy.params().p_cost(), cfg.argon2_parallelism);
}

#[test]
fn needs_rehash_detects_outdated_parameters() {
    use argon2::password_hash::{PasswordHash, PasswordHasher, SaltString};
    use argon2::{Algorithm, Argon2, Params, Version};
    use rand_core::OsRng;

    seed_security_env();

    let weak_params = Params::new(19456, 1, 1, None).expect("weak params");
    let weak_hasher = Argon2::new(Algorithm::Argon2id, Version::V0x13, weak_params);
    let salt = SaltString::generate(&mut OsRng);
    let weak_hash = weak_hasher
        .hash_password(b"senha-teste", &salt)
        .expect("hash with weak params")
        .to_string();
    let parsed_weak = PasswordHash::new(&weak_hash).expect("parse weak hash");
    assert!(needs_rehash(&parsed_weak));

    let current_hash = hash_password("senha-teste").expect("hash with current policy");
    let parsed_current = PasswordHash::new(&current_hash).expect("parse current hash");
    assert!(!needs_rehash(&parsed_current));
}

#[tokio::test]
async fn cleanup_expired_auth_data_removes_only_stale_records() {
    seed_security_env();
    let db = test_db().await;

    sqlx::query(
            "INSERT INTO sessions (token, user_id, expires_at, csrf_token, last_seen_at)
             VALUES
                ('expired-session', 'user-a', datetime('now', '-2 hours'), 'csrf-a', datetime('now')),
                ('active-session', 'user-b', datetime('now', '+2 hours'), 'csrf-b', datetime('now'))",
        )
        .execute(&db)
        .await
        .expect("seed sessions");

    sqlx::query(
            "INSERT INTO pending_registrations
                (email, username, username_lookup, password_hash, code_hash, attempts, expires_at, created_at)
             VALUES
                ('old@teste.com', 'Old', 'old', 'hash', 'code', 0, datetime('now', '-2 hours'), datetime('now', '-2 days')),
                ('fresh@teste.com', 'Fresh', 'fresh', 'hash', 'code', 0, datetime('now', '+2 hours'), datetime('now'))",
        )
        .execute(&db)
        .await
        .expect("seed pending registrations");

    sqlx::query(
            "INSERT INTO password_reset_codes
                (email, user_id, code_hash, attempts, expires_at, created_at)
             VALUES
                ('reset-old@teste.com', 'user-a', 'code', 0, datetime('now', '-2 hours'), datetime('now', '-2 days')),
                ('reset-fresh@teste.com', 'user-b', 'code', 0, datetime('now', '+2 hours'), datetime('now'))",
        )
        .execute(&db)
        .await
        .expect("seed password reset codes");

    let summary = cleanup_expired_auth_data(&db)
        .await
        .expect("cleanup should succeed");

    assert_eq!(summary.expired_sessions_deleted, 1);
    assert_eq!(summary.expired_pending_registrations_deleted, 1);
    assert_eq!(summary.expired_password_reset_codes_deleted, 1);

    let remaining_sessions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE token = 'active-session'")
            .fetch_one(&db)
            .await
            .expect("count active sessions");
    assert_eq!(remaining_sessions.0, 1);

    let remaining_pending: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pending_registrations WHERE email = 'fresh@teste.com'",
    )
    .fetch_one(&db)
    .await
    .expect("count fresh pending");
    assert_eq!(remaining_pending.0, 1);

    let remaining_reset: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM password_reset_codes WHERE email = 'reset-fresh@teste.com'",
    )
    .fetch_one(&db)
    .await
    .expect("count fresh reset");
    assert_eq!(remaining_reset.0, 1);
}
