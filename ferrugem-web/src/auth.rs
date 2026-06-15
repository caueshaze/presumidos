use crate::error::ServerFnError;

use crate::models::{AuthResult, SessionState, UserPublic};

#[cfg(feature = "server")]
use axum::http::HeaderMap;

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub token: String,
    pub user_id: String,
    pub csrf_token: String,
    pub admin_reauthed_at: Option<String>,
}

#[cfg(feature = "server")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AuthCleanupSummary {
    pub expired_sessions_deleted: u64,
    pub expired_pending_registrations_deleted: u64,
    pub expired_password_reset_codes_deleted: u64,
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
fn sqlite_utc_after_minutes(minutes: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::minutes(minutes))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Validade dos codigos de verificacao por email, em minutos.
#[cfg(feature = "server")]
const EMAIL_CODE_TTL_MINUTES: i64 = 15;

/// Numero maximo de tentativas de digitacao de um codigo antes de exigir um novo envio.
#[cfg(feature = "server")]
const EMAIL_CODE_MAX_ATTEMPTS: i64 = 5;

#[cfg(feature = "server")]
type UserPublicRow = (String, String, String, bool, Option<String>, Option<String>);

#[cfg(feature = "server")]
type LoginRow = (
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
);

#[cfg(feature = "server")]
fn parsed_sqlite_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
}

#[cfg(feature = "server")]
async fn insert_audit_log(
    db: &sqlx::SqlitePool,
    actor_user_id: Option<&str>,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    ip: Option<&str>,
    details: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip)
    .bind(details.to_string())
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("insert_audit_log", e))?;

    Ok(())
}

#[cfg(feature = "server")]
async fn insert_audit_log_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    actor_user_id: Option<&str>,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    ip: Option<&str>,
    details: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip)
    .bind(details.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|e| crate::security::internal_error("insert_audit_log_tx", e))?;

    Ok(())
}

#[cfg(feature = "server")]
async fn delete_session_by_token(db: &sqlx::SqlitePool, token: &str) -> Result<(), ServerFnError> {
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
    let cookie_token =
        crate::security::parse_cookie(headers, crate::security::session_cookie_name());
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
    let row: UserPublicRow =
        sqlx::query_as("SELECT id, username, email, is_admin, blocked_at, blocked_reason FROM users WHERE id = ?1")
            .bind(user_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("load_user_public", e))?;

    Ok(UserPublic {
        id: row.0,
        username: row.1,
        email: row.2,
        is_admin: row.3,
        blocked_at: row.4,
        blocked_reason: row.5,
    })
}

#[cfg(feature = "server")]
fn admin_reauth_is_fresh(value: Option<&str>) -> bool {
    let ttl = chrono::Duration::minutes(crate::config::settings().admin_reauth_ttl_minutes);
    value
        .and_then(parsed_sqlite_utc)
        .is_some_and(|stamp| chrono::Utc::now() - stamp <= ttl)
}

#[cfg(feature = "server")]
fn can_bootstrap_admin(has_any_admin: bool, provided_secret: &str, expected_secret: &str) -> bool {
    !has_any_admin
        && !provided_secret.trim().is_empty()
        && provided_secret.trim() == expected_secret.trim()
}

#[cfg(feature = "server")]
async fn user_exists_by_identity(
    db: &sqlx::SqlitePool,
    username_lookup: &str,
    email: &str,
) -> Result<bool, ServerFnError> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE lower(username) = ?1 OR lower(email) = ?2")
            .bind(username_lookup)
            .bind(email)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("user_exists_by_identity", e))?;

    Ok(existing.is_some())
}

#[cfg(feature = "server")]
async fn count_admins(db: &sqlx::SqlitePool) -> Result<i64, ServerFnError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin = 1")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("count_admins", e))?;

    Ok(row.0)
}

#[cfg(feature = "server")]
pub(crate) async fn insert_user_account(
    db: &sqlx::SqlitePool,
    username: &str,
    email: &str,
    password_hash: &str,
    is_admin: bool,
) -> Result<String, ServerFnError> {
    let user_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, is_admin)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&user_id)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(is_admin)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("insert_user_account", e))?;

    Ok(user_id)
}

