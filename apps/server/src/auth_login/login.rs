use super::super::*;

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

    let Some((id, username, email, password_hash, is_admin, blocked_at, blocked_reason)) = row
    else {
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
