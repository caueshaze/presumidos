use super::*;
use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub async fn delete_pool(
    token: String,
    pool_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();
    require_pool_manager(db, &pool_id, &session.user_id).await?;

    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("delete_pool_begin_tx", e))?;

    sqlx::query("DELETE FROM point_adjustments WHERE pool_id = ?1")
        .bind(&pool_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_pool_adjustments", e))?;

    sqlx::query("DELETE FROM pool_members WHERE pool_id = ?1")
        .bind(&pool_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_pool_members", e))?;

    sqlx::query("DELETE FROM pools WHERE id = ?1")
        .bind(&pool_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("delete_pool_pool", e))?;

    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("delete_pool_commit", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "pool_deleted",
        "pool",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({}),
    )
    .await?;

    Ok(())
}