#[cfg(feature = "server")]
#[cfg_attr(not(test), allow(dead_code))]
async fn create_public_user_account(
    db: &sqlx::SqlitePool,
    username: &str,
    username_lookup: &str,
    email: &str,
    password: &str,
) -> Result<String, ServerFnError> {
    if user_exists_by_identity(db, username_lookup, email).await? {
        return Err(crate::security::public_error(
            "Usuario ou email ja cadastrado.",
        ));
    }

    let password_hash = hash_password(password)?;
    insert_user_account(db, username, email, &password_hash, false).await
}

#[cfg(feature = "server")]
async fn create_bootstrap_admin_account(
    db: &sqlx::SqlitePool,
    username: &str,
    username_lookup: &str,
    email: &str,
    password: &str,
    bootstrap_secret: &str,
    ip: &str,
) -> Result<String, ServerFnError> {
    let has_any_admin = count_admins(db).await? > 0;
    if has_any_admin {
        insert_audit_log(
            db,
            None,
            "bootstrap_admin_blocked_existing_admin",
            "user",
            None,
            Some(ip),
            serde_json::json!({
                "username": username,
                "email": email,
            }),
        )
        .await?;
        return Err(crate::security::public_error(
            "O bootstrap inicial de administrador nao esta mais disponivel.",
        ));
    }

    if !can_bootstrap_admin(
        has_any_admin,
        bootstrap_secret,
        &crate::config::settings().admin_bootstrap_secret,
    ) {
        insert_audit_log(
            db,
            None,
            "bootstrap_admin_failed_invalid_secret",
            "user",
            None,
            Some(ip),
            serde_json::json!({
                "username": username,
                "email": email,
            }),
        )
        .await?;
        return Err(crate::security::public_error(
            "Credencial de bootstrap invalida.",
        ));
    }

    if user_exists_by_identity(db, username_lookup, email).await? {
        return Err(crate::security::public_error(
            "Usuario ou email ja cadastrado.",
        ));
    }

    let password_hash = hash_password(password)?;

    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("bootstrap_admin_begin", e))?;

    let user_id = {
        let user_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, is_admin)
             VALUES (?1, ?2, ?3, ?4, 1)",
        )
        .bind(&user_id)
        .bind(username)
        .bind(email)
        .bind(&password_hash)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("bootstrap_admin_insert_user", e))?;
        user_id
    };

    insert_audit_log_tx(
        &mut tx,
        Some(user_id.as_str()),
        "bootstrap_admin_created_explicit",
        "user",
        Some(user_id.as_str()),
        Some(ip),
        serde_json::json!({
            "username": username,
            "email": email,
        }),
    )
    .await
    .map_err(|e| crate::security::internal_error("bootstrap_admin_audit", e))?;

    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("bootstrap_admin_commit", e))?;

    Ok(user_id)
}

#[cfg(feature = "server")]
fn validate_registration_input(
    username: String,
    email: String,
    password: &str,
) -> Result<(String, String, String), ServerFnError> {
    let username = crate::security::normalize_required_text("Usuario", username, 3, 32)?;
    let username_lookup = username.to_lowercase();
    let email = crate::security::normalize_email(email)?;
    if password.len() < 8 || password.len() > 128 {
        return Err(crate::security::public_error(
            "A senha deve ter entre 8 e 128 caracteres.",
        ));
    }

    Ok((username, username_lookup, email))
}

#[cfg(feature = "server")]
fn argon2_policy() -> argon2::Argon2<'static> {
    use argon2::{Algorithm, Argon2, Params, Version};

    let cfg = crate::config::settings();
    let params = Params::new(
        cfg.argon2_memory_kib,
        cfg.argon2_time_cost,
        cfg.argon2_parallelism,
        None,
    )
    .expect("parametros de argon2 invalidos");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

#[cfg(feature = "server")]
fn needs_rehash(parsed_hash: &argon2::password_hash::PasswordHash<'_>) -> bool {
    use argon2::{Params, Version};

    let cfg = crate::config::settings();
    if parsed_hash.version != Some(Version::V0x13 as u32) {
        return true;
    }

    match Params::try_from(parsed_hash) {
        Ok(params) => {
            params.m_cost() != cfg.argon2_memory_kib
                || params.t_cost() != cfg.argon2_time_cost
                || params.p_cost() != cfg.argon2_parallelism
        }
        Err(_) => true,
    }
}

