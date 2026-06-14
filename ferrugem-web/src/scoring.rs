use crate::error::ServerFnError;

use crate::models::LeaderboardEntry;

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

/// Bônus extra do mata-mata, somado à pontuação base.
#[cfg(any(feature = "server", test))]
pub fn knockout_bonus(official: &Outcome, guess: &Outcome) -> i64 {
    let mut bonus = 0;

    let qualifier_ok = official.qualifier.is_some() && official.qualifier == guess.qualifier;
    if qualifier_ok {
        bonus += 2;
    }

    if official.went_to_penalties && guess.went_to_penalties {
        // Acertou que foi para os pênaltis.
        bonus += 1;

        // O classificado que palpitou passou justamente nos pênaltis.
        if qualifier_ok {
            bonus += 1;
        }

        // Placar exato dos pênaltis (firula opcional).
        if let (Some(gh), Some(ga), Some(oh), Some(oa)) = (
            guess.penalty_home,
            guess.penalty_away,
            official.penalty_home,
            official.penalty_away,
        ) {
            if gh == oh && ga == oa {
                bonus += 1;
            }
        }
    }

    bonus
}

/// Pontuação total de um palpite contra o resultado oficial de uma partida.
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

/// Calcula o ranking de um bolão somando a pontuação de cada palpite contra os
/// resultados oficiais já lançados.
#[cfg(feature = "server")]
pub async fn get_leaderboard(
    token: String,
    pool_id: String,
) -> Result<Vec<LeaderboardEntry>, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;
    use crate::models::is_knockout;
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

    // Palpites dos membros com o respectivo resultado oficial já lançado.
    #[derive(sqlx::FromRow)]
    struct ScoredRow {
        user_id: String,
        phase: Option<String>,
        m_home: i64,
        m_away: i64,
        m_qualifier: Option<String>,
        m_penalties: bool,
        m_pen_home: Option<i64>,
        m_pen_away: Option<i64>,
        p_home: i64,
        p_away: i64,
        p_qualifier: Option<String>,
        p_penalties: bool,
        p_pen_home: Option<i64>,
        p_pen_away: Option<i64>,
    }

    // Pontuação provisória ao vivo: além do resultado oficial, conta também o
    // placar parcial dos jogos em andamento (`live_*`). Para esses, usamos o
    // placar ao vivo como resultado corrente — os campos de mata-mata ficam
    // nulos, então o jogo ao vivo soma só a pontuação base (sem bônus de KO até
    // o resultado oficial). O total "trava" quando a partida encerra.
    let rows = sqlx::query_as::<_, ScoredRow>(
        "SELECT pm.user_id AS user_id,
                m.phase AS phase,
                COALESCE(m.home_score, m.live_home_score) AS m_home,
                COALESCE(m.away_score, m.live_away_score) AS m_away,
                m.qualifier AS m_qualifier,
                m.went_to_penalties AS m_penalties,
                m.penalty_home_score AS m_pen_home,
                m.penalty_away_score AS m_pen_away,
                pr.home_score AS p_home,
                pr.away_score AS p_away,
                pr.qualifier AS p_qualifier,
                pr.went_to_penalties AS p_penalties,
                pr.penalty_home_score AS p_pen_home,
                pr.penalty_away_score AS p_pen_away
         FROM pool_members pm
         JOIN predictions pr ON pr.user_id = pm.user_id
         JOIN matches m ON m.id = pr.match_id
                       AND (
                            (m.home_score IS NOT NULL AND m.away_score IS NOT NULL)
                            OR (m.finished = 0
                                AND m.live_home_score IS NOT NULL
                                AND m.live_away_score IS NOT NULL)
                       )
                       -- Só conta se o usuário já era membro quando a partida
                       -- começou: bloqueia pontuar palpites de jogos que
                       -- terminaram antes de ele entrar no bolão.
                       AND datetime(m.kickoff) >= datetime(pm.joined_at)
         WHERE pm.pool_id = ?1",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_leaderboard_scores", e))?;

    let mut points: HashMap<String, i64> = members
        .iter()
        .map(|(id, _)| (id.clone(), 0))
        .collect();

    for row in &rows {
        let official = Outcome {
            home_score: row.m_home,
            away_score: row.m_away,
            qualifier: row.m_qualifier.clone(),
            went_to_penalties: row.m_penalties,
            penalty_home: row.m_pen_home,
            penalty_away: row.m_pen_away,
        };
        let guess = Outcome {
            home_score: row.p_home,
            away_score: row.p_away,
            qualifier: row.p_qualifier.clone(),
            went_to_penalties: row.p_penalties,
            penalty_home: row.p_pen_home,
            penalty_away: row.p_pen_away,
        };
        let pts = match_points(is_knockout(row.phase.as_deref()), &official, &guess);
        *points.entry(row.user_id.clone()).or_insert(0) += pts;
    }

    // Ajustes manuais de pontos lançados pelo organizador (ou admin) somam ao total.
    let adjustments: Vec<(String, i64)> = sqlx::query_as(
        "SELECT user_id, SUM(delta) FROM point_adjustments WHERE pool_id = ?1 GROUP BY user_id",
    )
    .bind(&pool_id)
    .fetch_all(db)
    .await
    .map_err(|e| crate::security::internal_error("get_leaderboard_adjustments", e))?;

    for (user_id, total) in adjustments {
        if let Some(p) = points.get_mut(&user_id) {
            *p += total;
        }
    }

    let mut entries: Vec<LeaderboardEntry> = members
        .into_iter()
        .map(|(id, username)| LeaderboardEntry {
            points: points.get(&id).copied().unwrap_or(0),
            user_id: id,
            username,
        })
        .collect();

    entries.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then_with(|| a.username.cmp(&b.username))
    });

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{match_points, Outcome};

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

    // Mata-mata — vitória no tempo normal: Brasil 2x0 México, Brasil classifica.
    #[test]
    fn knockout_normal_win() {
        let real = ko(2, 0, "home", false, None);
        assert_eq!(match_points(true, &real, &ko(2, 0, "home", false, None)), 9); // 7 + 2
        assert_eq!(match_points(true, &real, &ko(3, 0, "home", false, None)), 5); // 3 + 2 (gols 0 não contam)
        assert_eq!(match_points(true, &real, &ko(2, 1, "home", false, None)), 6); // 3 + 1 gol + 2
        assert_eq!(match_points(true, &real, &ko(1, 0, "home", false, None)), 5); // 3 + 2
        assert_eq!(match_points(true, &real, &ko(1, 1, "home", false, None)), 2); // 0 + 2 classificado
        assert_eq!(match_points(true, &real, &ko(0, 1, "away", false, None)), 0); // visitante venceria — errou tudo
    }

    // Mata-mata — empate decidido nos pênaltis: Brasil 1x1 Argentina,
    // Brasil classifica nos pênaltis 5x4.
    #[test]
    fn knockout_penalties() {
        let real = ko(1, 1, "home", true, Some((5, 4)));
        assert_eq!(
            match_points(true, &real, &ko(1, 1, "home", true, Some((5, 4)))),
            12
        );
        assert_eq!(
            match_points(true, &real, &ko(1, 1, "home", true, Some((4, 3)))),
            11
        );
        assert_eq!(match_points(true, &real, &ko(1, 1, "home", true, None)), 11);
        assert_eq!(
            match_points(true, &real, &ko(2, 2, "home", true, Some((5, 4)))),
            8
        );
        assert_eq!(match_points(true, &real, &ko(0, 0, "home", true, None)), 7);
        assert_eq!(match_points(true, &real, &ko(1, 1, "away", true, None)), 8);
        assert_eq!(match_points(true, &real, &ko(2, 1, "home", false, None)), 2);
        assert_eq!(match_points(true, &real, &ko(2, 1, "away", false, None)), 0);
    }
}
