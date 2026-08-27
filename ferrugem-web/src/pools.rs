use crate::error::ServerFnError;

use crate::models::{
    EventKind, EventStatus, EventSummary, MemberPredictions, PointAdjustment, PoolDashboardSummary,
    PoolPredictionRecord, PoolReport, PoolSummary, PredictionReactionGroup,
    PublicPoolInvitePreview, UserPublic,
};

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct PoolReportRow {
    id: String,
    pool_id: String,
    pool_name: String,
    invite_code: String,
    reporter_user_id: Option<String>,
    reporter_username: Option<String>,
    category: String,
    details: String,
    status: String,
    reviewed_by: Option<String>,
    reviewed_by_username: Option<String>,
    reviewed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[cfg(feature = "server")]
pub async fn football_scoring_config(
    token: String,
    pool_id: String,
) -> Result<crate::models::FootballScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let member: Option<(String,)> =
        sqlx::query_as("SELECT ?2 WHERE EXISTS (SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?2) OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("football_config_membership", e))?;
    if member.is_none() {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    }
    sqlx::query_as("SELECT exact_score_points,correct_result_exact_side_points,correct_result_points,incorrect_result_points,knockout_bonus_points FROM football_pool_scoring WHERE pool_id=?1").bind(&pool_id).fetch_one(db).await.map_err(|e|crate::security::internal_error("football_config_load",e))
}

#[cfg(feature = "server")]
pub async fn update_football_scoring_config(
    token: String,
    pool_id: String,
    config: crate::models::FootballScoringConfig,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if [
        config.exact_score_points,
        config.correct_result_exact_side_points,
        config.correct_result_points,
        config.incorrect_result_points,
        config.knockout_bonus_points,
    ]
    .iter()
    .any(|v| !(0..=1000).contains(v))
    {
        return Err(crate::security::public_error(
            "Pontuação deve estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let owner:Option<(String,)>=sqlx::query_as("SELECT created_by FROM pools p WHERE p.id=?1 AND (p.created_by=?2 OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)) AND NOT EXISTS (SELECT 1 FROM prediction_items pi JOIN matches m ON m.prediction_item_id=pi.id WHERE pi.event_version_id=p.event_version_id AND datetime(pi.lock_at)<=datetime('now'))").bind(&pool_id).bind(&session.user_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("football_config_owner",e))?;
    if owner.is_none() {
        return Err(crate::security::public_error(
            "Apenas o dono ou admin pode alterar antes do primeiro lock.",
        ));
    }
    sqlx::query("UPDATE football_pool_scoring SET exact_score_points=?2,correct_result_exact_side_points=?3,correct_result_points=?4,incorrect_result_points=?5,knockout_bonus_points=?6,updated_at=datetime('now') WHERE pool_id=?1").bind(&pool_id).bind(config.exact_score_points).bind(config.correct_result_exact_side_points).bind(config.correct_result_points).bind(config.incorrect_result_points).bind(config.knockout_bonus_points).execute(db).await.map_err(|e|crate::security::internal_error("football_config_update",e))?;
    crate::scoring::recalculate_all_breakdowns(Some(&session.user_id)).await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn custom_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
) -> Result<crate::models::CustomItemScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let ok: Option<(String,)> = sqlx::query_as(
        "SELECT ?2 WHERE EXISTS (SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?2) OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("custom_config_member", e))?;
    if ok.is_none() {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    }
    sqlx::query_as("SELECT pool_id,item_id,correct_points,incorrect_points FROM custom_pool_item_scoring WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).fetch_one(db).await.map_err(|e|crate::security::internal_error("custom_config_load",e))
}

#[cfg(feature = "server")]
pub async fn update_custom_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
    correct: i64,
    incorrect: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if !(0..=1000).contains(&correct) || !(0..=1000).contains(&incorrect) {
        return Err(crate::security::public_error(
            "Pontuação deve estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let owner:Option<(String,)>=sqlx::query_as("SELECT p.created_by FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id WHERE p.id=?1 AND (p.created_by=?2 OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)) AND pi.id=?3 AND pi.kind='single_choice' AND datetime(pi.lock_at)>datetime('now')").bind(&pool_id).bind(&session.user_id).bind(&item_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("custom_config_owner",e))?;
    if owner.is_none() {
        return Err(crate::security::public_error(
            "Apenas o dono ou admin pode alterar antes do lock.",
        ));
    }
    sqlx::query("UPDATE custom_pool_item_scoring SET correct_points=?3,incorrect_points=?4,updated_at=datetime('now') WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).bind(correct).bind(incorrect).execute(db).await.map_err(|e|crate::security::internal_error("custom_config_update",e))?;
    crate::scoring::recalculate_custom_breakdowns().await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn numeric_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
) -> Result<crate::models::NumericItemScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let row:Option<(String,String,i64,i64,i64,i64,i64)>=sqlx::query_as("SELECT s.pool_id,s.item_id,s.exact_points,s.tolerance_scaled,s.within_tolerance_points,s.incorrect_points,n.decimal_places FROM numeric_pool_item_scoring s JOIN numeric_questions n ON n.item_id=s.item_id WHERE s.pool_id=?1 AND s.item_id=?2 AND (EXISTS(SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?3) OR EXISTS(SELECT 1 FROM users WHERE id=?3 AND is_admin=1))").bind(&pool_id).bind(&item_id).bind(&session.user_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("numeric_config_load",e))?;
    row.map(
        |(pool_id, item_id, exact, tolerance, within, incorrect, places)| {
            crate::models::NumericItemScoringConfig {
                pool_id,
                item_id,
                exact_points: exact,
                tolerance: crate::numeric::display_scaled(tolerance, places as u8),
                within_tolerance_points: within,
                incorrect_points: incorrect,
            }
        },
    )
    .ok_or_else(|| crate::security::public_error("Configuração numeric inválida."))
}

#[cfg(feature = "server")]
pub async fn update_numeric_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
    exact: i64,
    tolerance: String,
    within: i64,
    incorrect: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if !(0..=1000).contains(&exact)
        || !(0..=1000).contains(&within)
        || !(0..=1000).contains(&incorrect)
    {
        return Err(crate::security::public_error(
            "Pontos devem estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let places:Option<(i64,)>=sqlx::query_as("SELECT n.decimal_places FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN users u ON u.id=?2 WHERE p.id=?1 AND pi.id=?3 AND (p.created_by=?2 OR u.is_admin=1) AND datetime(pi.lock_at)>datetime('now')").bind(&pool_id).bind(&session.user_id).bind(&item_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("numeric_config_owner",e))?;
    let Some((places,)) = places else {
        return Err(crate::security::public_error(
            "Somente o dono do bolão pode alterar regras antes do lock.",
        ));
    };
    let tolerance = crate::numeric::parse_scaled(&tolerance, places as u8)
        .map_err(crate::security::public_error)?;
    if tolerance < 0 {
        return Err(crate::security::public_error(
            "Tolerância não pode ser negativa.",
        ));
    }
    sqlx::query("UPDATE numeric_pool_item_scoring SET exact_points=?3,tolerance_scaled=?4,within_tolerance_points=?5,incorrect_points=?6,updated_at=datetime('now') WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).bind(exact).bind(tolerance).bind(within).bind(incorrect).execute(db).await.map_err(|e|crate::security::internal_error("numeric_config_update",e))?;
    crate::scoring::recalculate_custom_breakdowns().await
}

#[cfg(feature = "server")]
pub async fn multiple_choice_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
) -> Result<crate::models::MultipleChoiceItemScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    sqlx::query_as("SELECT s.pool_id,s.item_id,s.exact_points,s.partial_points,s.incorrect_points FROM multiple_choice_pool_item_scoring s WHERE s.pool_id=?1 AND s.item_id=?2 AND (EXISTS(SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?3) OR EXISTS(SELECT 1 FROM users WHERE id=?3 AND is_admin=1))").bind(&pool_id).bind(&item_id).bind(&session.user_id).fetch_one(db).await.map_err(|e|crate::security::internal_error("multiple_choice_config_load",e))
}
#[cfg(feature = "server")]
pub async fn update_multiple_choice_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
    exact: i64,
    partial: i64,
    incorrect: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if [exact, partial, incorrect]
        .iter()
        .any(|v| !(0..=1000).contains(v))
    {
        return Err(crate::security::public_error(
            "Pontos devem estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let owner:Option<(String,)>=sqlx::query_as("SELECT p.created_by FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id LEFT JOIN users u ON u.id=?2 WHERE p.id=?1 AND pi.id=?3 AND pi.kind='multiple_choice' AND (p.created_by=?2 OR u.is_admin=1) AND datetime(pi.lock_at)>datetime('now')").bind(&pool_id).bind(&session.user_id).bind(&item_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("multiple_choice_config_owner",e))?;
    if owner.is_none() {
        return Err(crate::security::public_error(
            "Somente o dono do bolão pode alterar regras antes do lock.",
        ));
    }
    sqlx::query("UPDATE multiple_choice_pool_item_scoring SET exact_points=?3,partial_points=?4,incorrect_points=?5,updated_at=datetime('now') WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).bind(exact).bind(partial).bind(incorrect).execute(db).await.map_err(|e|crate::security::internal_error("multiple_choice_config_update",e))?;
    crate::scoring::recalculate_custom_breakdowns().await
}

#[cfg(feature = "server")]
type PoolSummaryRow = (
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    Option<String>,
);

#[cfg(feature = "server")]
pub(crate) fn event_summary(
    id: String,
    name: String,
    slug: String,
    kind: String,
    status: String,
    ends_at: Option<String>,
) -> EventSummary {
    let is_historical = status == "finished"
        || ends_at
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|value| value <= chrono::Utc::now());
    EventSummary {
        id,
        name,
        slug,
        kind: if kind == "custom" {
            EventKind::Custom
        } else {
            EventKind::Football
        },
        status: match status.as_str() {
            "draft" => EventStatus::Draft,
            "finished" => EventStatus::Finished,
            _ => EventStatus::Active,
        },
        ends_at,
        description: None,
        cover_url: None,
        external_url: None,
        cover_asset_url: None,
        is_historical,
    }
}

#[cfg(feature = "server")]
type PoolMemberUserRow = (String, String, String, bool, Option<String>, Option<String>);

#[cfg(feature = "server")]
const ALLOWED_REACTION_EMOJIS: [&str; 6] = ["🔥", "👏", "😂", "😮", "😅", "😭"];

#[cfg(feature = "server")]
fn sqlite_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(feature = "server")]
fn normalize_reaction_emoji(emoji: String) -> Result<String, ServerFnError> {
    let emoji = crate::security::normalize_required_text("Emoji", emoji, 1, 8)?;
    if ALLOWED_REACTION_EMOJIS.contains(&emoji.as_str()) {
        Ok(emoji)
    } else {
        Err(crate::security::public_error("Emoji de reacao invalido."))
    }
}

#[cfg(feature = "server")]
async fn ensure_pool_membership(
    db: &sqlx::SqlitePool,
    pool_id: &str,
    user_id: &str,
    error_context: &str,
) -> Result<(), ServerFnError> {
    let membership: Option<(String,)> =
        sqlx::query_as("SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
            .bind(pool_id)
            .bind(user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error(error_context, e))?;

    if membership.is_none() {
        Err(crate::security::public_error(
            "Voce nao e membro deste bolao.",
        ))
    } else {
        Ok(())
    }
}

#[cfg(feature = "server")]
async fn generate_invite_code(pool: &sqlx::SqlitePool) -> Result<String, ServerFnError> {
    use uuid::Uuid;

    for _ in 0..5 {
        let code = Uuid::new_v4().simple().to_string()[..6].to_uppercase();

        let exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM pools WHERE invite_code = ?1")
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
fn normalize_invite_code(value: String) -> Result<String, ServerFnError> {
    let value = crate::security::normalize_required_text("Codigo de convite", value, 6, 64)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(crate::security::public_error("Codigo de convite invalido."));
    }
    Ok(value.to_uppercase())
}

#[cfg(feature = "server")]
fn timestamp_elapsed(value: &str) -> bool {
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(value) {
        return parsed <= chrono::Utc::now();
    }
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .map(|parsed| parsed <= chrono::Utc::now().naive_utc())
        .unwrap_or(false)
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct InviteRecord {
    pool_id: String,
    event_id: String,
    pool_name: String,
    created_by: String,
    description: String,
    visible_rules: String,
    join_closed_at: Option<String>,
    event_name: String,
    event_slug: String,
    event_kind: String,
    event_status: String,
    event_ends_at: Option<String>,
    event_description: Option<String>,
    cover_url: Option<String>,
    cover_asset_id: Option<String>,
    creator_display_name: String,
    member_count: i64,
    lock_deadline: Option<String>,
}

#[cfg(feature = "server")]
async fn resolve_invite(
    db: &sqlx::SqlitePool,
    invite_code: &str,
) -> Result<Option<InviteRecord>, ServerFnError> {
    sqlx::query_as::<_, InviteRecord>(
        "SELECT p.id AS pool_id,p.event_id,p.name AS pool_name,p.created_by,p.description,p.visible_rules,p.join_closed_at,
                v.name AS event_name,e.slug AS event_slug,e.kind AS event_kind,e.status AS event_status,
                e.ends_at AS event_ends_at,v.description AS event_description,v.cover_url,v.cover_asset_id,
                u.username AS creator_display_name,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id=p.id) AS member_count,
                COALESCE((SELECT MIN(pi.lock_at) FROM prediction_items pi
                          WHERE pi.event_version_id=p.event_version_id),e.ends_at) AS lock_deadline
         FROM pools p
         JOIN event_versions v ON v.id=p.event_version_id
         JOIN events e ON e.id=p.event_id
         JOIN users u ON u.id=p.created_by
         WHERE upper(p.invite_code)=?1",
    )
    .bind(invite_code)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("invite_resolve", e))
}

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
        });
    };

    let ended = invite.event_status == "finished"
        || invite
            .event_ends_at
            .as_deref()
            .is_some_and(timestamp_elapsed);
    let closed = invite.join_closed_at.is_some() || ended;
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
        } else if invite.join_closed_at.is_some() || (closed && !can_rejoin_ended) {
            "closed"
        } else {
            "joinable"
        }
        .to_string(),
        pool_id: already_member.then_some(invite.pool_id),
    })
}

#[cfg(feature = "server")]
pub async fn list_my_pools(token: String) -> Result<Vec<PoolSummary>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;

    let rows: Vec<PoolSummaryRow> = sqlx::query_as(
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id) AS member_count,
                p.created_by,
                p.description,
                p.visible_rules,
                p.join_closed_at, v.name, e.slug, e.kind, e.status, e.ends_at
         FROM pools p
         JOIN events e ON e.id = p.event_id
         JOIN event_versions v ON v.id = p.event_version_id
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
        .map(
            |(
                id,
                event_id,
                name,
                invite_code,
                member_count,
                created_by,
                description,
                visible_rules,
                join_closed_at,
                event_name,
                event_slug,
                event_kind,
                event_status,
                event_ends_at,
            )| PoolSummary {
                id,
                event_id: event_id.clone(),
                event: event_summary(
                    event_id.clone(),
                    event_name,
                    event_slug,
                    event_kind,
                    event_status,
                    event_ends_at,
                ),
                name,
                invite_code,
                member_count,
                created_by,
                description,
                visible_rules,
                join_closed_at,
            },
        )
        .collect())
}