#[cfg(feature = "server")]
pub(crate) fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::password_hash::SaltString;
    use argon2::PasswordHasher;
    use rand_core::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    argon2_policy()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| crate::security::internal_error("hash_password", e))
        .map(|hash| hash.to_string())
}

#[cfg(feature = "server")]
pub async fn require_user(token: &str) -> Result<AuthSession, ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

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

/// Troca o nome de usuário da conta autenticada. Mantém as mesmas regras de
/// validação e unicidade (case-insensitive) do cadastro.
#[cfg(feature = "server")]
pub async fn change_username(
    token: String,
    new_username: String,
    csrf_token: String,
) -> Result<crate::models::UserPublic, ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let username = crate::security::normalize_required_text("Usuario", new_username, 3, 32)?;
    let username_lookup = username.to_lowercase();

    let db = pool();

    // Unicidade contra OUTROS usuários (permite ajustar maiúsc./minúsc. do próprio nome).
    let taken: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE lower(username) = ?1 AND id != ?2")
            .bind(&username_lookup)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("change_username_lookup", e))?;
    if taken.is_some() {
        return Err(crate::security::public_error(
            "Esse nome de usuario ja esta em uso.",
        ));
    }

    sqlx::query("UPDATE users SET username = ?1 WHERE id = ?2")
        .bind(&username)
        .bind(&session.user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("change_username_update", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "username_changed",
        "user",
        Some(&session.user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "new_username": username }),
    )
    .await?;

    let row: (String, String, String, bool) =
        sqlx::query_as("SELECT id, username, email, is_admin FROM users WHERE id = ?1")
            .bind(&session.user_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("change_username_fetch", e))?;

    Ok(crate::models::UserPublic {
        id: row.0,
        username: row.1,
        email: row.2,
        is_admin: row.3,
        blocked_at: None,
        blocked_reason: None,
    })
}

/// Exclui a própria conta autenticada e limpa os dados operacionais
/// diretamente vinculados a ela.
///
/// Restrições atuais:
/// - a conta não pode ser a criadora de bolões ainda existentes;
/// - a última conta admin não pode se autoexcluir.
#[cfg(feature = "server")]
pub async fn delete_account(token: String, csrf_token: String) -> Result<(), ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();
    let (email, is_admin): (String, bool) =
        sqlx::query_as("SELECT email, is_admin FROM users WHERE id = ?1")
            .bind(&session.user_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("delete_account_load_user", e))?;

    let owned_pools: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE created_by = ?1")
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_count_owned_pools", e))?;
    if owned_pools.0 > 0 {
        return Err(crate::security::public_error(
            "Sua conta ainda criou bolões ativos. Apague os bolões criados por voce antes de excluir a conta.",
        ));
    }

    if is_admin && count_admins(db).await? <= 1 {
        return Err(crate::security::public_error(
            "Nao e possivel excluir a unica conta de administrador.",
        ));
    }

    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("delete_account_begin_tx", e))?;

    insert_audit_log_tx(
        &mut tx,
        None,
        "account_deleted",
        "user",
        Some(&session.user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "email_hash": crate::security::sensitive_value_hash(&email),
            "self_service": true
        }),
    )
    .await?;

    sqlx::query("DELETE FROM push_reminder_deliveries WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_reminder_deliveries", e))?;

    sqlx::query("DELETE FROM push_subscriptions WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_push_subscriptions", e))?;

    sqlx::query("DELETE FROM notification_preferences WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            crate::security::internal_error("delete_account_notification_preferences", e)
        })?;

    sqlx::query("DELETE FROM point_adjustments WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_point_adjustments", e))?;

    sqlx::query("DELETE FROM predictions WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_predictions", e))?;

    sqlx::query("DELETE FROM pool_members WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_pool_members", e))?;

    sqlx::query("DELETE FROM password_reset_codes WHERE user_id = ?1 OR email = ?2")
        .bind(&session.user_id)
        .bind(&email)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_password_reset_codes", e))?;

    sqlx::query("DELETE FROM pending_registrations WHERE email = ?1")
        .bind(&email)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_pending_registrations", e))?;

    sqlx::query("UPDATE audit_logs SET actor_user_id = NULL WHERE actor_user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_null_actor_audit_logs", e))?;

    sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_sessions", e))?;

    sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_user", e))?;

    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("delete_account_commit", e))?;

    crate::security::clear_session_cookie();
    Ok(())
}

