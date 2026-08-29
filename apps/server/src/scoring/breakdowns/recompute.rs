use crate::{
    error::ServerFnError,
    models::{FootballScoringConfig, ScoringJob},
    scoring::{
        breakdowns::calculation::{
            breakdown_points, build_eligibility, BreakdownPoints, BreakdownRow,
        },
        jobs::{create_scoring_job, finish_scoring_job},
        Outcome,
    },
};

#[cfg(feature = "server")]
pub(crate) async fn recompute_breakdowns(
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
            JOIN predictions pr ON pr.user_id = pm.user_id AND pr.pool_id = pm.pool_id
            JOIN matches m ON m.id = pr.match_id AND m.prediction_item_id = pr.item_id
            JOIN prediction_items pi ON pi.id = pr.item_id
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
                pi.lock_at,
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
                pr.penalty_away_score AS prediction_penalty_away_score,
                f.exact_score_points AS exact_score_points_config,
                f.correct_result_exact_side_points AS correct_result_exact_side_points_config,
                f.correct_result_points AS correct_result_points_config,
                f.incorrect_result_points AS incorrect_result_points_config,
                f.knockout_bonus_points AS knockout_bonus_points_config
         FROM pool_members pm
         JOIN predictions pr ON pr.user_id = pm.user_id AND pr.pool_id = pm.pool_id
         JOIN matches m ON m.id = pr.match_id AND m.prediction_item_id = pr.item_id
         JOIN prediction_items pi ON pi.id = pr.item_id
         JOIN football_pool_scoring f ON f.pool_id = pm.pool_id
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
        let (points, official_source) =
            if let (Some(home), Some(away)) = (row.official_home_score, row.official_away_score) {
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
                    breakdown_points(
                        is_knockout(row.phase.as_deref()),
                        &official,
                        &guess,
                        &FootballScoringConfig {
                            exact_score_points: row.exact_score_points_config,
                            correct_result_exact_side_points: row
                                .correct_result_exact_side_points_config,
                            correct_result_points: row.correct_result_points_config,
                            incorrect_result_points: row.incorrect_result_points_config,
                            knockout_bonus_points: row.knockout_bonus_points_config,
                        },
                    ),
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
                        incorrect_result_points: 0,
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
pub(crate) async fn ensure_breakdowns_seeded(db: &sqlx::SqlitePool) -> Result<(), ServerFnError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_score_breakdowns")
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("ensure_breakdowns_seeded_count", e))?;
    if count.0 == 0 {
        let has_predictions: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM predictions")
            .fetch_one(db)
            .await
            .map_err(|e| {
                crate::security::internal_error("ensure_breakdowns_seeded_predictions", e)
            })?;
        if has_predictions.0 > 0 {
            let _ = recompute_breakdowns(db, "", &[], "all", None, None).await?;
        }
    }
    Ok(())
}

