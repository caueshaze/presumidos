use crate::error::ServerFnError;

use crate::models::{MemberPredictions, PoolSummary, PredictionRecord, UserPublic};

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

/// Palpites dos membros de um bolão, na visão "perfil por membro".
///
/// Por justiça (mesma regra que trava o envio em `submit_prediction`), só
/// retorna palpites de partidas que **já começaram** (`kickoff <= agora`),
/// evitando que um membro copie o palpite alheio antes do jogo. O filtro é
/// feito no servidor. Todos os membros são retornados, mesmo sem palpite
/// visível (lista vazia).
#[cfg(feature = "server")]
pub async fn get_pool_member_predictions(
    token: String,
    pool_id: String,
) -> Result<Vec<MemberPredictions>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use chrono::Utc;
    use std::collections::HashMap;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let session = require_user(&token).await?;
    let db = pool();

    let membership: Option<(String,)> = sqlx::query_as(
        "SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("get_pool_member_predictions_membership", e))?;

    if membership.is_none() {
        return Err(crate::security::public_error("Voce nao e membro deste bolao."));
    }

    // Todos os membros, ordenados por nome (inclui quem ainda não tem palpite visível).
    let members: Vec<(String, String)> = sqlx::query_as(
        "SELECT u.id, u.username
         FROM pool_members pm
         JOIN users u ON u.id = pm.user_id
         WHERE pm.pool_id = ?1
         ORDER BY u.username COLLATE NOCASE",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_pool_member_predictions_members", e))?;

    // Palpites apenas de partidas já iniciadas (kickoff <= agora).
    #[derive(sqlx::FromRow)]
    struct PredRow {
        user_id: String,
        match_id: String,
        home_score: i64,
        away_score: i64,
        qualifier: Option<String>,
        went_to_penalties: bool,
        penalty_home_score: Option<i64>,
        penalty_away_score: Option<i64>,
    }

    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query_as::<_, PredRow>(
        "SELECT pr.user_id AS user_id,
                pr.match_id AS match_id,
                pr.home_score AS home_score,
                pr.away_score AS away_score,
                pr.qualifier AS qualifier,
                pr.went_to_penalties AS went_to_penalties,
                pr.penalty_home_score AS penalty_home_score,
                pr.penalty_away_score AS penalty_away_score
         FROM pool_members pm
         JOIN predictions pr ON pr.user_id = pm.user_id
         JOIN matches m ON m.id = pr.match_id
         WHERE pm.pool_id = ?1 AND datetime(m.kickoff) <= datetime(?2)
         ORDER BY m.kickoff",
    )
    .bind(&pool_id)
    .bind(&now)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_pool_member_predictions_preds", e))?;

    let mut by_user: HashMap<String, Vec<PredictionRecord>> = HashMap::new();
    for row in rows {
        by_user.entry(row.user_id).or_default().push(PredictionRecord {
            match_id: row.match_id,
            home_score: row.home_score,
            away_score: row.away_score,
            qualifier: row.qualifier,
            went_to_penalties: row.went_to_penalties,
            penalty_home_score: row.penalty_home_score,
            penalty_away_score: row.penalty_away_score,
        });
    }

    Ok(members
        .into_iter()
        .map(|(user_id, username)| {
            let predictions = by_user.remove(&user_id).unwrap_or_default();
            MemberPredictions {
                user_id,
                username,
                predictions,
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Administração de bolões (somente admin)
// ---------------------------------------------------------------------------

/// Lista TODOS os bolões existentes (visão de admin), com a contagem de membros.
/// Diferente de `list_my_pools`, não filtra pelos bolões do solicitante.
#[cfg(feature = "server")]
pub async fn list_all_pools_admin(token: String) -> Result<Vec<PoolSummary>, ServerFnError> {
    use crate::auth::require_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    require_admin(&token).await?;

    let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT p.id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm WHERE pm.pool_id = p.id) AS member_count
         FROM pools p
         ORDER BY p.name COLLATE NOCASE",
    )
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_all_pools_admin", e))?;

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

/// Lista os membros de um bolão (visão de admin), independente de o admin
/// participar dele.
#[cfg(feature = "server")]
pub async fn list_pool_members_admin(
    token: String,
    pool_id: String,
) -> Result<Vec<UserPublic>, ServerFnError> {
    use crate::auth::require_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    require_admin(&token).await?;

    let rows: Vec<(String, String, String, bool)> = sqlx::query_as(
        "SELECT u.id, u.username, u.email, u.is_admin
         FROM pool_members pm
         JOIN users u ON u.id = pm.user_id
         WHERE pm.pool_id = ?1
         ORDER BY u.username COLLATE NOCASE",
    )
    .bind(&pool_id)
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_pool_members_admin", e))?;

    Ok(rows
        .into_iter()
        .map(|(id, username, email, is_admin)| UserPublic {
            id,
            username,
            email,
            is_admin,
        })
        .collect())
}

/// Adiciona um usuário a um bolão já existente (visão de admin).
#[cfg(feature = "server")]
pub async fn add_pool_member_admin(
    token: String,
    pool_id: String,
    user_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    crate::security::validate_uuid("Usuario", &user_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();

    let pool_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM pools WHERE id = ?1")
        .bind(&pool_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("add_pool_member_admin_pool_lookup", e))?;
    if pool_exists.is_none() {
        return Err(crate::security::public_error("Bolao nao encontrado."));
    }

    let user_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("add_pool_member_admin_user_lookup", e))?;
    if user_exists.is_none() {
        return Err(crate::security::public_error("Usuario nao encontrado."));
    }

    sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(&pool_id)
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("add_pool_member_admin_insert", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "pool_member_added",
        "pool",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "target_user_id": user_id }),
    )
    .await?;

    Ok(())
}

/// Remove um usuário de um bolão (visão de admin).
#[cfg(feature = "server")]
pub async fn remove_pool_member_admin(
    token: String,
    pool_id: String,
    user_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    crate::security::validate_uuid("Usuario", &user_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    let db = pool();

    sqlx::query("DELETE FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
        .bind(&pool_id)
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("remove_pool_member_admin_delete", e))?;

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "pool_member_removed",
        "pool",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "target_user_id": user_id }),
    )
    .await?;

    Ok(())
}