/// Lista todos os usuários cadastrados (visão de admin), para gestão de bolões.
#[allow(dead_code)]
#[cfg(feature = "server")]
pub async fn list_all_users(
    token: String,
) -> Result<Vec<crate::models::UserPublic>, ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    require_admin(&token).await?;

    let rows: Vec<UserPublicRow> = sqlx::query_as(
        "SELECT id, username, email, is_admin, blocked_at, blocked_reason FROM users ORDER BY username COLLATE NOCASE",
    )
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_all_users", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, username, email, is_admin, blocked_at, blocked_reason)| crate::models::UserPublic {
                id,
                username,
                email,
                is_admin,
                blocked_at,
                blocked_reason,
            },
        )
        .collect())
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

/// Passo 1 do cadastro: valida os dados, guarda um cadastro pendente e envia
/// um codigo de verificacao por email. A conta so e criada apos `confirm_registration`.
#[cfg(feature = "server")]
pub async fn request_registration(
    username: String,
    email: String,
    password: String,
) -> Result<(), ServerFnError> {
    use crate::db::pool;
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:register:ip:{ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 5,
        },
        blocked_event: "rate_limit_triggered_register_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
        }),
    })
    .await?;

    let (username, username_lookup, email) =
        validate_registration_input(username, email, &password)?;

    let db = pool();

    if user_exists_by_identity(db, &username_lookup, &email).await? {
        return Err(crate::security::public_error(
            "Usuario ou email ja cadastrado.",
        ));
    }

    let password_hash = hash_password(&password)?;
    let code = crate::security::verification_code();
    let code_hash = crate::security::hash_code(&code);
    let expires_at = sqlite_utc_after_minutes(EMAIL_CODE_TTL_MINUTES);

    sqlx::query(
        "INSERT INTO pending_registrations
            (email, username, username_lookup, password_hash, code_hash, attempts, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
         ON CONFLICT(email) DO UPDATE SET
            username = excluded.username,
            username_lookup = excluded.username_lookup,
            password_hash = excluded.password_hash,
            code_hash = excluded.code_hash,
            attempts = 0,
            expires_at = excluded.expires_at,
            created_at = datetime('now')",
    )
    .bind(&email)
    .bind(&username)
    .bind(&username_lookup)
    .bind(&password_hash)
    .bind(&code_hash)
    .bind(&expires_at)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("request_registration_upsert", e))?;

    crate::email::send_verification_code(&email, &code).await?;

    crate::security::log_event(
        "register_code_sent",
        serde_json::json!({
            "email_hash": crate::security::sensitive_value_hash(&email),
            "ip": ip
        }),
    );

    Ok(())
}

/// Passo 2 do cadastro: confere o codigo, cria a conta de fato e inicia a sessao.
#[cfg(feature = "server")]
pub async fn confirm_registration(
    email: String,
    code: String,
) -> Result<AuthResult, ServerFnError> {
    use crate::db::pool;
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:register_confirm:ip:{ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 10,
        },
        blocked_event: "rate_limit_triggered_register_confirm_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
        }),
    })
    .await?;

    let email = crate::security::normalize_email(email)?;
    let db = pool();

    let pending: Option<(String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT username, username_lookup, password_hash, code_hash, attempts, expires_at
         FROM pending_registrations WHERE email = ?1",
    )
    .bind(&email)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("confirm_registration_lookup", e))?;

    let Some((username, username_lookup, password_hash, code_hash, attempts, expires_at)) = pending
    else {
        return Err(crate::security::public_error(
            "Codigo invalido ou expirado. Solicite um novo cadastro.",
        ));
    };

    if is_email_code_expired(&expires_at, attempts) {
        sqlx::query("DELETE FROM pending_registrations WHERE email = ?1")
            .bind(&email)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("confirm_registration_expire", e))?;
        return Err(crate::security::public_error(
            "Codigo invalido ou expirado. Solicite um novo cadastro.",
        ));
    }

    if crate::security::hash_code(&code) != code_hash {
        register_email_code_attempt(db, "pending_registrations", &email).await?;
        return Err(crate::security::public_error("Codigo invalido."));
    }

    // Corrida: o username/email pode ter sido cadastrado entre os dois passos.
    if user_exists_by_identity(db, &username_lookup, &email).await? {
        sqlx::query("DELETE FROM pending_registrations WHERE email = ?1")
            .bind(&email)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("confirm_registration_cleanup", e))?;
        return Err(crate::security::public_error(
            "Usuario ou email ja cadastrado.",
        ));
    }

    let user_id = insert_user_account(db, &username, &email, &password_hash, false).await?;

    sqlx::query("DELETE FROM pending_registrations WHERE email = ?1")
        .bind(&email)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("confirm_registration_delete", e))?;

    let session = create_session(db, &user_id).await?;
    crate::security::set_session_cookie(&session.token);

    crate::security::append_audit_log(
        db,
        Some(&user_id),
        "register_confirmed",
        "user",
        Some(&user_id),
        Some(&ip),
        serde_json::json!({
            "email_hash": crate::security::sensitive_value_hash(&email)
        }),
    )
    .await?;

    Ok(AuthResult {
        user: UserPublic {
            id: user_id,
            username,
            email,
            is_admin: false,
            blocked_at: None,
            blocked_reason: None,
        },
        token: String::new(),
        csrf_token: session.csrf_token,
    })
}

