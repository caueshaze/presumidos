use super::super::*;

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
