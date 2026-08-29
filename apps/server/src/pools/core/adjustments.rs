use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub(crate) async fn require_pool_manager(
    db: &sqlx::SqlitePool,
    pool_id: &str,
    user_id: &str,
) -> Result<(), ServerFnError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT created_by FROM pools WHERE id = ?1")
        .bind(pool_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("require_pool_manager_pool", e))?;

    let Some((created_by,)) = row else {
        return Err(crate::security::public_error("Bolao nao encontrado."));
    };

    if created_by == user_id {
        return Ok(());
    }

    let is_admin: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?1")
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("require_pool_manager_admin", e))?;

    if is_admin.0 {
        Ok(())
    } else {
        Err(crate::security::public_error(
            "Apenas o organizador do bolao pode ajustar pontos.",
        ))
    }
}

#[cfg(feature = "server")]
async fn require_active_pool(db: &sqlx::SqlitePool, pool_id: &str) -> Result<(), ServerFnError> {
    let active: Option<(String,)> = sqlx::query_as(
        "SELECT p.id FROM pools p JOIN events e ON e.id = p.event_id
         WHERE p.id = ?1 AND e.status = 'active'",
    )
    .bind(pool_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("require_active_pool", e))?;
    if active.is_none() {
        return Err(crate::security::public_error(
            "Esta edição está encerrada e só pode ser consultada.",
        ));
    }
    Ok(())
}

/// Lista os ajustes de pontos de um bolão (visível a qualquer membro, por transparência).
#[cfg(feature = "server")]
pub async fn list_pool_adjustments(
    token: String,
    pool_id: String,
) -> Result<Vec<PointAdjustment>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let session = require_user(&token).await?;
    let db = pool();

    let membership: Option<(String,)> =
        sqlx::query_as("SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("list_pool_adjustments_membership", e))?;
    if membership.is_none() {
        return Err(crate::security::public_error(
            "Voce nao e membro deste bolao.",
        ));
    }

    let rows: Vec<(String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT a.id, a.user_id, u.username, a.delta, a.reason, a.created_at
         FROM point_adjustments a
         JOIN users u ON u.id = a.user_id
         WHERE a.pool_id = ?1
         ORDER BY a.created_at DESC",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_pool_adjustments", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, user_id, username, delta, reason, created_at)| PointAdjustment {
                id,
                user_id,
                username,
                delta,
                reason,
                created_at,
            },
        )
        .collect())
}

/// Lança um ajuste manual de pontos para um membro do bolão.
#[cfg(feature = "server")]
pub async fn add_point_adjustment(
    token: String,
    pool_id: String,
    user_id: String,
    delta: i64,
    reason: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use uuid::Uuid;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    crate::security::validate_uuid("Usuario", &user_id)?;
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();
    require_pool_manager(db, &pool_id, &session.user_id).await?;
    require_active_pool(db, &pool_id).await?;

    if delta == 0 {
        return Err(crate::security::public_error("O ajuste nao pode ser zero."));
    }
    if !(-1000..=1000).contains(&delta) {
        return Err(crate::security::public_error(
            "Ajuste fora do limite permitido (-1000 a 1000).",
        ));
    }
    let reason = crate::security::normalize_optional_text(reason, 200)?;

    // O alvo precisa ser membro do bolão.
    let target_member: Option<(String,)> =
        sqlx::query_as("SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
            .bind(&pool_id)
            .bind(&user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("add_point_adjustment_member", e))?;
    if target_member.is_none() {
        return Err(crate::security::public_error(
            "Esse usuario nao e membro do bolao.",
        ));
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO point_adjustments (id, pool_id, user_id, delta, reason, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&id)
    .bind(&pool_id)
    .bind(&user_id)
    .bind(delta)
    .bind(&reason)
    .bind(&session.user_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("add_point_adjustment_insert", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "point_adjustment_added",
        "pool",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "target_user_id": user_id, "delta": delta, "reason": reason }),
    )
    .await?;

    Ok(())
}

/// Remove um ajuste de pontos previamente lançado.
#[cfg(feature = "server")]
pub async fn remove_point_adjustment(
    token: String,
    pool_id: String,
    adjustment_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    crate::security::validate_uuid("Ajuste", &adjustment_id)?;
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();
    require_pool_manager(db, &pool_id, &session.user_id).await?;
    require_active_pool(db, &pool_id).await?;

    sqlx::query("DELETE FROM point_adjustments WHERE id = ?1 AND pool_id = ?2")
        .bind(&adjustment_id)
        .bind(&pool_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("remove_point_adjustment_delete", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "point_adjustment_removed",
        "pool",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "adjustment_id": adjustment_id }),
    )
    .await?;

    Ok(())
}
