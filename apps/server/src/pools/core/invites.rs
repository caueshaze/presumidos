use super::*;
use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub async fn public_invite_preview(
    invite_code: String,
) -> Result<PublicPoolInvitePreview, ServerFnError> {
    use std::time::Duration;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    let client_ip = crate::security::client_ip(&headers);
    crate::security::enforce_rate_limit(crate::security::RateLimitRequest {
        key: format!("rl:invite_preview:ip:{client_ip}"),
        rule: crate::security::RateLimitRule {
            window: Duration::from_secs(60),
            max_attempts: 60,
        },
        blocked_event: "rate_limit_triggered_invite_preview_ip",
        failure_policy: crate::security::RateLimitFailurePolicy::FailOpen,
        audit_fields: serde_json::json!({ "client_ip": client_ip }),
    })
    .await?;
    crate::operability::metrics()
        .invite_preview
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let invite_code = match normalize_invite_code(invite_code) {
        Ok(value) => value,
        Err(_) => {
            return Ok(PublicPoolInvitePreview {
                pool_name: None,
                event_name: None,
                event_description: None,
                cover_asset_url: None,
                cover_url: None,
                creator_display_name: None,
                member_count: None,
                lock_deadline: None,
                join_status: "invalid".to_string(),
                pool_id: None,
                predictions_closed_at: None,
                closed_at: None,
            });
        }
    };

    let db = crate::db::pool();
    let Some(invite) = resolve_invite(db, &invite_code).await? else {
        return Ok(PublicPoolInvitePreview {
            pool_name: None,
            event_name: None,
            event_description: None,
            cover_asset_url: None,
            cover_url: None,
            creator_display_name: None,
            member_count: None,
            lock_deadline: None,
            join_status: "invalid".to_string(),
            pool_id: None,
            predictions_closed_at: None,
            closed_at: None,
        });
    };

    let ended = invite.event_status == "finished"
        || invite
            .event_ends_at
            .as_deref()
            .is_some_and(timestamp_elapsed);
    let closed = invite.join_closed_at.is_some()
        || invite.predictions_closed_at.is_some()
        || invite.closed_at.is_some()
        || ended;
    let user_id = crate::auth::current_user(String::new())
        .await?
        .user
        .map(|u| u.id);
    let already_member = if let Some(user_id) = &user_id {
        sqlx::query_as::<_, (i64,)>(
            "SELECT EXISTS(SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?2)",
        )
        .bind(&invite.pool_id)
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("invite_preview_membership", e))?
        .0 != 0
    } else {
        false
    };
    let can_rejoin_ended = if ended {
        if let Some(user_id) = user_id.as_deref() {
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
            .bind(user_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("invite_preview_rejoin", e))?
            .0 != 0
        } else {
            false
        }
    } else {
        false
    };

    Ok(PublicPoolInvitePreview {
        pool_name: Some(invite.pool_name),
        event_name: Some(invite.event_name),
        event_description: invite.event_description,
        cover_asset_url: invite
            .cover_asset_id
            .map(|asset_id| format!("/media/assets/{asset_id}/cover")),
        cover_url: invite.cover_url,
        creator_display_name: Some(invite.creator_display_name),
        member_count: Some(invite.member_count),
        lock_deadline: invite.lock_deadline,
        join_status: if already_member {
            "already_member"
        } else if invite.join_closed_at.is_some()
            || invite.predictions_closed_at.is_some()
            || invite.closed_at.is_some()
            || (closed && !can_rejoin_ended)
        {
            "closed"
        } else {
            "joinable"
        }
        .to_string(),
        pool_id: already_member.then_some(invite.pool_id),
        predictions_closed_at: invite.predictions_closed_at,
        closed_at: invite.closed_at,
    })
}