/// Passo 1 do reset: gera e envia um codigo por email. Sempre retorna `Ok`
/// (mesmo para email inexistente) para nao revelar quais emails existem.
#[cfg(feature = "server")]
pub async fn request_password_reset(email: String) -> Result<(), ServerFnError> {
    use crate::db::pool;
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:password_reset:ip:{ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 5,
        },
        blocked_event: "rate_limit_triggered_password_reset_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
        }),
    })
    .await?;

    let email = crate::security::normalize_email(email)?;
    let db = pool();

    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE lower(email) = ?1")
        .bind(&email)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("password_reset_lookup", e))?;

    let Some((user_id,)) = user else {
        crate::security::log_event(
            "password_reset_unknown_email",
            serde_json::json!({
                "email_hash": crate::security::sensitive_value_hash(&email),
                "ip": ip
            }),
        );
        return Ok(());
    };

    let code = crate::security::verification_code();
    let code_hash = crate::security::hash_code(&code);
    let expires_at = sqlite_utc_after_minutes(EMAIL_CODE_TTL_MINUTES);

    sqlx::query(
        "INSERT INTO password_reset_codes
            (email, user_id, code_hash, attempts, expires_at)
         VALUES (?1, ?2, ?3, 0, ?4)
         ON CONFLICT(email) DO UPDATE SET
            user_id = excluded.user_id,
            code_hash = excluded.code_hash,
            attempts = 0,
            expires_at = excluded.expires_at,
            created_at = datetime('now')",
    )
    .bind(&email)
    .bind(&user_id)
    .bind(&code_hash)
    .bind(&expires_at)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("password_reset_upsert", e))?;

    crate::email::send_password_reset_code(&email, &code).await?;

    crate::security::log_event(
        "password_reset_code_sent",
        serde_json::json!({ "user_id": user_id, "ip": ip }),
    );

    Ok(())
}

/// Passo 2 do reset: confere o codigo, troca a senha e invalida sessoes antigas.
#[cfg(feature = "server")]
pub async fn confirm_password_reset(
    email: String,
    code: String,
    new_password: String,
) -> Result<(), ServerFnError> {
    use crate::db::pool;
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:password_reset_confirm:ip:{ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 10,
        },
        blocked_event: "rate_limit_triggered_password_reset_confirm_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
        }),
    })
    .await?;

    if new_password.len() < 8 || new_password.len() > 128 {
        return Err(crate::security::public_error(
            "A senha deve ter entre 8 e 128 caracteres.",
        ));
    }

    let email = crate::security::normalize_email(email)?;
    let db = pool();

    let row: Option<(String, String, i64, String)> = sqlx::query_as(
        "SELECT user_id, code_hash, attempts, expires_at
         FROM password_reset_codes WHERE email = ?1",
    )
    .bind(&email)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("password_reset_confirm_lookup", e))?;

    let Some((user_id, code_hash, attempts, expires_at)) = row else {
        return Err(crate::security::public_error(
            "Codigo invalido ou expirado. Solicite um novo.",
        ));
    };

    if is_email_code_expired(&expires_at, attempts) {
        sqlx::query("DELETE FROM password_reset_codes WHERE email = ?1")
            .bind(&email)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("password_reset_confirm_expire", e))?;
        return Err(crate::security::public_error(
            "Codigo invalido ou expirado. Solicite um novo.",
        ));
    }

    if crate::security::hash_code(&code) != code_hash {
        register_email_code_attempt(db, "password_reset_codes", &email).await?;
        return Err(crate::security::public_error("Codigo invalido."));
    }

    let password_hash = hash_password(&new_password)?;
    sqlx::query("UPDATE users SET password_hash = ?1 WHERE id = ?2")
        .bind(&password_hash)
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("password_reset_update", e))?;

    sqlx::query("DELETE FROM password_reset_codes WHERE email = ?1")
        .bind(&email)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("password_reset_confirm_delete", e))?;

    invalidate_user_sessions(db, &user_id).await?;

    crate::security::append_audit_log(
        db,
        Some(&user_id),
        "password_reset",
        "user",
        Some(&user_id),
        Some(&ip),
        serde_json::json!({
            "email_hash": crate::security::sensitive_value_hash(&email)
        }),
    )
    .await?;

    Ok(())
}

