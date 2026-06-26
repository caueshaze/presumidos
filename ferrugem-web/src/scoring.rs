use crate::error::ServerFnError;

use crate::models::{LeaderboardEntry, MatchPointsSummary, PredictionScoreBreakdown, ScoringJob};

/// Resultado oficial (ou palpite) de uma partida, no formato usado pela
/// pontuação. Os campos de mata-mata só são considerados em jogos de mata-mata.
#[cfg(any(feature = "server", test))]
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub home_score: i64,
    pub away_score: i64,
    /// 'home' ou 'away' — quem se classifica (mata-mata).
    pub qualifier: Option<String>,
    pub went_to_penalties: bool,
    pub penalty_home: Option<i64>,
    pub penalty_away: Option<i64>,
}

/// Pontuação base do placar (vale para todos os jogos), sobre o tempo normal:
/// - placar exato → 7
/// - resultado correto → 3
/// - resultado correto + gols de um time que fez ≥1 → 4
/// - resultado errado → 0
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(any(feature = "server", test))]
pub fn base_points(guess_home: i64, guess_away: i64, real_home: i64, real_away: i64) -> i64 {
    if guess_home == real_home && guess_away == real_away {
        return 7;
    }

    let correct_outcome = (guess_home > guess_away && real_home > real_away)
        || (guess_home < guess_away && real_home < real_away)
        || (guess_home == guess_away && real_home == real_away);

    if !correct_outcome {
        return 0;
    }

    // O bônus de +1 só conta se o time acertado fez pelo menos 1 gol.
    let goal_bonus = (guess_home == real_home && real_home > 0)
        || (guess_away == real_away && real_away > 0);

    if goal_bonus {
        4
    } else {
        3
    }
}

/// Bônus de pênaltis do mata-mata, somado à pontuação base.
///
/// Só vale quando o placar do tempo normal foi acertado exatamente (base = 7)
/// e o jogo de fato foi para os pênaltis. O classificado deixa de ter bônus
/// próprio — quem avança é deduzido do placar/pênaltis.
/// - placar exato dos pênaltis (ex.: 5x4) → +3 (leva um empate exato de 7 para 10)
/// - só o vencedor da disputa de pênaltis → +1
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(any(feature = "server", test))]
pub fn knockout_bonus(official: &Outcome, guess: &Outcome) -> i64 {
    // Bônus só se o placar do tempo normal foi exatamente acertado.
    if guess.home_score != official.home_score || guess.away_score != official.away_score {
        return 0;
    }
    if !(official.went_to_penalties && guess.went_to_penalties) {
        return 0;
    }

    let (Some(gh), Some(ga), Some(oh), Some(oa)) = (
        guess.penalty_home,
        guess.penalty_away,
        official.penalty_home,
        official.penalty_away,
    ) else {
        return 0;
    };

    if gh == oh && ga == oa {
        // Placar exato dos pênaltis.
        3
    } else if (gh > ga) == (oh > oa) {
        // Acertou só o vencedor da disputa.
        1
    } else {
        0
    }
}

