use crate::error::ServerFnError;
use crate::models::{KnockoutEntry, MatchRecord, PredictionRecord};

use super::lifecycle::{knockout_released_flag, token_is_admin};
use super::repository::{record_from_row, MatchRow};

#[cfg(feature = "server")]
pub async fn list_matches(token: String) -> Result<Vec<MatchRecord>, ServerFnError> {
    crate::security::apply_security_headers();
    let is_admin = token_is_admin(&token).await;
    let show_knockout = is_admin || knockout_released_flag().await?;
    let event_id = crate::events::world_cup_2026_event_id(crate::db::pool()).await?;
    let version_selector = if is_admin {
        "COALESCE((SELECT id FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1), (SELECT current_published_version_id FROM events WHERE id=?1))"
    } else {
        "(SELECT current_published_version_id FROM events WHERE id=?1)"
    };
    let sql = if show_knockout {
        format!("SELECT m.id,m.home_team,m.away_team,m.kickoff,m.group_name,m.phase,home_score,away_score,qualifier,went_to_penalties,penalty_home_score,penalty_away_score,finished FROM matches m WHERE m.event_version_id={version_selector} ORDER BY m.kickoff ASC")
    } else {
        format!("SELECT m.id,m.home_team,m.away_team,m.kickoff,m.group_name,m.phase,home_score,away_score,qualifier,went_to_penalties,penalty_home_score,penalty_away_score,finished FROM matches m WHERE m.event_version_id={version_selector} AND m.phase='Fase de grupos' ORDER BY m.kickoff ASC")
    };
    let rows = sqlx::query_as::<_, MatchRow>(&sql)
        .bind(&event_id)
        .fetch_all(crate::db::pool())
        .await
        .map_err(|error| crate::security::internal_error("list_matches", error))?;
    Ok(rows.into_iter().map(record_from_row).collect())
}

#[cfg(feature = "server")]
pub async fn get_my_predictions(
    token: String,
    pool_id: String,
) -> Result<Vec<PredictionRecord>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct PredictionRow {
        item_id: String,
        match_id: String,
        home_score: i64,
        away_score: i64,
        qualifier: Option<String>,
        went_to_penalties: bool,
        penalty_home_score: Option<i64>,
        penalty_away_score: Option<i64>,
    }
    crate::security::apply_security_headers();
    let session = crate::auth::require_user(&token).await?;
    let rows = sqlx::query_as::<_, PredictionRow>("SELECT item_id,match_id,home_score,away_score,qualifier,went_to_penalties,penalty_home_score,penalty_away_score FROM predictions WHERE user_id=?1 AND pool_id=?2 AND match_id IS NOT NULL")
        .bind(&session.user_id).bind(&pool_id).fetch_all(crate::db::pool()).await
        .map_err(|error| crate::security::internal_error("get_my_predictions", error))?;
    Ok(rows
        .into_iter()
        .map(|row| PredictionRecord {
            item_id: row.item_id,
            match_id: row.match_id,
            home_score: row.home_score,
            away_score: row.away_score,
            qualifier: row.qualifier,
            went_to_penalties: row.went_to_penalties,
            penalty_home_score: row.penalty_home_score,
            penalty_away_score: row.penalty_away_score,
        })
        .collect())
}

