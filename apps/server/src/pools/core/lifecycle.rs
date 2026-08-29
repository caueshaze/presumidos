use super::*;
use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub async fn create_pool_for_event(
    token: String,
    name: String,
    requested_event_id: Option<String>,
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
    let event_id = match requested_event_id {
        Some(id) => {
            let allowed: Option<(String, String)> = sqlx::query_as("SELECT id, current_published_version_id FROM events WHERE id=?1 AND status='active' AND pool_creation_enabled=1 AND current_published_version_id IS NOT NULL AND (ends_at IS NULL OR datetime(ends_at) > datetime('now')) AND (kind='football' OR kind='custom')")
                .bind(&id).fetch_optional(db).await.map_err(|e| crate::security::internal_error("create_pool_event_allowed", e))?;
            allowed.map(|v| v.0).ok_or_else(|| {
                crate::security::public_error("Evento indisponível para criar bolão.")
            })?
        }
        None => {
            return Err(crate::security::public_error(
                "Escolha um evento publicado para criar o bolão.",
            ));
        }
    };
    let pool_id = Uuid::new_v4().to_string();
    let invite_code = generate_invite_code(db).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("create_pool_begin_tx", e))?;

    let event_version_id: (String,) = sqlx::query_as("SELECT current_published_version_id FROM events WHERE id=?1 AND current_published_version_id IS NOT NULL")
        .bind(&event_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("create_pool_event_version", e))?;
    sqlx::query("INSERT INTO pools (id, event_id, event_version_id, name, invite_code, created_by) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .bind(&pool_id)
        .bind(&event_id)
        .bind(&event_version_id.0)
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

    let event: (String, String, String, String, Option<String>) =
        sqlx::query_as("SELECT v.name, e.slug, e.kind, e.status, e.ends_at FROM events e JOIN event_versions v ON v.id=?2 WHERE e.id=?1")
            .bind(&event_id)
            .bind(&event_version_id.0)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("create_pool_event", e))?;
    Ok(PoolSummary {
        id: pool_id,
        event_id: event_id.clone(),
        event: event_summary(event_id, event.0, event.1, event.2, event.3, event.4),
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

    let invite_code = normalize_invite_code(invite_code)?;
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

    let Some(invite) = resolve_invite(db, &invite_code).await? else {
        return Err(crate::security::public_error("Codigo de convite invalido."));
    };

    if invite.join_closed_at.is_some() {
        return Err(crate::security::public_error(
            "Este bolao esta fechado para novos participantes.",
        ));
    }
    let ended = invite.event_status == "finished"
        || invite
            .event_ends_at
            .as_deref()
            .is_some_and(timestamp_elapsed);
    let can_rejoin_ended = if ended {
        sqlx::query_as::<_, (i64,)>(
            "SELECT EXISTS(
                SELECT 1 FROM audit_logs
                WHERE action = 'pool_member_left'
                  AND target_type = 'pool'
                  AND target_id = ?1
                  AND actor_user_id = ?2
            )",
        )
        .bind(&invite.pool_id)
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("join_pool_rejoin_check", e))?
        .0 != 0
    } else {
        false
    };
    if ended && !can_rejoin_ended {
        return Err(crate::security::public_error(
            "Este bolão pertence a uma edição encerrada.",
        ));
    }

    sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(&invite.pool_id)
        .bind(&session.user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("join_pool_insert_member", e))?;

    let _ = crate::scoring::recalculate_pool_user_breakdowns(
        &invite.pool_id,
        &session.user_id,
        Some(&session.user_id),
    )
    .await?;

    crate::operability::metrics()
        .invite_join_success
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let member_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id = ?1")
            .bind(&invite.pool_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("join_pool_count_members", e))?;

    Ok(PoolSummary {
        id: invite.pool_id,
        event: event_summary(
            invite.event_id.clone(),
            invite.event_name,
            invite.event_slug,
            invite.event_kind,
            invite.event_status,
            invite.event_ends_at,
        ),
        event_id: invite.event_id,
        name: invite.pool_name,
        invite_code,
        member_count: member_count.0,
        created_by: invite.created_by,
        description: invite.description,
        visible_rules: invite.visible_rules,
        join_closed_at: invite.join_closed_at,
    })
}

/// Remove a membership without deleting the member's pool-scoped predictions.
#[cfg(feature = "server")]
pub async fn leave_pool(
    token: String,
    pool_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = pool();

    let membership: Option<(String,)> = sqlx::query_as(
        "SELECT p.created_by
         FROM pools p
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE p.id = ?1 AND pm.user_id = ?2",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("leave_pool_membership", e))?;
    let Some((created_by,)) = membership else {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    };
    if created_by == session.user_id {
        return Err(crate::security::public_error(
            "O dono nao pode sair. Exclua o bolao pelas opcoes.",
        ));
    }

    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("leave_pool_begin_tx", e))?;
    sqlx::query("DELETE FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
        .bind(&pool_id)
        .bind(&session.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("leave_pool_delete_membership", e))?;
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, 'pool_member_left', 'pool', ?3, ?4, ?5)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&session.user_id)
    .bind(&pool_id)
    .bind(crate::security::client_ip(&headers))
    .bind(serde_json::json!({ "preserved_pool_data": true }).to_string())
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("leave_pool_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("leave_pool_commit", e))?;
    Ok(())
}
