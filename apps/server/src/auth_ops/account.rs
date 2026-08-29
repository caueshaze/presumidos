use super::super::*;
use super::guards::require_user;

/// Troca o nome de usuário da conta autenticada. Mantém as mesmas regras de
/// validação e unicidade (case-insensitive) do cadastro.
#[cfg(feature = "server")]
pub async fn change_username(
    token: String,
    new_username: String,
    csrf_token: String,
) -> Result<crate::models::UserPublic, ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let username = crate::security::normalize_required_text("Usuario", new_username, 3, 32)?;
    let username_lookup = username.to_lowercase();

    let db = pool();

    // Unicidade contra OUTROS usuários (permite ajustar maiúsc./minúsc. do próprio nome).
    let taken: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE lower(username) = ?1 AND id != ?2")
            .bind(&username_lookup)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("change_username_lookup", e))?;
    if taken.is_some() {
        return Err(crate::security::public_error(
            "Esse nome de usuario ja esta em uso.",
        ));
    }

    sqlx::query("UPDATE users SET username = ?1 WHERE id = ?2")
        .bind(&username)
        .bind(&session.user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("change_username_update", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "username_changed",
        "user",
        Some(&session.user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "new_username": username }),
    )
    .await?;

    let row: (String, String, String, bool) =
        sqlx::query_as("SELECT id, username, email, is_admin FROM users WHERE id = ?1")
            .bind(&session.user_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("change_username_fetch", e))?;

    Ok(crate::models::UserPublic {
        id: row.0,
        username: row.1,
        email: row.2,
        is_admin: row.3,
        blocked_at: None,
        blocked_reason: None,
    })
}

/// Exclui a própria conta autenticada e limpa os dados operacionais
/// diretamente vinculados a ela.
///
/// Restrições atuais:
/// - a conta não pode ser a criadora de bolões ainda existentes;
/// - a última conta admin não pode se autoexcluir.
#[cfg(feature = "server")]
pub async fn delete_account(token: String, csrf_token: String) -> Result<(), ServerFnError> {
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();
    let (email, is_admin): (String, bool) =
        sqlx::query_as("SELECT email, is_admin FROM users WHERE id = ?1")
            .bind(&session.user_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("delete_account_load_user", e))?;

    let owned_pools: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE created_by = ?1")
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_count_owned_pools", e))?;
    if owned_pools.0 > 0 {
        return Err(crate::security::public_error(
            "Sua conta ainda criou bolões ativos. Apague os bolões criados por voce antes de excluir a conta.",
        ));
    }

    if is_admin && count_admins(db).await? <= 1 {
        return Err(crate::security::public_error(
            "Nao e possivel excluir a unica conta de administrador.",
        ));
    }

    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("delete_account_begin_tx", e))?;

    insert_audit_log_tx(
        &mut tx,
        None,
        "account_deleted",
        "user",
        Some(&session.user_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "email_hash": crate::security::sensitive_value_hash(&email),
            "self_service": true
        }),
    )
    .await?;

    sqlx::query("DELETE FROM push_reminder_deliveries WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_reminder_deliveries", e))?;

    sqlx::query("DELETE FROM push_subscriptions WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_push_subscriptions", e))?;

    sqlx::query("DELETE FROM notification_preferences WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            crate::security::internal_error("delete_account_notification_preferences", e)
        })?;

    sqlx::query("DELETE FROM prediction_reaction_views WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_reaction_views", e))?;

    sqlx::query(
        "DELETE FROM prediction_reactions
         WHERE target_user_id = ?1 OR reactor_user_id = ?1",
    )
    .bind(&session.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("delete_account_prediction_reactions", e))?;

    sqlx::query("DELETE FROM point_adjustments WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_point_adjustments", e))?;

    sqlx::query("DELETE FROM predictions WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_predictions", e))?;

    sqlx::query("DELETE FROM pool_members WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_pool_members", e))?;

    sqlx::query("DELETE FROM password_reset_codes WHERE user_id = ?1 OR email = ?2")
        .bind(&session.user_id)
        .bind(&email)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_password_reset_codes", e))?;

    sqlx::query("DELETE FROM pending_registrations WHERE email = ?1")
        .bind(&email)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_pending_registrations", e))?;

    sqlx::query("UPDATE audit_logs SET actor_user_id = NULL WHERE actor_user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_null_actor_audit_logs", e))?;

    sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_sessions", e))?;

    sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_account_user", e))?;

    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("delete_account_commit", e))?;

    crate::security::clear_session_cookie();
    Ok(())
}
