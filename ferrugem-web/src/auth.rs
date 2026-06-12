use dioxus::prelude::*;

use crate::models::{AuthResult, SessionState, UserPublic};

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
}

#[cfg(feature = "server")]
type HeaderMap = dioxus::prelude::dioxus_fullstack::HeaderMap;

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub token: String,
    pub user_id: String,
    pub csrf_token: String,
    pub admin_reauthed_at: Option<String>,
}

#[cfg(feature = "server")]
fn sqlite_utc_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(feature = "server")]
fn sqlite_utc_after_hours(hours: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::hours(hours))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(feature = "server")]
fn parsed_sqlite_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
}

/// Estado de autenticação compartilhado via contexto.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthState {
    pub user: Option<UserPublic>,
    pub token: String,
    pub csrf_token: String,
    pub loading: bool,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            user: None,
            token: String::new(),
            csrf_token: String::new(),
            loading: true,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn clear_token() {
    const TOKEN_KEY: &str = "bolao_token";
    if let Some(storage) = local_storage() {
        let _ = storage.remove_item(TOKEN_KEY);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn clear_token() {}

#[cfg(target_arch = "wasm32")]
pub async fn load_token() -> String {
    const TOKEN_KEY: &str = "bolao_token";
    local_storage()
        .and_then(|storage| storage.get_item(TOKEN_KEY).ok())
        .flatten()
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn load_token() -> String {
    String::new()
}

#[cfg(feature = "server")]
async fn delete_session_by_token(
    db: &sqlx::SqlitePool,
    token: &str,
) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM sessions WHERE token = ?1")
        .bind(token)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("delete_session_by_token", e))?;
    Ok(())
}

#[cfg(feature = "server")]
async fn invalidate_user_sessions(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("invalidate_user_sessions", e))?;
    Ok(())
}

#[cfg(feature = "server")]
async fn create_session(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<AuthSession, ServerFnError> {
    let token = uuid::Uuid::new_v4().to_string();
    let csrf_token = crate::security::csrf_token();
    let now = sqlite_utc_now();
    let expires_at = sqlite_utc_after_hours(crate::config::settings().session_ttl_hours);

    sqlx::query(
        "INSERT INTO sessions
            (token, user_id, expires_at, csrf_token, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(&expires_at)
    .bind(&csrf_token)
    .bind(&now)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("create_session", e))?;

    Ok(AuthSession {
        token,
        user_id: user_id.to_string(),
        csrf_token,
        admin_reauthed_at: None,
    })
}

#[cfg(feature = "server")]
async fn touch_session(db: &sqlx::SqlitePool, session: &AuthSession) -> Result<(), ServerFnError> {
    let now = sqlite_utc_now();
    let expires_at = sqlite_utc_after_hours(crate::config::settings().session_ttl_hours);

    sqlx::query(
        "UPDATE sessions
         SET expires_at = ?1, last_seen_at = ?2
         WHERE token = ?3",
    )
    .bind(&expires_at)
    .bind(&now)
    .bind(&session.token)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("touch_session", e))?;

    crate::security::set_session_cookie(&session.token);
    Ok(())
}

#[cfg(feature = "server")]
async fn resolve_session(
    db: &sqlx::SqlitePool,
    legacy_token: &str,
    headers: &HeaderMap,
) -> Result<Option<AuthSession>, ServerFnError> {
    let cookie_token = crate::security::parse_cookie(headers, crate::security::session_cookie_name());
    let token = cookie_token
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| legacy_token.trim().to_string());

    if token.is_empty() {
        return Ok(None);
    }

    let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT user_id, csrf_token, expires_at, admin_reauthed_at
         FROM sessions
         WHERE token = ?1",
    )
    .bind(&token)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("resolve_session", e))?;

    let Some((user_id, csrf_token, expires_at, admin_reauthed_at)) = row else {
        crate::security::clear_session_cookie();
        return Ok(None);
    };

    let expired = parsed_sqlite_utc(&expires_at)
        .map(|value| chrono::Utc::now() >= value)
        .unwrap_or(true);

    if expired {
        delete_session_by_token(db, &token).await?;
        crate::security::clear_session_cookie();
        crate::security::log_event(
            "session_expired",
            serde_json::json!({
                "user_id": user_id,
            }),
        );
        return Ok(None);
    }

    let session = AuthSession {
        token,
        user_id,
        csrf_token,
        admin_reauthed_at,
    };

    touch_session(db, &session).await?;
    Ok(Some(session))
}

