use super::super::*;

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
