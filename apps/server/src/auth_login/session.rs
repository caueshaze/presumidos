use super::super::*;

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
