use crate::{error::ServerFnError, models::LeaderboardEntry, scoring::ensure_breakdowns_seeded};

#[derive(Debug, Default, Clone, Copy)]
struct LeaderboardTally {
    points: i64,
    exact_scores: i64,
    correct_results: i64,
    bonus_points: i64,
}

/// Calcula o ranking de um bolão somando a pontuação de cada palpite contra os
/// resultados oficiais já lançados.
///
/// Empates em pontos são resolvidos, nesta ordem, por: mais placares exatos,
/// mais acertos de resultado, mais bônus de precisão e, por fim, ordem
/// alfabética do nome (apenas para manter o ranking determinístico).
#[cfg(feature = "server")]
pub async fn get_leaderboard(
    token: String,
    pool_id: String,
) -> Result<Vec<LeaderboardEntry>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use std::collections::HashMap;

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
            .map_err(|e| crate::security::internal_error("get_leaderboard_membership", e))?;

    if membership.is_none() {
        return Err(crate::security::public_error(
            "Voce nao e membro deste bolao.",
        ));
    }

    // Todos os membros, para ranquear inclusive quem ainda não pontuou.
    let members: Vec<(String, String)> = sqlx::query_as(
        "SELECT u.id, u.username
         FROM pool_members pm
         JOIN users u ON u.id = pm.user_id
         WHERE pm.pool_id = ?1",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_leaderboard_members", e))?;

    ensure_breakdowns_seeded(db).await?;

    let mut tallies: HashMap<String, LeaderboardTally> = members
        .iter()
        .map(|(id, _)| (id.clone(), LeaderboardTally::default()))
        .collect();

    let materialized: Vec<(String, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT user_id,
                COALESCE(SUM(total_points), 0),
                COALESCE(SUM(CASE WHEN exact_score_points > 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN exact_score_points > 0 OR outcome_points > 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(goal_bonus_points + qualifier_points + penalties_points), 0)
         FROM prediction_score_breakdowns
         WHERE pool_id = ?1 AND eligible = 1
         GROUP BY user_id",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_leaderboard_materialized", e))?;

    for (user_id, total, exact_scores, correct_results, bonus_points) in materialized {
        let t = tallies.entry(user_id).or_default();
        t.points += total;
        t.exact_scores += exact_scores;
        t.correct_results += correct_results;
        t.bonus_points += bonus_points;
    }
    let custom: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT user_id, COALESCE(SUM(total_points),0), COALESCE(SUM(CASE WHEN correct_points > 0 THEN 1 ELSE 0 END),0)
         FROM custom_prediction_score_breakdowns WHERE pool_id=?1 AND eligible=1 GROUP BY user_id",
    ).bind(&pool_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("get_leaderboard_custom", e))?;
    for (user_id, total, correct) in custom {
        let t = tallies.entry(user_id).or_default();
        t.points += total;
        t.correct_results += correct;
    }
    let numeric: Vec<(String,i64,i64)>=sqlx::query_as("SELECT user_id,COALESCE(SUM(total_points),0),COALESCE(SUM(CASE WHEN outcome='exact' THEN 1 ELSE 0 END),0) FROM numeric_prediction_score_breakdowns WHERE pool_id=?1 AND eligible=1 GROUP BY user_id").bind(&pool_id).fetch_all(db).await.map_err(|e|crate::security::internal_error("get_leaderboard_numeric",e))?;
    for (user_id, total, exact) in numeric {
        let t = tallies.entry(user_id).or_default();
        t.points += total;
        t.correct_results += exact;
    }
    let multiple: Vec<(String,i64,i64)>=sqlx::query_as("SELECT user_id,COALESCE(SUM(total_points),0),COALESCE(SUM(CASE WHEN outcome='exact' THEN 1 ELSE 0 END),0) FROM multiple_choice_prediction_score_breakdowns WHERE pool_id=?1 AND eligible=1 GROUP BY user_id").bind(&pool_id).fetch_all(db).await.map_err(|e|crate::security::internal_error("get_leaderboard_multiple_choice",e))?;
    for (user_id, total, exact) in multiple {
        let t = tallies.entry(user_id).or_default();
        t.points += total;
        t.correct_results += exact;
    }

    // Ajustes manuais de pontos lançados pelo organizador (ou admin) somam ao total.
    let adjustments: Vec<(String, i64)> = sqlx::query_as(
        "SELECT user_id, SUM(delta) FROM point_adjustments WHERE pool_id = ?1 GROUP BY user_id",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_leaderboard_adjustments", e))?;

    // Ajustes manuais somam só nos pontos totais: o desempate deve refletir
    // apenas acertos reais dos palpites, não correções do organizador.
    for (user_id, total) in adjustments {
        if let Some(t) = tallies.get_mut(&user_id) {
            t.points += total;
        }
    }

    let mut entries: Vec<LeaderboardEntry> = members
        .into_iter()
        .map(|(id, username)| {
            let t = tallies.get(&id).copied().unwrap_or_default();
            LeaderboardEntry {
                points: t.points,
                exact_scores: t.exact_scores,
                correct_results: t.correct_results,
                bonus_points: t.bonus_points,
                user_id: id,
                username,
            }
        })
        .collect();

    rank_leaderboard(&mut entries);

    Ok(entries)
}

/// Ordena o ranking pelo total de pontos e, em caso de empate, pelos critérios
/// de desempate: mais placares exatos, mais acertos de resultado, mais bônus de
/// precisão e, por último, ordem alfabética do nome (só para ser determinístico).
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(any(feature = "server", test))]
pub fn rank_leaderboard(entries: &mut [LeaderboardEntry]) {
    entries.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then_with(|| b.exact_scores.cmp(&a.exact_scores))
            .then_with(|| b.correct_results.cmp(&a.correct_results))
            .then_with(|| b.bonus_points.cmp(&a.bonus_points))
            .then_with(|| a.username.cmp(&b.username))
    });
}