/// Verifica se um codigo de email ja expirou (por tempo ou por excesso de tentativas).
#[cfg(feature = "server")]
fn is_email_code_expired(expires_at: &str, attempts: i64) -> bool {
    if attempts >= EMAIL_CODE_MAX_ATTEMPTS {
        return true;
    }
    match parsed_sqlite_utc(expires_at) {
        Some(stamp) => chrono::Utc::now() > stamp,
        None => true,
    }
}

/// Incrementa o contador de tentativas de um codigo de email na tabela informada.
#[cfg(feature = "server")]
async fn register_email_code_attempt(
    db: &sqlx::SqlitePool,
    table: &str,
    email: &str,
) -> Result<(), ServerFnError> {
    let sql = format!("UPDATE {table} SET attempts = attempts + 1 WHERE email = ?1");
    sqlx::query(&sql)
        .bind(email)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("register_email_code_attempt", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn cleanup_expired_auth_data(
    db: &sqlx::SqlitePool,
) -> Result<AuthCleanupSummary, ServerFnError> {
    let expired_sessions_deleted = sqlx::query(
        "DELETE FROM sessions
         WHERE datetime(expires_at) <= datetime('now')",
    )
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("cleanup_expired_sessions", e))?
    .rows_affected();

    let expired_pending_registrations_deleted = sqlx::query(
        "DELETE FROM pending_registrations
         WHERE attempts >= ?1
            OR datetime(expires_at) <= datetime('now')
            OR datetime(created_at) <= datetime('now', '-1 day')",
    )
    .bind(EMAIL_CODE_MAX_ATTEMPTS)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("cleanup_expired_pending_registrations", e))?
    .rows_affected();

    let expired_password_reset_codes_deleted = sqlx::query(
        "DELETE FROM password_reset_codes
         WHERE attempts >= ?1
            OR datetime(expires_at) <= datetime('now')
            OR datetime(created_at) <= datetime('now', '-1 day')",
    )
    .bind(EMAIL_CODE_MAX_ATTEMPTS)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("cleanup_expired_password_reset_codes", e))?
    .rows_affected();

    Ok(AuthCleanupSummary {
        expired_sessions_deleted,
        expired_pending_registrations_deleted,
        expired_password_reset_codes_deleted,
    })
}

#[cfg(feature = "server")]
pub async fn run_bootstrap_admin(
    username: String,
    email: String,
    password: String,
    bootstrap_secret: String,
) -> Result<UserPublic, ServerFnError> {
    use crate::db::pool;

    let (username, username_lookup, email) =
        validate_registration_input(username, email, &password)?;

    let db = pool();

    let user_id = create_bootstrap_admin_account(
        db,
        &username,
        &username_lookup,
        &email,
        &password,
        &bootstrap_secret,
        "local-bootstrap",
    )
    .await?;

    crate::security::log_event(
        "bootstrap_admin_success",
        serde_json::json!({
            "user_id": user_id,
            "ip": "local-bootstrap",
        }),
    );

    Ok(UserPublic {
        id: user_id,
        username,
        email,
        is_admin: true,
        blocked_at: None,
        blocked_reason: None,
    })
}

