use super::super::*;
use super::time::*;
pub(crate) fn admin_reauth_is_fresh(value: Option<&str>) -> bool {
    let ttl = chrono::Duration::minutes(crate::config::settings().admin_reauth_ttl_minutes);
    value
        .and_then(parsed_sqlite_utc)
        .is_some_and(|stamp| chrono::Utc::now() - stamp <= ttl)
}

pub(crate) fn can_bootstrap_admin(
    has_any_admin: bool,
    provided_secret: &str,
    expected_secret: &str,
) -> bool {
    !has_any_admin
        && !provided_secret.trim().is_empty()
        && provided_secret.trim() == expected_secret.trim()
}
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
    let expected_bootstrap_secret = crate::config::settings().admin_bootstrap_secret.clone();
    create_bootstrap_admin_account_with_expected_secret(
        db,
        username,
        username_lookup,
        email,
        password,
        bootstrap_secret,
        &expected_bootstrap_secret,
        ip,
    )
    .await
}

pub(crate) async fn create_bootstrap_admin_account_with_expected_secret(
    db: &sqlx::SqlitePool,
    username: &str,
    username_lookup: &str,
    email: &str,
    password: &str,
    bootstrap_secret: &str,
    expected_bootstrap_secret: &str,
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

    if !can_bootstrap_admin(has_any_admin, bootstrap_secret, expected_bootstrap_secret) {
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