#[cfg(feature = "server")]
async fn load_user_public(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<UserPublic, ServerFnError> {
    let row: (String, String, String, bool) =
        sqlx::query_as("SELECT id, username, email, is_admin FROM users WHERE id = ?1")
            .bind(user_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("load_user_public", e))?;

    Ok(UserPublic {
        id: row.0,
        username: row.1,
        email: row.2,
        is_admin: row.3,
    })
}

#[cfg(feature = "server")]
fn admin_reauth_is_fresh(value: Option<&str>) -> bool {
    let ttl = chrono::Duration::minutes(crate::config::settings().admin_reauth_ttl_minutes);
    value.and_then(parsed_sqlite_utc)
        .is_some_and(|stamp| chrono::Utc::now() - stamp <= ttl)
}

#[cfg(feature = "server")]
pub async fn require_user(token: &str) -> Result<AuthSession, ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();

    resolve_session(pool(), token, &headers)
        .await?
        .ok_or_else(|| crate::security::public_error("Sessao invalida. Faca login novamente."))
}

#[cfg(feature = "server")]
pub async fn require_admin(token: &str) -> Result<AuthSession, ServerFnError> {
    use crate::db::pool;

    let session = require_user(token).await?;
    let row: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?1")
        .bind(&session.user_id)
        .fetch_one(pool())
        .await
        .map_err(|e| crate::security::internal_error("require_admin", e))?;

    if !row.0 {
        return Err(crate::security::public_error(
            "Apenas administradores podem realizar esta acao.",
        ));
    }

    Ok(session)
}

#[cfg(feature = "server")]
pub async fn require_recent_admin(token: &str) -> Result<AuthSession, ServerFnError> {
    let session = require_admin(token).await?;

    if !admin_reauth_is_fresh(session.admin_reauthed_at.as_deref()) {
        return Err(crate::security::public_error(
            "SECURITY:ADMIN_REAUTH_REQUIRED",
        ));
    }

    Ok(session)
}

#[server]
pub async fn register(
    username: String,
    email: String,
    password: String,
) -> Result<AuthResult, ServerFnError> {
    use crate::db::pool;
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHasher};
    use rand_core::OsRng;
    use std::time::Duration;
    use uuid::Uuid;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(
        "register",
        &ip,
        crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 5,
        },
    )?;

    let username = crate::security::normalize_required_text("Usuario", username, 3, 32)?;
    let username_lookup = username.to_lowercase();
    let email = crate::security::normalize_email(email)?;
    if password.len() < 8 || password.len() > 128 {
        return Err(crate::security::public_error(
            "A senha deve ter entre 8 e 128 caracteres.",
        ));
    }

    let db = pool();

    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE lower(username) = ?1 OR lower(email) = ?2")
            .bind(&username_lookup)
            .bind(&email)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("register_existing", e))?;

    if existing.is_some() {
        return Err(crate::security::public_error(
            "Usuario ou email ja cadastrado.",
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| crate::security::internal_error("register_hash_password", e))?
        .to_string();

    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("register_user_count", e))?;

    let is_admin = user_count.0 == 0;
    let user_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, is_admin)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&user_id)
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .bind(is_admin)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("register_insert_user", e))?;

    let session = create_session(db, &user_id).await?;
    crate::security::set_session_cookie(&session.token);

    if is_admin {
        crate::security::append_audit_log(
            db,
            Some(&user_id),
            "bootstrap_admin_created",
            "user",
            Some(&user_id),
            Some(&ip),
            serde_json::json!({
                "username": username,
                "email": email,
            }),
        )
        .await?;
    }

    crate::security::log_event(
        "register_success",
        serde_json::json!({
            "user_id": user_id,
            "ip": ip,
            "is_admin": is_admin,
        }),
    );

    Ok(AuthResult {
        user: UserPublic {
            id: user_id,
            username,
            email,
            is_admin,
        },
        token: String::new(),
        csrf_token: session.csrf_token,
    })
}

