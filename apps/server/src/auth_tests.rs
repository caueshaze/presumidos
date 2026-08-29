use sqlx::SqlitePool;

pub(super) async fn test_db() -> SqlitePool {
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

pub(super) fn seed_security_env() {
    // DB de teste sempre num arquivo unico em temp_dir: independente do diretorio
    // de onde `cargo test` roda e sem deixar `.db` obsoleto dentro do repo.
    let db_path = std::env::temp_dir().join(format!("presumidos-test-{}.db", uuid::Uuid::new_v4()));
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

#[path = "auth_tests/bootstrap.rs"]
mod bootstrap;

#[path = "auth_tests/security.rs"]
mod security;
