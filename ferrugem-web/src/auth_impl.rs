use super::*;

#[path = "auth_support.rs"]
mod support;
pub(crate) use support::*;

#[path = "auth_ops.rs"]
mod ops;
pub(crate) use ops::*;

#[path = "auth_registration.rs"]
mod registration;
pub(crate) use registration::*;

#[path = "auth_login.rs"]
mod login;
pub(crate) use login::*;

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{
        admin_reauth_is_fresh, argon2_policy, can_bootstrap_admin, cleanup_expired_auth_data,
        count_admins, create_bootstrap_admin_account, create_public_user_account, hash_password,
        needs_rehash, sqlite_utc_after_hours, sqlite_utc_now, validate_registration_input,
    };
    use sqlx::SqlitePool;

    async fn test_db() -> SqlitePool {
        let db = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("memory sqlite should connect");

        sqlx::query(
            "CREATE TABLE users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                is_admin INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&db)
        .await
        .expect("users table");

        sqlx::query(
            "CREATE TABLE sessions (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT NOT NULL,
                csrf_token TEXT NOT NULL DEFAULT '',
                admin_reauthed_at TEXT,
                last_seen_at TEXT
            )",
        )
        .execute(&db)
        .await
        .expect("sessions table");

        sqlx::query(
            "CREATE TABLE audit_logs (
                id TEXT PRIMARY KEY,
                actor_user_id TEXT,
                action TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_id TEXT,
                ip_address TEXT,
                details_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&db)
        .await
        .expect("audit_logs table");

        sqlx::query(
            "CREATE TABLE pending_registrations (
                email TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                username_lookup TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                code_hash TEXT NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&db)
        .await
        .expect("pending_registrations table");

        sqlx::query(
            "CREATE TABLE password_reset_codes (
                email TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                code_hash TEXT NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&db)
        .await
        .expect("password_reset_codes table");

        db
    }

    fn seed_security_env() {
        // DB de teste sempre num arquivo unico em temp_dir: independente do diretorio
        // de onde `cargo test` roda e sem deixar `.db` obsoleto dentro do repo.
        let db_path =
            std::env::temp_dir().join(format!("presumidos-test-{}.db", uuid::Uuid::new_v4()));
        std::env::set_var("APP_ENV", "test");
        std::env::set_var("DATABASE_PATH", db_path.to_string_lossy().to_string());
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

    #[tokio::test]
    async fn public_registration_flow_never_creates_admin() {
        seed_security_env();
        let db = test_db().await;
        let (username, username_lookup, email) = validate_registration_input(
            "Caue".to_string(),
            "caue@teste.com".to_string(),
            "senha-super-segura",
        )
        .expect("input should validate");

        let user_id = create_public_user_account(
            &db,
            &username,
            &username_lookup,
            &email,
            "senha-super-segura",
        )
        .await
        .expect("public registration should work");

        let row: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?1")
            .bind(&user_id)
            .fetch_one(&db)
            .await
            .expect("user should exist");

        assert!(!row.0);
        assert_eq!(count_admins(&db).await.expect("count admins"), 0);
    }

    #[tokio::test]
    async fn bootstrap_admin_creates_first_admin_and_blocks_second_one() {
        seed_security_env();
        let db = test_db().await;
        let (username, username_lookup, email) = validate_registration_input(
            "Root".to_string(),
            "root@teste.com".to_string(),
            "senha-super-segura",
        )
        .expect("input should validate");

        let user_id = create_bootstrap_admin_account(
            &db,
            &username,
            &username_lookup,
            &email,
            "senha-super-segura",
            "bootstrap-secret-super-seguro-0123456789abcdef",
            "127.0.0.1",
        )
        .await
        .expect("bootstrap should create first admin");

        let row: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?1")
            .bind(&user_id)
            .fetch_one(&db)
            .await
            .expect("admin should exist");
        assert!(row.0);
        assert_eq!(count_admins(&db).await.expect("count admins"), 1);

        let audit_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_logs WHERE action = 'bootstrap_admin_created_explicit'",
        )
        .fetch_one(&db)
        .await
        .expect("audit should exist");
        assert_eq!(audit_count.0, 1);

        let second = create_bootstrap_admin_account(
            &db,
            "Outro",
            "outro",
            "outro@teste.com",
            "senha-super-segura",
            "bootstrap-secret-super-seguro-0123456789abcdef",
            "127.0.0.1",
        )
        .await;
        assert!(second.is_err());

        let blocked_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_logs WHERE action = 'bootstrap_admin_blocked_existing_admin'",
        )
        .fetch_one(&db)
        .await
        .expect("blocked audit should exist");
        assert_eq!(blocked_count.0, 1);
    }

    #[tokio::test]
    async fn bootstrap_admin_invalid_secret_is_audited_without_creating_admin() {
        seed_security_env();
        let db = test_db().await;

        let attempt = create_bootstrap_admin_account(
            &db,
            "Root",
            "root",
            "root@teste.com",
            "senha-super-segura",
            "segredo-incorreto",
            "127.0.0.1",
        )
        .await;
        assert!(attempt.is_err());

        assert_eq!(count_admins(&db).await.expect("count admins"), 0);

        let failed_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_logs WHERE action = 'bootstrap_admin_failed_invalid_secret'",
        )
        .fetch_one(&db)
        .await
        .expect("failed audit should exist");
        assert_eq!(failed_count.0, 1);
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
}
