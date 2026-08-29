use crate::{
    error::ServerFnError,
    models::{MatchPointsSummary, PredictionScoreBreakdown},
    scoring::ensure_breakdowns_seeded,
};

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

    let membership: Option<(String,)> =
        sqlx::query_as("SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
            .bind(pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("list_pool_breakdowns_membership", e))?;
    if membership.is_none() {
        return Err(crate::security::public_error(
            "Voce nao e membro deste bolao.",
        ));
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