/// Pontuação total de um palpite contra o resultado oficial de uma partida.
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(any(feature = "server", test))]
pub fn match_points(is_knockout: bool, official: &Outcome, guess: &Outcome) -> i64 {
    let base = base_points(
        guess.home_score,
        guess.away_score,
        official.home_score,
        official.away_score,
    );

    if is_knockout {
        base + knockout_bonus(official, guess)
    } else {
        base
    }
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BreakdownPoints {
    exact_score_points: i64,
    outcome_points: i64,
    goal_bonus_points: i64,
    qualifier_points: i64,
    penalties_points: i64,
    total_points: i64,
}

#[cfg(feature = "server")]
#[derive(Debug, sqlx::FromRow)]
struct BreakdownRow {
    pool_id: String,
    user_id: String,
    match_id: String,
    joined_at: String,
    phase: Option<String>,
    kickoff: String,
    official_home_score: Option<i64>,
    official_away_score: Option<i64>,
    official_qualifier: Option<String>,
    official_went_to_penalties: bool,
    official_penalty_home_score: Option<i64>,
    official_penalty_away_score: Option<i64>,
    result_source: Option<String>,
    prediction_home_score: i64,
    prediction_away_score: i64,
    prediction_qualifier: Option<String>,
    prediction_went_to_penalties: bool,
    prediction_penalty_home_score: Option<i64>,
    prediction_penalty_away_score: Option<i64>,
}

#[cfg(feature = "server")]
#[derive(Debug, sqlx::FromRow)]
struct LiveOverlayRow {
    user_id: String,
    phase: Option<String>,
    kickoff: String,
    joined_at: String,
    live_home_score: i64,
    live_away_score: i64,
    p_home: i64,
    p_away: i64,
    p_qualifier: Option<String>,
    p_penalties: bool,
    p_pen_home: Option<i64>,
    p_pen_away: Option<i64>,
}

#[cfg(feature = "server")]
fn breakdown_points(is_knockout: bool, official: &Outcome, guess: &Outcome) -> BreakdownPoints {
    let exact_score_points = if guess.home_score == official.home_score && guess.away_score == official.away_score {
        7
    } else {
        0
    };

    let correct_outcome = (guess.home_score > guess.away_score && official.home_score > official.away_score)
        || (guess.home_score < guess.away_score && official.home_score < official.away_score)
        || (guess.home_score == guess.away_score && official.home_score == official.away_score);

    let outcome_points = if exact_score_points > 0 {
        0
    } else if correct_outcome {
        3
    } else {
        0
    };

    let goal_bonus_points = if exact_score_points > 0 {
        0
    } else if correct_outcome
        && ((guess.home_score == official.home_score && official.home_score > 0)
            || (guess.away_score == official.away_score && official.away_score > 0))
    {
        1
    } else {
        0
    };

    // O classificado deixa de ter bônus próprio (deduzido do placar/pênaltis).
    let qualifier_points = 0;

    let penalties_points = if is_knockout {
        knockout_bonus(official, guess)
    } else {
        0
    };

    BreakdownPoints {
        exact_score_points,
        outcome_points,
        goal_bonus_points,
        qualifier_points,
        penalties_points,
        total_points: exact_score_points
            + outcome_points
            + goal_bonus_points
            + qualifier_points
            + penalties_points,
    }
}

#[cfg(feature = "server")]
fn build_eligibility(row: &BreakdownRow) -> (bool, String) {
    let joined_at = chrono::NaiveDateTime::parse_from_str(&row.joined_at, "%Y-%m-%d %H:%M:%S").ok();
    let kickoff = chrono::DateTime::parse_from_rfc3339(&row.kickoff).ok();
    match (joined_at, kickoff) {
        (Some(joined), Some(kickoff)) => {
            let joined = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(joined, chrono::Utc);
            let kickoff = kickoff.with_timezone(&chrono::Utc);
            if kickoff >= joined {
                (true, "eligible".to_string())
            } else {
                (false, "joined_after_kickoff".to_string())
            }
        }
        _ => (false, "invalid_dates".to_string()),
    }
}

#[cfg(feature = "server")]
async fn create_scoring_job(
    db: &sqlx::SqlitePool,
    scope_type: &str,
    scope_id: Option<&str>,
    triggered_by: Option<&str>,
) -> Result<String, ServerFnError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO scoring_jobs (id, scope_type, scope_id, triggered_by, status, summary_json)
         VALUES (?1, ?2, ?3, ?4, 'running', '{}')",
    )
    .bind(&job_id)
    .bind(scope_type)
    .bind(scope_id)
    .bind(triggered_by)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("create_scoring_job", e))?;
    Ok(job_id)
}

#[cfg(feature = "server")]
async fn finish_scoring_job(
    db: &sqlx::SqlitePool,
    job_id: &str,
    status: &str,
    summary_json: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "UPDATE scoring_jobs
         SET status = ?1, finished_at = datetime('now'), summary_json = ?2
         WHERE id = ?3",
    )
    .bind(status)
    .bind(summary_json.to_string())
    .bind(job_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("finish_scoring_job", e))?;
    Ok(())
}