#[cfg(feature = "server")]
pub async fn dashboard_pools(token: String) -> Result<Vec<PoolDashboardSummary>, ServerFnError> {
    use crate::auth::require_user;

    crate::security::apply_security_headers();
    let session = require_user(&token).await?;
    let rows: Vec<(String, String, String, String, i64, String, String, String, Option<String>, String, String, String, String, Option<String>, i64, i64)> = sqlx::query_as(
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id = p.id),
                p.created_by, p.description, p.visible_rules, p.join_closed_at,
                v.name, e.slug, e.kind, e.status, e.ends_at,
                (SELECT COUNT(*) FROM predictions pr WHERE pr.pool_id = p.id AND pr.user_id = ?1),
                (SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id = p.event_version_id)
         FROM pools p
         JOIN events e ON e.id = p.event_id
         JOIN event_versions v ON v.id = p.event_version_id
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE pm.user_id = ?1
         ORDER BY CASE WHEN e.ends_at IS NULL THEN 0 ELSE 1 END, datetime(e.ends_at) DESC, p.created_at DESC",
    )
    .bind(&session.user_id)
    .fetch_all(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("dashboard_pools", e))?;
    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                event_id,
                name,
                invite_code,
                member_count,
                created_by,
                description,
                visible_rules,
                join_closed_at,
                event_name,
                event_slug,
                event_kind,
                event_status,
                event_ends_at,
                answered_count,
                item_count,
            )| PoolDashboardSummary {
                pool: PoolSummary {
                    id,
                    event_id: event_id.clone(),
                    event: event_summary(
                        event_id,
                        event_name,
                        event_slug,
                        event_kind,
                        event_status,
                        event_ends_at,
                    ),
                    name,
                    invite_code,
                    member_count,
                    created_by,
                    description,
                    visible_rules,
                    join_closed_at,
                },
                answered_count,
                item_count,
            },
        )
        .collect())
}

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