#[cfg(feature = "server")]
pub(super) fn sanitize_knockout_input(
    is_knockout: bool,
    home_score: i64,
    away_score: i64,
    entry: KnockoutEntry,
) -> Result<KnockoutEntry, ServerFnError> {
    if !is_knockout {
        return Ok(KnockoutEntry::default());
    }
    if home_score != away_score {
        return Ok(KnockoutEntry {
            qualifier: Some(
                if home_score > away_score {
                    "home"
                } else {
                    "away"
                }
                .into(),
            ),
            went_to_penalties: false,
            penalty_home: None,
            penalty_away: None,
        });
    }
    let (penalty_home, penalty_away) = match (entry.penalty_home, entry.penalty_away) {
        (Some(home), Some(away)) if home < 0 || away < 0 => {
            return Err(crate::security::public_error(
                "O placar dos pênaltis não pode ser negativo.",
            ))
        }
        (Some(home), Some(away)) if home == away => {
            return Err(crate::security::public_error(
                "O placar dos pênaltis não pode terminar empatado.",
            ))
        }
        (Some(home), Some(away)) => (home, away),
        _ => {
            return Err(crate::security::public_error(
                "Empate no tempo normal: informe o placar dos pênaltis dos dois lados.",
            ))
        }
    };
    Ok(KnockoutEntry {
        qualifier: Some(
            if penalty_home > penalty_away {
                "home"
            } else {
                "away"
            }
            .into(),
        ),
        went_to_penalties: true,
        penalty_home: Some(penalty_home),
        penalty_away: Some(penalty_away),
    })
}

#[cfg(feature = "server")]
pub async fn submit_prediction(
    token: String,
    pool_id: String,
    match_id: String,
    home_score: i64,
    away_score: i64,
    knockout: KnockoutEntry,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::models::is_knockout;
    use uuid::Uuid;
    crate::security::apply_security_headers();
    crate::security::validate_match_id(&match_id)?;
    if home_score < 0 || away_score < 0 {
        return Err(crate::security::public_error(
            "Os placares nao podem ser negativos.",
        ));
    }
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let row: Option<(String, String, Option<String>)> = sqlx::query_as("SELECT pi.id,pi.lock_at,m.phase FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id JOIN pools p ON p.event_version_id=pi.event_version_id JOIN events e ON e.id=p.event_id JOIN pool_members pm ON pm.pool_id=p.id AND pm.user_id=?2 WHERE m.id=?1 AND p.id=?3 AND e.status='active'")
        .bind(&match_id).bind(&session.user_id).bind(&pool_id).fetch_optional(db).await
        .map_err(|error| crate::security::internal_error("submit_prediction_match_lookup", error))?;
    let Some((item_id, lock_at, phase)) = row else {
        return Err(crate::security::public_error("Partida nao encontrada."));
    };
    let override_id = crate::prediction_access::can_edit_item("football_match", &lock_at, Some(&match_id), &session.user_id).await?
        .ok_or_else(|| crate::security::public_error("Essa partida esta travada para palpite; use uma reabertura administrativa se necessario."))?;
    let ko = sanitize_knockout_input(
        is_knockout(phase.as_deref()),
        home_score,
        away_score,
        knockout,
    )?;
    let mut tx = db
        .begin()
        .await
        .map_err(|error| crate::security::internal_error("submit_prediction_begin_tx", error))?;
    sqlx::query("INSERT INTO predictions (id,pool_id,user_id,item_id,match_id,home_score,away_score,qualifier,went_to_penalties,penalty_home_score,penalty_away_score) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(pool_id,user_id,item_id) DO UPDATE SET match_id=excluded.match_id,home_score=excluded.home_score,away_score=excluded.away_score,qualifier=excluded.qualifier,went_to_penalties=excluded.went_to_penalties,penalty_home_score=excluded.penalty_home_score,penalty_away_score=excluded.penalty_away_score")
        .bind(Uuid::new_v4().to_string()).bind(&pool_id).bind(&session.user_id).bind(&item_id).bind(&match_id).bind(home_score).bind(away_score).bind(&ko.qualifier).bind(ko.went_to_penalties).bind(ko.penalty_home).bind(ko.penalty_away).execute(&mut *tx).await.map_err(|error| crate::security::internal_error("submit_prediction_upsert", error))?;
    tx.commit()
        .await
        .map_err(|error| crate::security::internal_error("submit_prediction_commit", error))?;
    if !override_id.is_empty() {
        crate::admin::mark_prediction_override_used(&override_id).await?;
    }
    let _ = crate::scoring::recalculate_match_breakdowns(&match_id, Some(&session.user_id)).await?;
    Ok(())
}
