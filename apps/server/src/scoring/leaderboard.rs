use crate::{error::ServerFnError, models::LeaderboardEntry, scoring::ensure_breakdowns_seeded};

#[derive(Debug, Default, Clone, Copy)]
struct LeaderboardTally {
    points: i64,
    exact_scores: i64,
    correct_results: i64,
    bonus_points: i64,
}

fn football_business_order(a: &LeaderboardEntry, b: &LeaderboardEntry) -> std::cmp::Ordering {
    b.points
        .cmp(&a.points)
        .then_with(|| b.exact_scores.cmp(&a.exact_scores))
        .then_with(|| b.correct_results.cmp(&a.correct_results))
        .then_with(|| b.bonus_points.cmp(&a.bonus_points))
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

    let is_football: (String,) =
        sqlx::query_as("SELECT e.kind FROM pools p JOIN events e ON e.id=p.event_id WHERE p.id=?1")
            .bind(&pool_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("get_leaderboard_kind", e))?;
    let mut entries: Vec<LeaderboardEntry> = members
        .into_iter()
        .map(|(id, username)| {
            let t = tallies.get(&id).copied().unwrap_or_default();
            LeaderboardEntry {
                position: 0,
                points: t.points,
                exact_scores: t.exact_scores,
                correct_results: t.correct_results,
                bonus_points: t.bonus_points,
                user_id: id,
                username,
            }
        })
        .collect();

    if is_football.0 == "football" {
        rank_leaderboard(&mut entries);
    } else {
        let priorities = crate::pool_tiebreak::effective_priorities_for_pool(&pool_id).await?;
        let item_ids: Vec<String> = priorities.iter().map(|p| p.item_id.clone()).collect();
        let resolved: Vec<(String,)> = sqlx::query_as(
            "SELECT pi.id FROM prediction_items pi LEFT JOIN custom_questions cq ON cq.item_id=pi.id LEFT JOIN numeric_questions nq ON nq.item_id=pi.id
             WHERE pi.event_version_id=(SELECT event_version_id FROM pools WHERE id=?1) AND pi.id IN (SELECT value FROM json_each(?2))
             AND ((pi.kind='single_choice' AND cq.correct_option_id IS NOT NULL) OR (pi.kind='numeric' AND nq.result_value_scaled IS NOT NULL) OR (pi.kind='multiple_choice' AND EXISTS(SELECT 1 FROM multiple_choice_results mr WHERE mr.item_id=pi.id)))",
        ).bind(&pool_id).bind(serde_json::to_string(&item_ids).unwrap()).fetch_all(db).await.map_err(|e| crate::security::internal_error("leaderboard_tiebreak_resolved", e))?;
        let resolved_ids: std::collections::HashSet<String> =
            resolved.into_iter().map(|r| r.0).collect();
        let exact: Vec<(String,String)> = sqlx::query_as(
            "SELECT p.user_id,p.item_id FROM predictions p JOIN custom_prediction_values v ON v.prediction_id=p.id JOIN custom_questions q ON q.item_id=p.item_id JOIN prediction_items pi ON pi.id=p.item_id JOIN pool_members pm ON pm.pool_id=p.pool_id AND pm.user_id=p.user_id WHERE p.pool_id=?1 AND datetime(pi.lock_at)>=datetime(pm.joined_at) AND v.option_id=q.correct_option_id
             UNION SELECT p.user_id,p.item_id FROM predictions p JOIN numeric_prediction_values v ON v.prediction_id=p.id JOIN numeric_questions q ON q.item_id=p.item_id JOIN prediction_items pi ON pi.id=p.item_id JOIN pool_members pm ON pm.pool_id=p.pool_id AND pm.user_id=p.user_id WHERE p.pool_id=?1 AND datetime(pi.lock_at)>=datetime(pm.joined_at) AND v.value_scaled=q.result_value_scaled
             UNION SELECT user_id,item_id FROM multiple_choice_prediction_score_breakdowns WHERE pool_id=?1 AND eligible=1 AND outcome='exact'",
        ).bind(&pool_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("leaderboard_tiebreak_exact", e))?;
        let exact: std::collections::HashSet<(String, String)> = exact.into_iter().collect();
        let active: Vec<String> = item_ids
            .into_iter()
            .filter(|id| resolved_ids.contains(id))
            .collect();
        entries.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then_with(|| {
                    for item_id in &active {
                        let left = exact.contains(&(a.user_id.clone(), item_id.clone()));
                        let right = exact.contains(&(b.user_id.clone(), item_id.clone()));
                        let order = right.cmp(&left);
                        if order != std::cmp::Ordering::Equal {
                            return order;
                        }
                    }
                    std::cmp::Ordering::Equal
                })
                .then_with(|| a.username.cmp(&b.username))
        });
        let mut prior: Option<&LeaderboardEntry> = None;
        for index in 0..entries.len() {
            let same = prior
                .map(|entry| {
                    entry.points == entries[index].points
                        && active.iter().all(|item_id| {
                            exact.contains(&(entry.user_id.clone(), item_id.clone()))
                                == exact
                                    .contains(&(entries[index].user_id.clone(), item_id.clone()))
                        })
                })
                .unwrap_or(false);
            entries[index].position = if same {
                entries[index - 1].position
            } else {
                index as i64 + 1
            };
            prior = Some(&entries[index]);
        }
    }

    Ok(entries)
}

/// Ordena o ranking pelo total de pontos e, em caso de empate, pelos critérios
/// de desempate: mais placares exatos, mais acertos de resultado, mais bônus de
/// precisão e, por último, ordem alfabética do nome (só para ser determinístico).
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(any(feature = "server", test))]
pub fn rank_leaderboard(entries: &mut [LeaderboardEntry]) {
    entries.sort_by(|a, b| football_business_order(a, b).then_with(|| a.username.cmp(&b.username)));
    let mut previous: Option<&LeaderboardEntry> = None;
    for index in 0..entries.len() {
        let tied = previous
            .map(|entry| {
                football_business_order(entry, &entries[index]) == std::cmp::Ordering::Equal
            })
            .unwrap_or(false);
        entries[index].position = if tied {
            entries[index - 1].position
        } else {
            index as i64 + 1
        };
        previous = Some(&entries[index]);
    }
}
