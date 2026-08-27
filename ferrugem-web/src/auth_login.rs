use super::*;

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