#[cfg(feature = "server")]
async fn recompute_breakdowns(
    db: &sqlx::SqlitePool,
    where_sql: &str,
    binds: &[String],
    scope_type: &str,
    scope_id: Option<&str>,
    triggered_by: Option<&str>,
) -> Result<ScoringJob, ServerFnError> {
    use crate::models::is_knockout;

    let job_id = create_scoring_job(db, scope_type, scope_id, triggered_by).await?;
    let delete_sql = format!(
        "DELETE FROM prediction_score_breakdowns
         WHERE EXISTS (
            SELECT 1
            FROM pool_members pm
            JOIN predictions pr ON pr.user_id = pm.user_id
            JOIN matches m ON m.id = pr.match_id
            WHERE prediction_score_breakdowns.pool_id = pm.pool_id
              AND prediction_score_breakdowns.user_id = pm.user_id
              AND prediction_score_breakdowns.match_id = m.id
              {where_sql}
         )"
    );
    let mut delete_query = sqlx::query(&delete_sql);
    for value in binds {
        delete_query = delete_query.bind(value);
    }
    delete_query
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("recompute_breakdowns_delete", e))?;

    let select_sql = format!(
        "SELECT pm.pool_id,
                pm.user_id,
                pr.match_id,
                pm.joined_at,
                m.phase,
                m.kickoff,
                m.home_score AS official_home_score,
                m.away_score AS official_away_score,
                m.qualifier AS official_qualifier,
                m.went_to_penalties AS official_went_to_penalties,
                m.penalty_home_score AS official_penalty_home_score,
                m.penalty_away_score AS official_penalty_away_score,
                m.result_source,
                pr.home_score AS prediction_home_score,
                pr.away_score AS prediction_away_score,
                pr.qualifier AS prediction_qualifier,
                pr.went_to_penalties AS prediction_went_to_penalties,
                pr.penalty_home_score AS prediction_penalty_home_score,
                pr.penalty_away_score AS prediction_penalty_away_score
         FROM pool_members pm
         JOIN predictions pr ON pr.user_id = pm.user_id
         JOIN matches m ON m.id = pr.match_id
         WHERE 1 = 1
           {where_sql}"
    );
    let mut select_query = sqlx::query_as::<_, BreakdownRow>(&select_sql);
    for value in binds {
        select_query = select_query.bind(value);
    }
    let rows = select_query
        .fetch_all(db)
        .await
        .map_err(|e| crate::security::internal_error("recompute_breakdowns_select", e))?;

    let mut inserted = 0_i64;
    for row in rows {
        let (eligible, eligibility_reason) = build_eligibility(&row);
        let (points, official_source) = if let (Some(home), Some(away)) = (row.official_home_score, row.official_away_score) {
            let official = Outcome {
                home_score: home,
                away_score: away,
                qualifier: row.official_qualifier.clone(),
                went_to_penalties: row.official_went_to_penalties,
                penalty_home: row.official_penalty_home_score,
                penalty_away: row.official_penalty_away_score,
            };
            let guess = Outcome {
                home_score: row.prediction_home_score,
                away_score: row.prediction_away_score,
                qualifier: row.prediction_qualifier.clone(),
                went_to_penalties: row.prediction_went_to_penalties,
                penalty_home: row.prediction_penalty_home_score,
                penalty_away: row.prediction_penalty_away_score,
            };
            (
                breakdown_points(is_knockout(row.phase.as_deref()), &official, &guess),
                row.result_source.clone(),
            )
        } else {
            (
                BreakdownPoints {
                    exact_score_points: 0,
                    outcome_points: 0,
                    goal_bonus_points: 0,
                    qualifier_points: 0,
                    penalties_points: 0,
                    total_points: 0,
                },
                row.result_source.clone(),
            )
        };

        sqlx::query(
            "INSERT INTO prediction_score_breakdowns
                (id, pool_id, user_id, match_id, exact_score_points, outcome_points, goal_bonus_points,
                 qualifier_points, penalties_points, total_points, eligible, eligibility_reason,
                 official_source, computed_at, job_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'), ?14)
             ON CONFLICT(pool_id, user_id, match_id) DO UPDATE SET
                exact_score_points = excluded.exact_score_points,
                outcome_points = excluded.outcome_points,
                goal_bonus_points = excluded.goal_bonus_points,
                qualifier_points = excluded.qualifier_points,
                penalties_points = excluded.penalties_points,
                total_points = excluded.total_points,
                eligible = excluded.eligible,
                eligibility_reason = excluded.eligibility_reason,
                official_source = excluded.official_source,
                computed_at = excluded.computed_at,
                job_id = excluded.job_id",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&row.pool_id)
        .bind(&row.user_id)
        .bind(&row.match_id)
        .bind(points.exact_score_points)
        .bind(points.outcome_points)
        .bind(points.goal_bonus_points)
        .bind(points.qualifier_points)
        .bind(points.penalties_points)
        .bind(points.total_points)
        .bind(eligible)
        .bind(&eligibility_reason)
        .bind(official_source)
        .bind(&job_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("recompute_breakdowns_upsert", e))?;
        inserted += 1;
    }

    let summary = serde_json::json!({
        "rows_upserted": inserted,
        "scope_type": scope_type,
        "scope_id": scope_id,
    });
    finish_scoring_job(db, &job_id, "completed", summary).await?;

    Ok(ScoringJob {
        id: job_id,
        scope_type: scope_type.to_string(),
        scope_id: scope_id.map(ToOwned::to_owned),
        triggered_by: triggered_by.map(ToOwned::to_owned),
        status: "completed".to_string(),
        started_at: String::new(),
        finished_at: None,
        summary_json: serde_json::json!({
            "rows_upserted": inserted,
        })
        .to_string(),
    })
}

#[cfg(feature = "server")]
async fn ensure_breakdowns_seeded(db: &sqlx::SqlitePool) -> Result<(), ServerFnError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_score_breakdowns")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("ensure_breakdowns_seeded_count", e))?;
    if count.0 == 0 {
        let has_predictions: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM predictions")
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("ensure_breakdowns_seeded_predictions", e))?;
        if has_predictions.0 > 0 {
            let _ = recompute_breakdowns(db, "", &[], "all", None, None).await?;
        }
    }
    Ok(())
}

