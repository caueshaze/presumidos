use crate::error::ServerFnError;

use crate::models::{MemberPredictions, PointAdjustment, PoolSummary, PredictionRecord, UserPublic};

#[cfg(feature = "server")]
type PoolSummaryRow = (
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
    Option<String>,
);

#[cfg(feature = "server")]
type PoolMemberUserRow = (String, String, String, bool, Option<String>, Option<String>);

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

    let rows: Vec<PoolSummaryRow> = sqlx::query_as(
        "SELECT p.id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id) AS member_count,
                p.created_by,
                p.description,
                p.visible_rules,
                p.join_closed_at
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
        .map(|(id, name, invite_code, member_count, created_by, description, visible_rules, join_closed_at)| PoolSummary {
            id,
            name,
            invite_code,
            member_count,
            created_by,
            description,
            visible_rules,
            join_closed_at,
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
        created_by: session.user_id,
        description: String::new(),
        visible_rules: String::new(),
        join_closed_at: None,
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

    let row: Option<(String, String, String, String, String, Option<String>)> =
        sqlx::query_as("SELECT id, name, created_by, description, visible_rules, join_closed_at FROM pools WHERE invite_code = ?1")
            .bind(&invite_code)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("join_pool_lookup", e))?;

    let Some((pool_id, name, created_by, description, visible_rules, join_closed_at)) = row else {
        return Err(crate::security::public_error("Codigo de convite invalido."));
    };

    if join_closed_at.is_some() {
        return Err(crate::security::public_error(
            "Este bolao esta fechado para novos participantes.",
        ));
    }

    sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(&pool_id)
        .bind(&session.user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("join_pool_insert_member", e))?;

    let _ = crate::scoring::recalculate_pool_user_breakdowns(&pool_id, &session.user_id, Some(&session.user_id)).await?;

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
        created_by,
        description,
        visible_rules,
        join_closed_at,
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
         WHERE pm.pool_id = ?1
           AND datetime(m.kickoff) <= datetime(?2)
           -- Consistente com o ranking: só palpites de jogos que começaram
           -- depois de o usuário entrar no bolão.
           AND datetime(m.kickoff) >= datetime(pm.joined_at)
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

    let rows: Vec<PoolSummaryRow> = sqlx::query_as(
        "SELECT p.id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm WHERE pm.pool_id = p.id) AS member_count,
                p.created_by,
                p.description,
                p.visible_rules,
                p.join_closed_at
         FROM pools p
         ORDER BY p.name COLLATE NOCASE",
    )
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_all_pools_admin", e))?;

    Ok(rows
        .into_iter()
        .map(|(id, name, invite_code, member_count, created_by, description, visible_rules, join_closed_at)| PoolSummary {
            id,
            name,
            invite_code,
            member_count,
            created_by,
            description,
            visible_rules,
            join_closed_at,
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

    let rows: Vec<PoolMemberUserRow> = sqlx::query_as(
        "SELECT u.id, u.username, u.email, u.is_admin, u.blocked_at, u.blocked_reason
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
        .map(|(id, username, email, is_admin, blocked_at, blocked_reason)| UserPublic {
            id,
            username,
            email,
            is_admin,
            blocked_at,
            blocked_reason,
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

    let _ = crate::scoring::recalculate_pool_user_breakdowns(&pool_id, &user_id, Some(&session.user_id)).await?;

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
    sqlx::query("DELETE FROM prediction_score_breakdowns WHERE pool_id = ?1 AND user_id = ?2")
        .bind(&pool_id)
        .bind(&user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("remove_pool_member_admin_breakdowns", e))?;

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

// ---------------------------------------------------------------------------
// Ajustes manuais de pontos (organizador do bolão ou admin global)
// ---------------------------------------------------------------------------

/// Garante que o usuário é o organizador (criador) do bolão OU um admin global.
#[cfg(feature = "server")]
async fn require_pool_manager(
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
        return Err(crate::security::public_error("Voce nao e membro deste bolao."));
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
        .map(|(id, user_id, username, delta, reason, created_at)| PointAdjustment {
            id,
            user_id,
            username,
            delta,
            reason,
            created_at,
        })
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

    if delta == 0 {
        return Err(crate::security::public_error("O ajuste nao pode ser zero."));
    }
    if !(-1000..=1000).contains(&delta) {
        return Err(crate::security::public_error("Ajuste fora do limite permitido (-1000 a 1000)."));
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
        return Err(crate::security::public_error("Esse usuario nao e membro do bolao."));
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

/// Apaga um bolão (somente o criador ou um admin global).
///
/// Os palpites são globais por usuário (não pertencem ao bolão), então não são
/// tocados. Como o `PRAGMA foreign_keys` não está ligado, os registros filhos
/// (`pool_members`, `point_adjustments`) são apagados explicitamente numa
/// transação para não deixar órfãos.
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
