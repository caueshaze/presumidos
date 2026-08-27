use super::*;

#[cfg(feature = "server")]

pub(crate) fn sqlite_utc_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn sqlite_utc_after_hours(hours: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::hours(hours))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(feature = "server")]
pub(crate) fn sqlite_utc_after_minutes(minutes: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::minutes(minutes))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Validade dos codigos de verificacao por email, em minutos.
#[cfg(feature = "server")]
pub(crate) const EMAIL_CODE_TTL_MINUTES: i64 = 15;

/// Numero maximo de tentativas de digitacao de um codigo antes de exigir um novo envio.
#[cfg(feature = "server")]
pub(crate) const EMAIL_CODE_MAX_ATTEMPTS: i64 = 5;

#[cfg(feature = "server")]
pub(crate) type UserPublicRow = (String, String, String, bool, Option<String>, Option<String>);

#[cfg(feature = "server")]
pub(crate) type LoginRow = (
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
);

#[cfg(feature = "server")]
pub(crate) fn parsed_sqlite_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
}

#[cfg(feature = "server")]
pub(crate) async fn insert_audit_log(
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
pub(crate) async fn insert_audit_log_tx(
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
pub(crate) async fn delete_session_by_token(
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
pub(crate) async fn invalidate_user_sessions(
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
pub(crate) async fn create_session(
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
pub(crate) async fn touch_session(
    db: &sqlx::SqlitePool,
    session: &AuthSession,
) -> Result<(), ServerFnError> {
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
pub(crate) async fn resolve_session(
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
pub(crate) async fn load_user_public(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<UserPublic, ServerFnError> {
    let row: UserPublicRow = sqlx::query_as(
        "SELECT id, username, email, is_admin, blocked_at, blocked_reason FROM users WHERE id = ?1",
    )
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
pub(crate) fn admin_reauth_is_fresh(value: Option<&str>) -> bool {
    let ttl = chrono::Duration::minutes(crate::config::settings().admin_reauth_ttl_minutes);
    value
        .and_then(parsed_sqlite_utc)
        .is_some_and(|stamp| chrono::Utc::now() - stamp <= ttl)
}

#[cfg(feature = "server")]
pub(crate) fn can_bootstrap_admin(
    has_any_admin: bool,
    provided_secret: &str,
    expected_secret: &str,
) -> bool {
    !has_any_admin
        && !provided_secret.trim().is_empty()
        && provided_secret.trim() == expected_secret.trim()
}

#[cfg(feature = "server")]
pub(crate) async fn user_exists_by_identity(
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
pub(crate) async fn count_admins(db: &sqlx::SqlitePool) -> Result<i64, ServerFnError> {
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
pub(crate) async fn create_public_user_account(
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
pub(crate) async fn create_bootstrap_admin_account(
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
pub(crate) fn validate_registration_input(
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
pub(crate) fn argon2_policy() -> argon2::Argon2<'static> {
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
pub(crate) fn needs_rehash(parsed_hash: &argon2::password_hash::PasswordHash<'_>) -> bool {
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