/// Creates an internal report for a pool while preserving a snapshot of its
/// public identity, even if the pool is deleted later.
#[cfg(feature = "server")]
pub async fn create_pool_report(
    token: String,
    pool_id: String,
    category: String,
    details: String,
    csrf_token: String,
) -> Result<PoolReport, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let category = crate::security::normalize_required_text("Motivo", category, 1, 40)?;
    if !matches!(
        category.as_str(),
        "inappropriate_content" | "spam_or_fraud" | "harassment" | "other"
    ) {
        return Err(crate::security::public_error(
            "Motivo de denuncia invalido.",
        ));
    }
    let details = crate::security::normalize_optional_text(details, 1000)?;
    let db = pool();
    let Some((pool_name, invite_code)) = sqlx::query_as::<_, (String, String)>(
        "SELECT p.name, p.invite_code
         FROM pools p
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE p.id = ?1 AND pm.user_id = ?2",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_membership", e))?
    else {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    };
    let duplicate: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM pool_reports
         WHERE pool_id = ?1 AND reporter_user_id = ?2 AND status IN ('open', 'reviewing')
         LIMIT 1",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_duplicate", e))?;
    if duplicate.is_some() {
        return Err(crate::security::public_error(
            "Voce ja possui uma denuncia aberta para este bolao.",
        ));
    }

    let report_id = uuid::Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("create_pool_report_begin_tx", e))?;
    sqlx::query(
        "INSERT INTO pool_reports
            (id, pool_id, pool_name, invite_code, reporter_user_id, category, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&report_id)
    .bind(&pool_id)
    .bind(&pool_name)
    .bind(&invite_code)
    .bind(&session.user_id)
    .bind(&category)
    .bind(&details)
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_insert", e))?;
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, 'pool_report_created', 'pool', ?3, ?4, ?5)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&session.user_id)
    .bind(&pool_id)
    .bind(crate::security::client_ip(&headers))
    .bind(
        serde_json::json!({ "report_id": report_id.clone(), "category": category.clone() })
            .to_string(),
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("create_pool_report_commit", e))?;

    let row: PoolReportRow = sqlx::query_as(
        "SELECT r.id, r.pool_id, r.pool_name, r.invite_code, r.reporter_user_id,
                reporter.username AS reporter_username, r.category, r.details, r.status, r.reviewed_by,
                reviewer.username AS reviewed_by_username, r.reviewed_at, r.created_at, r.updated_at
         FROM pool_reports r
         LEFT JOIN users reporter ON reporter.id = r.reporter_user_id
         LEFT JOIN users reviewer ON reviewer.id = r.reviewed_by
         WHERE r.id = ?1",
    )
    .bind(&report_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_load", e))?;
    Ok(pool_report_from_row(row))
}

