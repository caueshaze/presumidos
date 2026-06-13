use crate::error::ServerFnError;

use crate::models::PoolSummary;

#[cfg(feature = "server")]
async fn generate_invite_code(pool: &sqlx::SqlitePool) -> Result<String, ServerFnError> {
    use uuid::Uuid;

    for _ in 0..5 {
        let code = Uuid::new_v4().simple().to_string()[..6].to_uppercase();

        let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM pools WHERE invite_code = ?1")
            .bind(&code)
            .fetch_optional(pool)
            .await
            .map_err(|e| crate::security::internal_error("generate_invite_code_lookup", e))?;

        if exists.is_none() {
            return Ok(code);
        }
    }

    Err(crate::security::public_error(
        "Não foi possível gerar um código de convite. Tente novamente.",
    ))
}

#[cfg(feature = "server")]
pub async fn list_my_pools(token: String) -> Result<Vec<PoolSummary>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;

    let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT p.id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id) AS member_count
         FROM pools p
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE pm.user_id = ?1
         ORDER BY p.created_at DESC",
    )
    .bind(&session.user_id)
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_my_pools", e))?;

    Ok(rows
        .into_iter()
        .map(|(id, name, invite_code, member_count)| PoolSummary {
            id,
            name,
            invite_code,
            member_count,
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn create_pool(
    token: String,
    name: String,
    csrf_token: String,
) -> Result<PoolSummary, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use uuid::Uuid;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let name = crate::security::normalize_required_text("Nome do bolao", name, 3, 80)?;

    let db = pool();
    let pool_id = Uuid::new_v4().to_string();
    let invite_code = generate_invite_code(db).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("create_pool_begin_tx", e))?;

    sqlx::query("INSERT INTO pools (id, name, invite_code, created_by) VALUES (?1, ?2, ?3, ?4)")
        .bind(&pool_id)
        .bind(&name)
        .bind(&invite_code)
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("create_pool_insert_pool", e))?;

    sqlx::query("INSERT INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(&pool_id)
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("create_pool_insert_member", e))?;

    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("create_pool_commit", e))?;

    Ok(PoolSummary {
        id: pool_id,
        name,
        invite_code,
        member_count: 1,
    })
}

#[cfg(feature = "server")]
pub async fn join_pool(
    token: String,
    invite_code: String,
    csrf_token: String,
) -> Result<PoolSummary, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let invite_code = crate::security::normalize_required_text("Codigo de convite", invite_code, 6, 12)?
        .to_uppercase();
    let client_ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:join_pool:ip:{client_ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 12,
        },
        blocked_event: "rate_limit_triggered_join_pool_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailOpen,
        audit_fields: serde_json::json!({
            "client_ip": client_ip,
        }),
    })
    .await?;

    let db = pool();

    let row: Option<(String, String)> = sqlx::query_as("SELECT id, name FROM pools WHERE invite_code = ?1")
        .bind(&invite_code)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("join_pool_lookup", e))?;

    let Some((pool_id, name)) = row else {
        return Err(crate::security::public_error("Codigo de convite invalido."));
    };

    sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(&pool_id)
        .bind(&session.user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("join_pool_insert_member", e))?;

    let member_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id = ?1")
        .bind(&pool_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("join_pool_count_members", e))?;

    Ok(PoolSummary {
        id: pool_id,
        name,
        invite_code,
        member_count: member_count.0,
    })
}