#[cfg(feature = "server")]
pub(crate) async fn recompute_custom_breakdowns(
    db: &sqlx::SqlitePool,
) -> Result<(), ServerFnError> {
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("custom_scoring_begin", e))?;
    sqlx::query("DELETE FROM custom_prediction_score_breakdowns")
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("custom_scoring_clear", e))?;
    sqlx::query(
        "INSERT INTO custom_prediction_score_breakdowns (id,pool_id,user_id,item_id,correct_points,incorrect_points,total_points,eligible,eligibility_reason)
         SELECT lower(hex(randomblob(16))), p.pool_id,p.user_id,p.item_id,
           CASE WHEN v.option_id=q.correct_option_id THEN c.correct_points ELSE 0 END,
           CASE WHEN v.option_id=q.correct_option_id THEN 0 ELSE c.incorrect_points END,
           CASE WHEN v.option_id=q.correct_option_id THEN c.correct_points ELSE c.incorrect_points END,
           CASE WHEN datetime(pi.lock_at) >= datetime(pm.joined_at) THEN 1 ELSE 0 END,
           CASE WHEN datetime(pi.lock_at) >= datetime(pm.joined_at) THEN 'eligible' ELSE 'joined_after_lock' END
         FROM predictions p JOIN custom_prediction_values v ON v.prediction_id=p.id
         JOIN custom_questions q ON q.item_id=p.item_id JOIN prediction_items pi ON pi.id=p.item_id
         JOIN custom_pool_item_scoring c ON c.pool_id=p.pool_id AND c.item_id=p.item_id
         JOIN pool_members pm ON pm.pool_id=p.pool_id AND pm.user_id=p.user_id
         WHERE q.correct_option_id IS NOT NULL",
    ).execute(&mut *tx).await.map_err(|e| crate::security::internal_error("custom_scoring_insert", e))?;
    sqlx::query("DELETE FROM numeric_prediction_score_breakdowns")
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("numeric_scoring_clear", e))?;
    sqlx::query(
        "INSERT INTO numeric_prediction_score_breakdowns (id,pool_id,user_id,item_id,predicted_value_scaled,official_value_scaled,difference_scaled,outcome,exact_points,tolerance_scaled,within_tolerance_points,incorrect_points,total_points,eligible,eligibility_reason)
         SELECT lower(hex(randomblob(16))),p.pool_id,p.user_id,p.item_id,v.value_scaled,q.result_value_scaled,abs(v.value_scaled-q.result_value_scaled),
           CASE WHEN v.value_scaled=q.result_value_scaled THEN 'exact' WHEN abs(v.value_scaled-q.result_value_scaled)<=c.tolerance_scaled THEN 'within_tolerance' ELSE 'incorrect' END,
           c.exact_points,c.tolerance_scaled,c.within_tolerance_points,c.incorrect_points,
           CASE WHEN v.value_scaled=q.result_value_scaled THEN c.exact_points WHEN abs(v.value_scaled-q.result_value_scaled)<=c.tolerance_scaled THEN c.within_tolerance_points ELSE c.incorrect_points END,
           CASE WHEN datetime(pi.lock_at)>=datetime(pm.joined_at) THEN 1 ELSE 0 END,
           CASE WHEN datetime(pi.lock_at)>=datetime(pm.joined_at) THEN 'eligible' ELSE 'joined_after_lock' END
         FROM predictions p JOIN numeric_prediction_values v ON v.prediction_id=p.id
         JOIN numeric_questions q ON q.item_id=p.item_id JOIN prediction_items pi ON pi.id=p.item_id
         JOIN numeric_pool_item_scoring c ON c.pool_id=p.pool_id AND c.item_id=p.item_id
         JOIN pool_members pm ON pm.pool_id=p.pool_id AND pm.user_id=p.user_id
         WHERE q.result_value_scaled IS NOT NULL"
    ).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("numeric_scoring_insert",e))?;
    sqlx::query("DELETE FROM multiple_choice_prediction_score_breakdowns")
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_scoring_clear", e))?;
    sqlx::query(
        "INSERT INTO multiple_choice_prediction_score_breakdowns (id,pool_id,user_id,item_id,outcome,selected_count,correct_count,intersection_count,exact_points,partial_points,incorrect_points,total_points,eligible,eligibility_reason)
         SELECT lower(hex(randomblob(16))),p.pool_id,p.user_id,p.item_id,
           CASE WHEN COUNT(sel.option_id)=(SELECT COUNT(*) FROM multiple_choice_results all_res WHERE all_res.item_id=p.item_id) AND COUNT(sel.option_id)=SUM(CASE WHEN res.option_id IS NOT NULL THEN 1 ELSE 0 END) THEN 'exact'
                WHEN COUNT(sel.option_id)>0 AND COUNT(sel.option_id)=SUM(CASE WHEN res.option_id IS NOT NULL THEN 1 ELSE 0 END) THEN 'partial' ELSE 'incorrect' END,
           COUNT(sel.option_id),(SELECT COUNT(*) FROM multiple_choice_results all_res WHERE all_res.item_id=p.item_id),SUM(CASE WHEN res.option_id IS NOT NULL THEN 1 ELSE 0 END),
           c.exact_points,c.partial_points,c.incorrect_points,
           CASE WHEN COUNT(sel.option_id)=(SELECT COUNT(*) FROM multiple_choice_results all_res WHERE all_res.item_id=p.item_id) AND COUNT(sel.option_id)=SUM(CASE WHEN res.option_id IS NOT NULL THEN 1 ELSE 0 END) THEN c.exact_points
                WHEN COUNT(sel.option_id)>0 AND COUNT(sel.option_id)=SUM(CASE WHEN res.option_id IS NOT NULL THEN 1 ELSE 0 END) THEN c.partial_points ELSE c.incorrect_points END,
           CASE WHEN datetime(pi.lock_at)>=datetime(pm.joined_at) THEN 1 ELSE 0 END,
           CASE WHEN datetime(pi.lock_at)>=datetime(pm.joined_at) THEN 'eligible' ELSE 'joined_after_lock' END
         FROM predictions p JOIN multiple_choice_prediction_options sel ON sel.prediction_id=p.id
         JOIN prediction_items pi ON pi.id=p.item_id JOIN multiple_choice_pool_item_scoring c ON c.pool_id=p.pool_id AND c.item_id=p.item_id
         JOIN pool_members pm ON pm.pool_id=p.pool_id AND pm.user_id=p.user_id
         LEFT JOIN multiple_choice_results res ON res.item_id=p.item_id AND res.option_id=sel.option_id
         WHERE EXISTS(SELECT 1 FROM multiple_choice_results r WHERE r.item_id=p.item_id)
         GROUP BY p.id,c.pool_id,c.item_id"
    ).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("multiple_choice_scoring_insert",e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("custom_scoring_commit", e))?;
    Ok(())
}
