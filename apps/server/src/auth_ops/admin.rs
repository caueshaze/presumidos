use super::super::*;
use super::guards::require_admin;

/// Lista todos os usuários cadastrados (visão de admin), para gestão de bolões.
#[allow(dead_code)]
#[cfg(feature = "server")]
pub async fn list_all_users(
    token: String,
) -> Result<Vec<crate::models::UserPublic>, ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    require_admin(&token).await?;

    let rows: Vec<UserPublicRow> = sqlx::query_as(
        "SELECT id, username, email, is_admin, blocked_at, blocked_reason FROM users ORDER BY username COLLATE NOCASE",
    )
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_all_users", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, username, email, is_admin, blocked_at, blocked_reason)| {
                crate::models::UserPublic {
                    id,
                    username,
                    email,
                    is_admin,
                    blocked_at,
                    blocked_reason,
                }
            },
        )
        .collect())
}