#[cfg(feature = "server")]
pub async fn recalculate_match_breakdowns(
    match_id: &str,
    triggered_by: Option<&str>,
) -> Result<ScoringJob, ServerFnError> {
    let db = crate::db::pool();
    recompute_breakdowns(
        db,
        " AND pr.match_id = ?1",
        &[match_id.to_string()],
        "match",
        Some(match_id),
        triggered_by,
    )
    .await
}

#[cfg(feature = "server")]
pub async fn recalculate_all_breakdowns(triggered_by: Option<&str>) -> Result<ScoringJob, ServerFnError> {
    let db = crate::db::pool();
    sqlx::query("DELETE FROM prediction_score_breakdowns")
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("recalculate_all_breakdowns_clear", e))?;
    recompute_breakdowns(db, "", &[], "all", None, triggered_by).await
}

#[cfg(feature = "server")]
pub async fn recalculate_pool_user_breakdowns(
    pool_id: &str,
    user_id: &str,
    triggered_by: Option<&str>,
) -> Result<ScoringJob, ServerFnError> {
    let db = crate::db::pool();
    recompute_breakdowns(
        db,
        " AND pm.pool_id = ?1 AND pm.user_id = ?2",
        &[pool_id.to_string(), user_id.to_string()],
        "pool_user",
        Some(pool_id),
        triggered_by,
    )
    .await
}

