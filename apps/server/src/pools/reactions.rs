use crate::error::ServerFnError;

const ALLOWED_REACTION_EMOJIS: [&str; 6] = ["🔥", "👏", "😂", "😮", "😅", "😭"];

fn normalize_reaction_emoji(emoji: String) -> Result<String, ServerFnError> {
    let emoji = crate::security::normalize_required_text("Emoji", emoji, 1, 8)?;
    if ALLOWED_REACTION_EMOJIS.contains(&emoji.as_str()) {
        Ok(emoji)
    } else {
        Err(crate::security::public_error("Emoji de reacao invalido."))
    }
}

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
    let db = crate::db::pool();
    super::ensure_pool_membership(
        db,
        &pool_id,
        &session.user_id,
        "react_to_prediction_membership",
    )
    .await?;
    let target_prediction: Option<(String, String)> = sqlx::query_as("SELECT p.id, COALESCE(m.home_team || ' x ' || m.away_team, pi.title) FROM pool_members pm JOIN pools pool ON pool.id = pm.pool_id JOIN predictions p ON p.user_id = pm.user_id AND p.pool_id = pm.pool_id LEFT JOIN matches m ON m.id = p.match_id AND m.prediction_item_id = p.item_id JOIN prediction_items pi ON pi.id = p.item_id WHERE pm.pool_id = ?1 AND pm.user_id = ?3 AND (p.id = ?2 OR (?5 IS NOT NULL AND p.match_id = ?5)) AND datetime(pi.reveal_at) <= datetime(?4) AND pi.event_version_id = pool.event_version_id AND datetime(pi.lock_at) >= datetime(pm.joined_at)")
        .bind(&pool_id).bind(prediction_id.as_deref().unwrap_or("")).bind(&target_user_id).bind(chrono::Utc::now().to_rfc3339()).bind(&match_id).fetch_optional(db).await.map_err(|e| crate::security::internal_error("react_to_prediction_target", e))?;
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
    let existing: Option<(String, String)> = sqlx::query_as("SELECT id, emoji FROM prediction_reactions WHERE pool_id = ?1 AND prediction_id = ?2 AND target_user_id = ?3 AND reactor_user_id = ?4")
        .bind(&pool_id).bind(&prediction_id).bind(&target_user_id).bind(&session.user_id).fetch_optional(db).await.map_err(|e| crate::security::internal_error("react_to_prediction_existing", e))?;
    let now = super::sqlite_now();
    let action = match existing {
        None => {
            sqlx::query("INSERT INTO prediction_reactions (id, pool_id, prediction_id, target_user_id, reactor_user_id, emoji, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)").bind(uuid::Uuid::new_v4().to_string()).bind(&pool_id).bind(&prediction_id).bind(&target_user_id).bind(&session.user_id).bind(&emoji).bind(&now).execute(db).await.map_err(|e| crate::security::internal_error("react_to_prediction_insert", e))?;
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
                "UPDATE prediction_reactions SET emoji = ?1, updated_at = ?2 WHERE id = ?3",
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
    crate::security::append_audit_log(db, Some(&session.user_id), action, "prediction_reaction", Some(&pool_id), Some(&crate::security::client_ip(&headers)), serde_json::json!({"pool_id": pool_id, "prediction_id": prediction_id, "target_user_id": target_user_id, "emoji": emoji})).await?;
    if action != "prediction_reaction_removed" {
        let url = format!("/palpites-do-bolao?poolId={pool_id}&memberId={target_user_id}");
        let title = format!("{} reagiu ao seu palpite", reactor_username.0);
        let body = format!(
            "{} reagiu com {} em {}.",
            reactor_username.0, emoji, prediction_label
        );
        let tag = format!("prediction-reaction-{pool_id}-{prediction_id}-{target_user_id}");
        crate::push::send_reaction_notification(db, &target_user_id, &title, &body, &url, &tag)
            .await?;
    }
    Ok(())
}

pub async fn mark_prediction_reactions_seen(
    token: String,
    pool_id: String,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_user;
    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    super::ensure_pool_membership(
        db,
        &pool_id,
        &session.user_id,
        "mark_prediction_reactions_seen_membership",
    )
    .await?;
    sqlx::query("INSERT INTO prediction_reaction_views (pool_id, user_id, seen_at) VALUES (?1, ?2, ?3) ON CONFLICT(pool_id, user_id) DO UPDATE SET seen_at = excluded.seen_at")
        .bind(&pool_id).bind(&session.user_id).bind(super::sqlite_now()).execute(db).await
        .map_err(|e| crate::security::internal_error("mark_prediction_reactions_seen", e))?;
    Ok(())
}
