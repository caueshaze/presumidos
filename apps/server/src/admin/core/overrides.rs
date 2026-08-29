use super::*;
use crate::{error::ServerFnError, models::*};

pub async fn reopen_prediction(
    token: String,
    match_id: String,
    user_id: String,
    reason: String,
    expires_at: String,
    csrf_token: String,
) -> Result<PredictionReopenOverride, ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    crate::security::validate_uuid("Usuario", &user_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();

    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM prediction_admin_overrides
         WHERE match_id = ?1 AND user_id = ?2
           AND revoked_at IS NULL
           AND used_at IS NULL
           AND datetime(expires_at) > datetime('now')",
    )
    .bind(&match_id)
    .bind(&user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("reopen_prediction_existing", e))?;
    if existing.is_some() {
        return Err(crate::security::public_error(
            "Ja existe uma reabertura ativa para esse palpite.",
        ));
    }

    let override_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO prediction_admin_overrides
            (id, match_id, user_id, reason, reopened_by, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&override_id)
    .bind(&match_id)
    .bind(&user_id)
    .bind(&reason)
    .bind(&session.user_id)
    .bind(&expires_at)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("reopen_prediction_insert", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "prediction_reopened",
        "prediction_override",
        Some(&override_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "match_id": match_id,
            "user_id": user_id,
            "expires_at": expires_at,
        }),
    )
    .await?;

    Ok(PredictionReopenOverride {
        id: override_id,
        match_id,
        user_id,
        reason,
        reopened_by: session.user_id,
        expires_at,
        used_at: None,
        created_at: String::new(),
        revoked_at: None,
    })
}

#[cfg(feature = "server")]
pub async fn revoke_prediction_reopen(
    token: String,
    override_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Reabertura", &override_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    sqlx::query(
        "UPDATE prediction_admin_overrides
         SET revoked_at = datetime('now')
         WHERE id = ?1",
    )
    .bind(&override_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("revoke_prediction_reopen", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "prediction_reopen_revoked",
        "prediction_override",
        Some(&override_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn active_prediction_override(
    match_id: &str,
    user_id: &str,
) -> Result<Option<PredictionReopenOverride>, ServerFnError> {
    let db = crate::db::pool();
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
        ),
    >(
        "SELECT id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at
         FROM prediction_admin_overrides
         WHERE match_id = ?1 AND user_id = ?2
           AND revoked_at IS NULL
           AND datetime(expires_at) > datetime('now')
         ORDER BY datetime(created_at) DESC
         LIMIT 1",
    )
    .bind(match_id)
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("active_prediction_override", e))?;

    Ok(row.map(
        |(id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at)| {
            PredictionReopenOverride {
                id,
                match_id,
                user_id: user_id.to_string(),
                reason,
                reopened_by,
                expires_at,
                used_at,
                created_at,
                revoked_at,
            }
        },
    ))
}

/// Reaberturas ativas do usuario autenticado (usado pela tela de palpites para
/// liberar o formulario mesmo apos o travamento padrao por horario).
#[cfg(feature = "server")]
pub async fn list_my_prediction_overrides(
    token: String,
) -> Result<Vec<PredictionReopenOverride>, ServerFnError> {
    use crate::auth::require_user;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    let db = crate::db::pool();

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
        ),
    >(
        "SELECT id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at
         FROM prediction_admin_overrides
         WHERE user_id = ?1
           AND revoked_at IS NULL
           AND datetime(expires_at) > datetime('now')
         ORDER BY datetime(created_at) DESC",
    )
    .bind(&session.user_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_my_prediction_overrides", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, reason, reopened_by, expires_at, created_at, used_at, match_id, revoked_at)| {
                PredictionReopenOverride {
                    id,
                    match_id,
                    user_id: session.user_id.clone(),
                    reason,
                    reopened_by,
                    expires_at,
                    used_at,
                    created_at,
                    revoked_at,
                }
            },
        )
        .collect())
}

#[cfg(feature = "server")]
pub async fn mark_prediction_override_used(override_id: &str) -> Result<(), ServerFnError> {
    let db = crate::db::pool();
    sqlx::query(
        "UPDATE prediction_admin_overrides
         SET used_at = COALESCE(used_at, datetime('now'))
         WHERE id = ?1",
    )
    .bind(override_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("mark_prediction_override_used", e))?;
    Ok(())
}
