use super::super::*;

use super::helpers::*;
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