#[cfg(feature = "server")]
pub async fn list_user_breakdowns(
    user_id: &str,
    pool_id: &str,
) -> Result<Vec<PredictionScoreBreakdown>, ServerFnError> {
    let db = crate::db::pool();
    ensure_breakdowns_seeded(db).await?;
    let rows = sqlx::query_as::<_, PredictionScoreBreakdown>(
        "SELECT b.pool_id AS pool_id,
                p.name AS pool_name,
                b.user_id AS user_id,
                u.username AS username,
                b.match_id AS match_id,
                m.home_team AS home_team,
                m.away_team AS away_team,
                b.exact_score_points AS exact_score_points,
                b.outcome_points AS outcome_points,
                b.goal_bonus_points AS goal_bonus_points,
                b.qualifier_points AS qualifier_points,
                b.penalties_points AS penalties_points,
                b.total_points AS total_points,
                b.eligible AS eligible,
                b.eligibility_reason AS eligibility_reason,
                b.official_source AS official_source,
                b.computed_at AS computed_at
         FROM prediction_score_breakdowns b
         JOIN pools p ON p.id = b.pool_id
         JOIN users u ON u.id = b.user_id
         JOIN matches m ON m.id = b.match_id
         WHERE b.user_id = ?1 AND b.pool_id = ?2
         ORDER BY datetime(m.kickoff) ASC",
    )
    .bind(user_id)
    .bind(pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_user_breakdowns", e))?;
    Ok(rows)
}

/// Breakdown de pontos de TODOS os membros de um bolão, para a tela "Palpites do
/// Bolão". Exige que o solicitante seja membro do bolão. Aqui a elegibilidade é
/// específica do bolão (quem entrou após o kickoff tem `eligible=false`).
#[cfg(feature = "server")]
pub async fn list_pool_breakdowns(
    pool_id: &str,
) -> Result<Vec<PredictionScoreBreakdown>, ServerFnError> {
    use crate::auth::require_user;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", pool_id)?;
    let session = require_user("").await?;
    let db = crate::db::pool();

    let membership: Option<(String,)> = sqlx::query_as(
        "SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2",
    )
    .bind(pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("list_pool_breakdowns_membership", e))?;
    if membership.is_none() {
        return Err(crate::security::public_error("Voce nao e membro deste bolao."));
    }

    ensure_breakdowns_seeded(db).await?;
    let rows = sqlx::query_as::<_, PredictionScoreBreakdown>(
        "SELECT b.pool_id AS pool_id,
                p.name AS pool_name,
                b.user_id AS user_id,
                u.username AS username,
                b.match_id AS match_id,
                m.home_team AS home_team,
                m.away_team AS away_team,
                b.exact_score_points AS exact_score_points,
                b.outcome_points AS outcome_points,
                b.goal_bonus_points AS goal_bonus_points,
                b.qualifier_points AS qualifier_points,
                b.penalties_points AS penalties_points,
                b.total_points AS total_points,
                b.eligible AS eligible,
                b.eligibility_reason AS eligibility_reason,
                b.official_source AS official_source,
                b.computed_at AS computed_at
         FROM prediction_score_breakdowns b
         JOIN pools p ON p.id = b.pool_id
         JOIN users u ON u.id = b.user_id
         JOIN matches m ON m.id = b.match_id
         WHERE b.pool_id = ?1
         ORDER BY datetime(m.kickoff) ASC",
    )
    .bind(pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_pool_breakdowns", e))?;
    Ok(rows)
}

/// Pontos que o usuário logado fez em cada jogo, para exibir no card de palpite.
/// Escopado à sessão (ignora qualquer id do cliente) e colapsado por jogo: como
/// os componentes só dependem do palpite vs resultado, são idênticos entre os
/// bolões do usuário — `MAX` colapsa as linhas e `MAX(eligible)` indica se conta
/// em ao menos um bolão.
#[cfg(feature = "server")]
pub async fn list_my_match_points() -> Result<Vec<MatchPointsSummary>, ServerFnError> {
    use crate::auth::require_user;

    crate::security::apply_security_headers();
    let session = require_user("").await?;
    let db = crate::db::pool();
    ensure_breakdowns_seeded(db).await?;
    let rows = sqlx::query_as::<_, MatchPointsSummary>(
        "SELECT b.match_id AS match_id,
                MAX(b.exact_score_points) AS exact_score_points,
                MAX(b.outcome_points) AS outcome_points,
                MAX(b.goal_bonus_points) AS goal_bonus_points,
                MAX(b.qualifier_points) AS qualifier_points,
                MAX(b.penalties_points) AS penalties_points,
                MAX(b.total_points) AS total_points,
                MAX(b.eligible) AS eligible
         FROM prediction_score_breakdowns b
         WHERE b.user_id = ?1
         GROUP BY b.match_id",
    )
    .bind(&session.user_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("list_my_match_points", e))?;
    Ok(rows)
}

/// Acumulador por usuário no ranking: total de pontos e os critérios de
/// desempate (placares exatos, acertos de resultado e bônus de precisão).
#[cfg(feature = "server")]
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

    let membership: Option<(String,)> = sqlx::query_as(
        "SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("get_leaderboard_membership", e))?;

    if membership.is_none() {
        return Err(crate::security::public_error("Voce nao e membro deste bolao."));
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

    // Overlay provisório: jogos ao vivo sem resultado oficial ainda não entram
    // na materialização, então somamos sob demanda para preservar o feedback do
    // ranking durante a partida.
    let live_rows = sqlx::query_as::<_, LiveOverlayRow>(
        "SELECT pm.user_id AS user_id,
                m.phase AS phase,
                m.kickoff AS kickoff,
                pm.joined_at AS joined_at,
                m.live_home_score AS live_home_score,
                m.live_away_score AS live_away_score,
                pr.home_score AS p_home,
                pr.away_score AS p_away,
                pr.qualifier AS p_qualifier,
                pr.went_to_penalties AS p_penalties,
                pr.penalty_home_score AS p_pen_home,
                pr.penalty_away_score AS p_pen_away
         FROM pool_members pm
         JOIN predictions pr ON pr.user_id = pm.user_id
         JOIN matches m ON m.id = pr.match_id
         WHERE pm.pool_id = ?1
           AND m.home_score IS NULL
           AND m.away_score IS NULL
           AND m.finished = 0
           AND m.live_home_score IS NOT NULL
           AND m.live_away_score IS NOT NULL",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_leaderboard_live_overlay", e))?;

    for row in live_rows {
        let joined = chrono::NaiveDateTime::parse_from_str(&row.joined_at, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc));
        let kickoff = chrono::DateTime::parse_from_rfc3339(&row.kickoff)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc));
        if matches!((joined, kickoff), (Some(joined), Some(kickoff)) if kickoff < joined) {
            continue;
        }

        let official = Outcome {
            home_score: row.live_home_score,
            away_score: row.live_away_score,
            qualifier: None,
            went_to_penalties: false,
            penalty_home: None,
            penalty_away: None,
        };
        let guess = Outcome {
            home_score: row.p_home,
            away_score: row.p_away,
            qualifier: row.p_qualifier,
            went_to_penalties: row.p_penalties,
            penalty_home: row.p_pen_home,
            penalty_away: row.p_pen_away,
        };
        let overlay = breakdown_points(crate::models::is_knockout(row.phase.as_deref()), &official, &guess);
        let t = tallies.entry(row.user_id).or_default();
        t.points += overlay.total_points;
        if overlay.exact_score_points > 0 {
            t.exact_scores += 1;
        }
        if overlay.exact_score_points > 0 || overlay.outcome_points > 0 {
            t.correct_results += 1;
        }
        t.bonus_points += overlay.goal_bonus_points + overlay.qualifier_points + overlay.penalties_points;
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

#[cfg(test)]
mod tests {
    use super::{match_points, rank_leaderboard, Outcome};
    use crate::models::LeaderboardEntry;

    fn entry(
        username: &str,
        points: i64,
        exact_scores: i64,
        correct_results: i64,
        bonus_points: i64,
    ) -> LeaderboardEntry {
        LeaderboardEntry {
            user_id: username.to_string(),
            username: username.to_string(),
            points,
            exact_scores,
            correct_results,
            bonus_points,
        }
    }

    fn order(entries: &mut Vec<LeaderboardEntry>) -> Vec<String> {
        rank_leaderboard(entries);
        entries.iter().map(|e| e.username.clone()).collect()
    }

    // Mais pontos sempre vem primeiro, independente dos critérios de desempate.
    #[test]
    fn ranks_by_points_first() {
        let mut entries = vec![
            entry("ana", 10, 0, 0, 0),
            entry("bia", 20, 0, 0, 0),
        ];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    // Empate em pontos → quem tem mais placares exatos sobe.
    #[test]
    fn breaks_tie_by_exact_scores() {
        let mut entries = vec![
            entry("ana", 30, 2, 5, 1),
            entry("bia", 30, 3, 4, 0),
        ];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    // Empate em pontos e placares exatos → mais acertos de resultado.
    #[test]
    fn breaks_tie_by_correct_results() {
        let mut entries = vec![
            entry("ana", 30, 2, 4, 5),
            entry("bia", 30, 2, 6, 0),
        ];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    // Empate em pontos, exatos e resultados → mais bônus de precisão.
    #[test]
    fn breaks_tie_by_bonus_points() {
        let mut entries = vec![
            entry("ana", 30, 2, 5, 1),
            entry("bia", 30, 2, 5, 4),
        ];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    // Empate total → ordem alfabética determinística.
    #[test]
    fn breaks_full_tie_by_username() {
        let mut entries = vec![
            entry("bia", 30, 2, 5, 1),
            entry("ana", 30, 2, 5, 1),
        ];
        assert_eq!(order(&mut entries), vec!["ana", "bia"]);
    }

    fn group(home: i64, away: i64) -> Outcome {
        Outcome {
            home_score: home,
            away_score: away,
            qualifier: None,
            went_to_penalties: false,
            penalty_home: None,
            penalty_away: None,
        }
    }

    fn ko(
        home: i64,
        away: i64,
        qualifier: &str,
        penalties: bool,
        pens: Option<(i64, i64)>,
    ) -> Outcome {
        Outcome {
            home_score: home,
            away_score: away,
            qualifier: Some(qualifier.to_string()),
            went_to_penalties: penalties,
            penalty_home: pens.map(|(h, _)| h),
            penalty_away: pens.map(|(_, a)| a),
        }
    }

    // Fase de grupos — resultado real Brasil 2x1 Japão.
    #[test]
    fn group_stage_brazil_2x1() {
        let real = group(2, 1);
        assert_eq!(match_points(false, &real, &group(2, 1)), 7); // exato
        assert_eq!(match_points(false, &real, &group(2, 0)), 4); // vencedor + gols Brasil
        assert_eq!(match_points(false, &real, &group(3, 1)), 4); // vencedor + gols Japão
        assert_eq!(match_points(false, &real, &group(1, 0)), 3); // só vencedor
        assert_eq!(match_points(false, &real, &group(4, 2)), 3); // só vencedor
        assert_eq!(match_points(false, &real, &group(1, 2)), 0); // errou vencedor
        assert_eq!(match_points(false, &real, &group(1, 1)), 0); // empate errado
    }

    // Fase de grupos — resultado real França 0x0 Canadá.
    #[test]
    fn group_stage_draw_0x0() {
        let real = group(0, 0);
        assert_eq!(match_points(false, &real, &group(0, 0)), 7); // exato
        assert_eq!(match_points(false, &real, &group(1, 1)), 3); // acertou empate
        assert_eq!(match_points(false, &real, &group(2, 2)), 3); // acertou empate
        assert_eq!(match_points(false, &real, &group(0, 1)), 0);
        assert_eq!(match_points(false, &real, &group(1, 0)), 0);
    }

    // Mata-mata — vitória no tempo normal: Brasil 2x0 México.
    // Sem empate, não há pênaltis: vale apenas a pontuação base.
    #[test]
    fn knockout_normal_win() {
        let real = ko(2, 0, "home", false, None);
        assert_eq!(match_points(true, &real, &ko(2, 0, "home", false, None)), 7); // placar exato
        assert_eq!(match_points(true, &real, &ko(3, 0, "home", false, None)), 3); // só vencedor (gols 0 não contam)
        assert_eq!(match_points(true, &real, &ko(2, 1, "home", false, None)), 4); // vencedor + gols mandante
        assert_eq!(match_points(true, &real, &ko(1, 0, "home", false, None)), 3); // só vencedor
        assert_eq!(match_points(true, &real, &ko(1, 1, "home", false, None)), 0); // palpitou empate, deu vitória
        assert_eq!(match_points(true, &real, &ko(0, 1, "away", false, None)), 0); // errou o vencedor
    }

    // Mata-mata — empate decidido nos pênaltis: Brasil 1x1 Argentina,
    // pênaltis 5x4 (mandante avança).
    #[test]
    fn knockout_penalties() {
        let real = ko(1, 1, "home", true, Some((5, 4)));
        // Placar exato 1x1 (7) + pênaltis exatos 5x4 (+3) = 10.
        assert_eq!(match_points(true, &real, &ko(1, 1, "home", true, Some((5, 4)))), 10);
        // Placar exato 1x1 (7) + só o vencedor dos pênaltis (+1) = 8.
        assert_eq!(match_points(true, &real, &ko(1, 1, "home", true, Some((4, 3)))), 8);
        // Placar exato 1x1 (7) + errou o vencedor dos pênaltis (0) = 7.
        assert_eq!(match_points(true, &real, &ko(1, 1, "home", true, Some((3, 5)))), 7);
        // Empate certo não exato (3): sem bônus de pênaltis mesmo acertando o placar/vencedor.
        assert_eq!(match_points(true, &real, &ko(2, 2, "home", true, Some((5, 4)))), 3);
        assert_eq!(match_points(true, &real, &ko(2, 2, "home", true, Some((4, 3)))), 3);
        // Palpitou vitória: errou o resultado (era empate) = 0.
        assert_eq!(match_points(true, &real, &ko(2, 1, "home", false, None)), 0);
        assert_eq!(match_points(true, &real, &ko(2, 1, "away", false, None)), 0);
    }
}
