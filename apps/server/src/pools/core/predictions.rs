use super::*;
use crate::{error::ServerFnError, models::*};

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

// ---------------------------------------------------------------------------
// Administração de bolões (somente admin)
// ---------------------------------------------------------------------------