#[cfg(feature = "server")]
fn pool_report_from_row(row: PoolReportRow) -> PoolReport {
    PoolReport {
        id: row.id,
        pool_id: row.pool_id,
        pool_name: row.pool_name,
        invite_code: row.invite_code,
        reporter_user_id: row.reporter_user_id,
        reporter_username: row.reporter_username,
        category: row.category,
        details: row.details,
        status: row.status,
        reviewed_by: row.reviewed_by,
        reviewed_by_username: row.reviewed_by_username,
        reviewed_at: row.reviewed_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
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

    #[derive(sqlx::FromRow)]
    struct ReactionRow {
        target_user_id: String,
        match_id: String,
        emoji: String,
        reactor_user_id: String,
        updated_at: String,
    }

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let session = require_user(&token).await?;
    let db = pool();
    ensure_pool_membership(
        db,
        &pool_id,
        &session.user_id,
        "get_pool_member_predictions_membership",
    )
    .await?;

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
         JOIN pools pool ON pool.id = pm.pool_id
         JOIN predictions pr ON pr.user_id = pm.user_id AND pr.pool_id = pm.pool_id
         JOIN matches m ON m.id = pr.match_id AND m.prediction_item_id = pr.item_id
         JOIN prediction_items pi ON pi.id = pr.item_id
         WHERE pm.pool_id = ?1
           AND datetime(pi.reveal_at) <= datetime(?2)
           AND pi.event_version_id = pool.event_version_id
           -- Consistente com o ranking: só palpites de jogos que começaram
           -- depois de o usuário entrar no bolão.
           AND datetime(pi.lock_at) >= datetime(pm.joined_at)
         ORDER BY m.kickoff",
    )
    .bind(&pool_id)
    .bind(&now)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_pool_member_predictions_preds", e))?;

    let seen_at: Option<(String,)> = sqlx::query_as(
        "SELECT seen_at
         FROM prediction_reaction_views
         WHERE pool_id = ?1 AND user_id = ?2",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("get_pool_member_predictions_seen_at", e))?;

    let reaction_rows = sqlx::query_as::<_, ReactionRow>(
        "SELECT pr.target_user_id AS target_user_id,
                m.id AS match_id,
                pr.emoji AS emoji,
                pr.reactor_user_id AS reactor_user_id,
                pr.updated_at AS updated_at
         FROM prediction_reactions pr
         JOIN pools pool ON pool.id = pr.pool_id
         JOIN predictions p ON p.id = pr.prediction_id
         JOIN matches m ON m.id = p.match_id
         JOIN prediction_items pi ON pi.id = m.prediction_item_id
         JOIN pool_members pm ON pm.pool_id = pr.pool_id AND pm.user_id = pr.target_user_id
         WHERE pr.pool_id = ?1
           AND datetime(pi.reveal_at) <= datetime(?2)
           AND pi.event_version_id = pool.event_version_id
           AND datetime(pi.lock_at) >= datetime(pm.joined_at)
         ORDER BY pr.updated_at ASC",
    )
    .bind(&pool_id)
    .bind(&now)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_pool_member_predictions_reactions", e))?;

    let mut by_user: HashMap<String, Vec<PoolPredictionRecord>> = HashMap::new();
    let mut by_key: HashMap<(String, String), usize> = HashMap::new();
    for row in rows {
        let predictions = by_user.entry(row.user_id.clone()).or_default();
        let index = predictions.len();
        predictions.push(PoolPredictionRecord {
            match_id: row.match_id.clone(),
            home_score: row.home_score,
            away_score: row.away_score,
            qualifier: row.qualifier,
            went_to_penalties: row.went_to_penalties,
            penalty_home_score: row.penalty_home_score,
            penalty_away_score: row.penalty_away_score,
            reactions: Vec::new(),
            viewer_reaction: None,
            unread_reaction_count: 0,
        });
        by_key.insert((row.user_id, row.match_id), index);
    }

    let seen_at = seen_at.map(|row| row.0);
    let mut unread_by_user: HashMap<String, i64> = HashMap::new();
    for row in reaction_rows {
        let key = (row.target_user_id.clone(), row.match_id.clone());
        let Some(index) = by_key.get(&key).copied() else {
            continue;
        };
        let Some(predictions) = by_user.get_mut(&row.target_user_id) else {
            continue;
        };
        let Some(prediction) = predictions.get_mut(index) else {
            continue;
        };

        if let Some(group) = prediction
            .reactions
            .iter_mut()
            .find(|group| group.emoji == row.emoji)
        {
            group.count += 1;
            if row.reactor_user_id == session.user_id {
                group.reacted_by_viewer = true;
            }
        } else {
            prediction.reactions.push(PredictionReactionGroup {
                emoji: row.emoji.clone(),
                count: 1,
                reacted_by_viewer: row.reactor_user_id == session.user_id,
            });
        }

        if row.reactor_user_id == session.user_id {
            prediction.viewer_reaction = Some(row.emoji.clone());
        }

        let unseen = row.target_user_id == session.user_id
            && seen_at
                .as_deref()
                .map(|seen| row.updated_at.as_str() > seen)
                .unwrap_or(true);
        if unseen {
            prediction.unread_reaction_count += 1;
            *unread_by_user
                .entry(row.target_user_id.clone())
                .or_default() += 1;
        }
    }

    Ok(members
        .into_iter()
        .map(|(user_id, username)| MemberPredictions {
            unread_reaction_count: unread_by_user.remove(&user_id).unwrap_or(0),
            user_id: user_id.clone(),
            username,
            predictions: by_user.remove(&user_id).unwrap_or_default(),
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn react_to_prediction(
    token: String,
    pool_id: String,
    target_user_id: String,
    prediction_id: Option<String>,
    match_id: Option<String>,
    emoji: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    let headers = crate::security::current_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    crate::security::validate_uuid("Usuario", &target_user_id)?;
    if prediction_id.is_none() && match_id.is_none() {
        return Err(crate::security::public_error("Prediction obrigatória."));
    }
    if let Some(id) = &prediction_id {
        crate::security::validate_uuid("Prediction", id)?;
    }
    if let Some(id) = &match_id {
        crate::security::validate_match_id(id)?;
    }
    let emoji = normalize_reaction_emoji(emoji)?;
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;

    if target_user_id == session.user_id {
        return Err(crate::security::public_error(
            "Voce nao pode reagir ao proprio palpite.",
        ));
    }

    let db = pool();
    ensure_pool_membership(
        db,
        &pool_id,
        &session.user_id,
        "react_to_prediction_membership",
    )
    .await?;

    let target_prediction: Option<(String, String)> = sqlx::query_as(
        "SELECT p.id, COALESCE(m.home_team || ' x ' || m.away_team, pi.title)
         FROM pool_members pm
         JOIN pools pool ON pool.id = pm.pool_id
         JOIN predictions p ON p.user_id = pm.user_id AND p.pool_id = pm.pool_id
         LEFT JOIN matches m ON m.id = p.match_id AND m.prediction_item_id = p.item_id
         JOIN prediction_items pi ON pi.id = p.item_id
         WHERE pm.pool_id = ?1
           AND pm.user_id = ?3
           AND (p.id = ?2 OR (?5 IS NOT NULL AND p.match_id = ?5))
           AND datetime(pi.reveal_at) <= datetime(?4)
           AND pi.event_version_id = pool.event_version_id
           AND datetime(pi.lock_at) >= datetime(pm.joined_at)",
    )
    .bind(&pool_id)
    .bind(prediction_id.as_deref().unwrap_or(""))
    .bind(&target_user_id)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&match_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("react_to_prediction_target", e))?;

    let Some((prediction_id, prediction_label)) = target_prediction else {
        return Err(crate::security::public_error(
            "Esse palpite nao esta disponivel para reacao.",
        ));
    };

    let reactor_username: (String,) = sqlx::query_as("SELECT username FROM users WHERE id = ?1")
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("react_to_prediction_reactor", e))?;

    let existing: Option<(String, String)> = sqlx::query_as(
        "SELECT id, emoji
         FROM prediction_reactions
         WHERE pool_id = ?1 AND prediction_id = ?2 AND target_user_id = ?3 AND reactor_user_id = ?4",
    )
    .bind(&pool_id)
    .bind(&prediction_id)
    .bind(&target_user_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("react_to_prediction_existing", e))?;

    let now = sqlite_now();
    let action = match existing {
        None => {
            sqlx::query(
                "INSERT INTO prediction_reactions
                    (id, pool_id, prediction_id, target_user_id, reactor_user_id, emoji, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&pool_id)
            .bind(&prediction_id)
            .bind(&target_user_id)
            .bind(&session.user_id)
            .bind(&emoji)
            .bind(&now)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("react_to_prediction_insert", e))?;
            "prediction_reaction_created"
        }
        Some((reaction_id, existing_emoji)) if existing_emoji == emoji => {
            sqlx::query("DELETE FROM prediction_reactions WHERE id = ?1")
                .bind(&reaction_id)
                .execute(db)
                .await
                .map_err(|e| crate::security::internal_error("react_to_prediction_delete", e))?;
            "prediction_reaction_removed"
        }
        Some((reaction_id, _)) => {
            sqlx::query(
                "UPDATE prediction_reactions
                 SET emoji = ?1, updated_at = ?2
                 WHERE id = ?3",
            )
            .bind(&emoji)
            .bind(&now)
            .bind(&reaction_id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("react_to_prediction_update", e))?;
            "prediction_reaction_changed"
        }
    };

    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        action,
        "prediction_reaction",
        Some(&pool_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({
            "pool_id": pool_id,
            "prediction_id": prediction_id,
            "target_user_id": target_user_id,
            "emoji": emoji,
        }),
    )
    .await?;

    if action != "prediction_reaction_removed" {
        let url = format!("/palpites-do-bolao?poolId={pool_id}&memberId={target_user_id}");
        let title = format!("{} reagiu ao seu palpite", reactor_username.0);
        let body = format!(
            "{} reagiu com {} em {}.",
            reactor_username.0, emoji, prediction_label
        );
        let tag = format!("prediction-reaction-{pool_id}-{prediction_id}-{target_user_id}");
        let _ =
            crate::push::send_reaction_notification(db, &target_user_id, &title, &body, &url, &tag)
                .await?;
    }

    Ok(())
}

#[cfg(feature = "server")]
pub async fn mark_prediction_reactions_seen(
    token: String,
    pool_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = pool();
    ensure_pool_membership(
        db,
        &pool_id,
        &session.user_id,
        "mark_prediction_reactions_seen_membership",
    )
    .await?;

    sqlx::query(
        "INSERT INTO prediction_reaction_views (pool_id, user_id, seen_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(pool_id, user_id) DO UPDATE SET seen_at = excluded.seen_at",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .bind(sqlite_now())
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("mark_prediction_reactions_seen", e))?;

    Ok(())
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
        "SELECT p.id, p.event_id, p.name, p.invite_code,
                (SELECT COUNT(*) FROM pool_members pm WHERE pm.pool_id = p.id) AS member_count,
                p.created_by,
                p.description,
                p.visible_rules,
                p.join_closed_at, v.name, e.slug, e.kind, e.status, e.ends_at
         FROM pools p
         JOIN events e ON e.id = p.event_id
         JOIN event_versions v ON v.id = p.event_version_id
         ORDER BY p.name COLLATE NOCASE",
    )
    .fetch_all(pool())
    .await
    .map_err(|e| crate::security::internal_error("list_all_pools_admin", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                event_id,
                name,
                invite_code,
                member_count,
                created_by,
                description,
                visible_rules,
                join_closed_at,
                event_name,
                event_slug,
                event_kind,
                event_status,
                event_ends_at,
            )| PoolSummary {
                id,
                event_id: event_id.clone(),
                event: event_summary(
                    event_id.clone(),
                    event_name,
                    event_slug,
                    event_kind,
                    event_status,
                    event_ends_at,
                ),
                name,
                invite_code,
                member_count,
                created_by,
                description,
                visible_rules,
                join_closed_at,
            },
        )
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
        .map(
            |(id, username, email, is_admin, blocked_at, blocked_reason)| UserPublic {
                id,
                username,
                email,
                is_admin,
                blocked_at,
                blocked_reason,
            },
        )
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

    let _ = crate::scoring::recalculate_pool_user_breakdowns(
        &pool_id,
        &user_id,
        Some(&session.user_id),
    )
    .await?;

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

#[cfg(feature = "server")]
pub async fn list_pool_reports_admin(
    token: String,
    status: Option<String>,
) -> Result<Vec<PoolReport>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    if let Some(value) = status.as_deref() {
        if !matches!(value, "open" | "reviewing" | "resolved" | "dismissed") {
            return Err(crate::security::public_error(
                "Status de denuncia invalido.",
            ));
        }
    }
    let rows: Vec<PoolReportRow> = sqlx::query_as(
        "SELECT r.id, r.pool_id, r.pool_name, r.invite_code, r.reporter_user_id,
                reporter.username AS reporter_username, r.category, r.details, r.status, r.reviewed_by,
                reviewer.username AS reviewed_by_username, r.reviewed_at, r.created_at, r.updated_at
         FROM pool_reports r
         LEFT JOIN users reporter ON reporter.id = r.reporter_user_id
         LEFT JOIN users reviewer ON reviewer.id = r.reviewed_by
         WHERE (?1 IS NULL OR r.status = ?1)
         ORDER BY CASE r.status WHEN 'open' THEN 0 WHEN 'reviewing' THEN 1 ELSE 2 END,
                  datetime(r.created_at) DESC",
    )
    .bind(status)
    .fetch_all(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("list_pool_reports_admin", e))?;
    Ok(rows.into_iter().map(pool_report_from_row).collect())
}

#[cfg(feature = "server")]
pub async fn update_pool_report_status_admin(
    token: String,
    report_id: String,
    status: String,
    csrf_token: String,
) -> Result<PoolReport, ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Denuncia", &report_id)?;
    if !matches!(
        status.as_str(),
        "open" | "reviewing" | "resolved" | "dismissed"
    ) {
        return Err(crate::security::public_error(
            "Status de denuncia invalido.",
        ));
    }
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let result = sqlx::query(
        "UPDATE pool_reports
         SET status = ?2,
             reviewed_by = CASE WHEN ?2 = 'open' THEN NULL ELSE ?3 END,
             reviewed_at = CASE WHEN ?2 = 'open' THEN NULL ELSE datetime('now') END,
             updated_at = datetime('now')
         WHERE id = ?1",
    )
    .bind(&report_id)
    .bind(&status)
    .bind(&session.user_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("update_pool_report_status", e))?;
    if result.rows_affected() == 0 {
        return Err(crate::security::public_error("Denuncia nao encontrada."));
    }
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "pool_report_status_changed",
        "pool_report",
        Some(&report_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "status": status }),
    )
    .await?;
    let row: PoolReportRow = sqlx::query_as(
        "SELECT r.id, r.pool_id, r.pool_name, r.invite_code, r.reporter_user_id,
                reporter.username AS reporter_username, r.category, r.details, r.status, r.reviewed_by,
                reviewer.username AS reviewed_by_username, r.reviewed_at, r.created_at, r.updated_at
         FROM pool_reports r
         LEFT JOIN users reporter ON reporter.id = r.reporter_user_id
         LEFT JOIN users reviewer ON reviewer.id = r.reviewed_by
         WHERE r.id = ?1",
    )
    .bind(&report_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("update_pool_report_load", e))?;
    Ok(pool_report_from_row(row))
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