#[server]
pub async fn login(
    username: String,
    password: String,
) -> Result<AuthResult, ServerFnError> {
    use crate::db::pool;
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();

    let login_identifier = crate::security::normalize_optional_text(username, 120)?.to_lowercase();
    let ip = crate::security::client_ip(&headers);

    crate::security::enforce_rate_limit(
        "login:ip",
        &ip,
        crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 10,
        },
    )?;
    crate::security::enforce_rate_limit(
        "login:id",
        &format!("{ip}:{login_identifier}"),
        crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 5,
        },
    )?;

    if password.len() > 128 {
        return Err(crate::security::public_error("Usuario ou senha invalidos."));
    }

    let db = pool();

    let row: Option<(String, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, username, email, password_hash, is_admin
         FROM users
         WHERE lower(username) = ?1 OR lower(email) = ?1",
    )
    .bind(&login_identifier)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("login_lookup_user", e))?;

    let Some((id, username, email, password_hash, is_admin)) = row else {
        crate::security::log_event(
            "login_failed",
            serde_json::json!({
                "reason": "missing_user",
                "login_identifier": login_identifier,
                "ip": ip,
            }),
        );
        return Err(crate::security::public_error("Usuario ou senha invalidos."));
    };

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|e| crate::security::internal_error("login_parse_hash", e))?;

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        crate::security::log_event(
            "login_failed",
            serde_json::json!({
                "reason": "bad_password",
                "user_id": id,
                "ip": ip,
            }),
        );
        return Err(crate::security::public_error("Usuario ou senha invalidos."));
    }

    invalidate_user_sessions(db, &id).await?;
    let session = create_session(db, &id).await?;
    crate::security::set_session_cookie(&session.token);

    crate::security::log_event(
        "login_success",
        serde_json::json!({
            "user_id": id,
            "ip": ip,
        }),
    );

    Ok(AuthResult {
        user: UserPublic {
            id,
            username,
            email,
            is_admin,
        },
        token: String::new(),
        csrf_token: session.csrf_token,
    })
}

#[server]
pub async fn confirm_admin_password(
    password: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::db::pool;
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(
        "admin_reauth",
        &ip,
        crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 8,
        },
    )?;

    let session = require_admin("").await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();
    let row: (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("confirm_admin_password_lookup", e))?;

    let parsed_hash = PasswordHash::new(&row.0)
        .map_err(|e| crate::security::internal_error("confirm_admin_password_parse", e))?;

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        crate::security::log_event(
            "admin_reauth_failed",
            serde_json::json!({
                "user_id": session.user_id,
                "ip": ip,
            }),
        );
        return Err(crate::security::public_error("Senha de administrador invalida."));
    }

    let now = sqlite_utc_now();
    sqlx::query("UPDATE sessions SET admin_reauthed_at = ?1 WHERE token = ?2")
        .bind(&now)
        .bind(&session.token)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("confirm_admin_password_update", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "admin_reauthenticated",
        "session",
        Some(&session.token),
        Some(&ip),
        serde_json::json!({}),
    )
    .await?;

    Ok(())
}

#[server]
pub async fn logout(
    token: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();

    let db = pool();
    if let Some(session) = resolve_session(db, &token, &headers).await? {
        crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
        delete_session_by_token(db, &session.token).await?;
        crate::security::log_event(
            "logout",
            serde_json::json!({
                "user_id": session.user_id,
                "ip": crate::security::client_ip(&headers),
            }),
        );
    }
    crate::security::clear_session_cookie();
    Ok(())
}

#[server]
pub async fn current_user(
    token: String,
) -> Result<SessionState, ServerFnError> {
    use crate::db::pool;
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(
        "current_user",
        &ip,
        crate::security::RateLimitRule {
            window: Duration::from_secs(30),
            max_attempts: 30,
        },
    )?;

    let db = pool();
    let session = resolve_session(db, &token, &headers).await?;

    let Some(session) = session else {
        crate::security::clear_session_cookie();
        return Ok(SessionState {
            user: None,
            csrf_token: String::new(),
        });
    };

    let user = load_user_public(db, &session.user_id).await?;

    Ok(SessionState {
        user: Some(user),
        csrf_token: session.csrf_token,
    })
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{admin_reauth_is_fresh, sqlite_utc_after_hours, sqlite_utc_now};

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
        std::env::set_var("APP_ENV", "test");
        std::env::set_var("DATABASE_PATH", "test.db");
        std::env::set_var(
            "SESSION_SECRET",
            "0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        std::env::set_var("SESSION_TTL_HOURS", "12");
        std::env::set_var("COOKIE_SECURE", "false");
        std::env::set_var("ADMIN_REAUTH_TTL_MINUTES", "10");

        let recent = chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        assert!(admin_reauth_is_fresh(Some(&recent)));
        assert!(!admin_reauth_is_fresh(Some("1999-01-01 00:00:00")));
        assert!(!admin_reauth_is_fresh(None));
    }
}
