use super::*;

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

    // Cooldown por email: evita reenviar codigos (e queimar cota do Resend) para o
    // mesmo endereco dentro do limite por IP.
    let email_hash = crate::security::rate_limit_identity_hash(&email);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:register:email:{email_hash}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 1,
        },
        blocked_event: "rate_limit_triggered_register_email",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "email_hash": email_hash,
        }),
    })
    .await?;
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:register:email_hourly:{email_hash}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(3600),
            max_attempts: 5,
        },
        blocked_event: "rate_limit_triggered_register_email_hourly",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "email_hash": email_hash,
        }),
    })
    .await?;

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

    // Cooldown por email, antes da busca do usuario para nao revelar quais emails
    // existem (mesmo erro generico independe da existencia) e cortar reenvios.
    let email_hash = crate::security::rate_limit_identity_hash(&email);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:password_reset:email:{email_hash}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 1,
        },
        blocked_event: "rate_limit_triggered_password_reset_email",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "email_hash": email_hash,
        }),
    })
    .await?;
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:password_reset:email_hourly:{email_hash}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(3600),
            max_attempts: 5,
        },
        blocked_event: "rate_limit_triggered_password_reset_email_hourly",
        failure_policy: crate::security::RateLimitFailurePolicy::FailClosed,
        audit_fields: serde_json::json!({
            "email_hash": email_hash,
        }),
    })
    .await?;

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