#[cfg(feature = "server")]
pub async fn login(username: String, password: String) -> Result<AuthResult, ServerFnError> {
    use crate::db::pool;
    use argon2::{PasswordHash, PasswordVerifier};
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

    let login_identifier = crate::security::normalize_optional_text(username, 120)?.to_lowercase();
    let ip = crate::security::client_ip(&headers);
    let identity_hash = crate::security::rate_limit_identity_hash(&login_identifier);

    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:login:ip:{ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 10,
        },
        blocked_event: "rate_limit_triggered_login_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
        }),
    })
    .await?;
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:login:identity:{identity_hash}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 5,
        },
        blocked_event: "rate_limit_triggered_login_identity",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
            "identity_hash": identity_hash,
        }),
    })
    .await?;

    if password.len() > 128 {
        return Err(crate::security::public_error("Usuario ou senha invalidos."));
    }

    let db = pool();

    let row: Option<LoginRow> = sqlx::query_as(
        "SELECT id, username, email, password_hash, is_admin, blocked_at, blocked_reason
         FROM users
         WHERE lower(username) = ?1 OR lower(email) = ?1",
    )
    .bind(&login_identifier)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("login_lookup_user", e))?;

    let Some((id, username, email, password_hash, is_admin, blocked_at, blocked_reason)) = row else {
        crate::security::log_event(
            "login_failed",
            serde_json::json!({
                "reason": "missing_user",
                "login_identifier_hash": crate::security::sensitive_value_hash(&login_identifier),
                "ip": ip,
            }),
        );
        return Err(crate::security::public_error("Usuario ou senha invalidos."));
    };

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|e| crate::security::internal_error("login_parse_hash", e))?;

    if argon2_policy()
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

    if blocked_at.is_some() {
        return Err(crate::security::public_error(
            blocked_reason.unwrap_or_else(|| "Sua conta esta bloqueada.".to_string()),
        ));
    }

    if needs_rehash(&parsed_hash) {
        match hash_password(&password) {
            Ok(new_hash) => {
                match sqlx::query("UPDATE users SET password_hash = ?1 WHERE id = ?2")
                    .bind(&new_hash)
                    .bind(&id)
                    .execute(db)
                    .await
                {
                    Ok(_) => {
                        crate::security::log_event(
                            "password_rehashed",
                            serde_json::json!({
                                "user_id": id,
                                "policy_version": crate::config::settings().argon2_policy_version,
                            }),
                        );
                    }
                    Err(e) => {
                        crate::security::internal_error("login_rehash_update", e);
                    }
                }
            }
            Err(e) => {
                crate::security::internal_error("login_rehash_compute", e);
            }
        }
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
            blocked_at,
            blocked_reason,
        },
        token: String::new(),
        csrf_token: session.csrf_token,
    })
}

#[cfg(feature = "server")]
pub async fn confirm_admin_password(
    password: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::db::pool;
    use argon2::{PasswordHash, PasswordVerifier};
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:reauth:ip:{ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 8,
        },
        blocked_event: "rate_limit_triggered_reauth_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
        }),
    })
    .await?;

    let session = require_admin("").await?;
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:reauth:user:{}", session.user_id),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 6,
        },
        blocked_event: "rate_limit_triggered_reauth_user",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "client_ip": ip,
            "user_id": session.user_id.clone(),
        }),
    })
    .await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();
    let row: (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("confirm_admin_password_lookup", e))?;

    let parsed_hash = PasswordHash::new(&row.0)
        .map_err(|e| crate::security::internal_error("confirm_admin_password_parse", e))?;

    if argon2_policy()
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
        return Err(crate::security::public_error(
            "Senha de administrador invalida.",
        ));
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

#[cfg(feature = "server")]
pub async fn logout(token: String, csrf_token: String) -> Result<(), ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

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

#[cfg(feature = "server")]
pub async fn current_user(token: String) -> Result<SessionState, ServerFnError> {
    use crate::db::pool;
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::enforce_trusted_proxy(&headers)?;

    let ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:current_user:ip:{ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(30),
            max_attempts: 30,
        },
        blocked_event: "rate_limit_triggered_current_user_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailOpen,
        audit_fields: serde_json::json!({
            "client_ip": ip,
        }),
    })
    .await?;

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
        std::env::set_var("APP_ENV", "test");
        std::env::set_var("DATABASE_PATH", "test.db");
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
