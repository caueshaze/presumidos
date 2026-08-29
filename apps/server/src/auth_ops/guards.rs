use super::super::*;

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
