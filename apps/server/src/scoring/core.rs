#[cfg(any(feature = "server", test))]
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub home_score: i64,
    pub away_score: i64,
    pub qualifier: Option<String>,
    pub went_to_penalties: bool,
    pub penalty_home: Option<i64>,
    pub penalty_away: Option<i64>,
}

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
    let goal_bonus =
        (guess_home == real_home && real_home > 0) || (guess_away == real_away && real_away > 0);
    if goal_bonus {
        4
    } else {
        3
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[cfg(any(feature = "server", test))]
pub fn knockout_bonus(official: &Outcome, guess: &Outcome) -> i64 {
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
    let exact_base =
        guess.home_score == official.home_score && guess.away_score == official.away_score;
    let correct_winner = (gh > ga) == (oh > oa);
    let exact_penalties = gh == oh && ga == oa;
    if exact_base && exact_penalties {
        3
    } else if exact_base && correct_winner {
        2
    } else if correct_winner {
        1
    } else {
        0
    }
}

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

#[cfg(test)]
mod tests {
    use crate::models::LeaderboardEntry;
    use crate::scoring::{base_points, match_points, rank_leaderboard, Outcome};

    fn entry(
        username: &str,
        points: i64,
        exact_scores: i64,
        correct_results: i64,
        bonus_points: i64,
    ) -> LeaderboardEntry {
        LeaderboardEntry {
            position: 0,
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
        let mut entries = vec![entry("ana", 10, 0, 0, 0), entry("bia", 20, 0, 0, 0)];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    #[test]
    fn equal_business_criteria_share_competition_position() {
        let mut entries = vec![
            entry("Cauê", 10, 1, 2, 0),
            entry("Ana", 10, 1, 2, 0),
            entry("Bruno", 9, 0, 0, 0),
        ];
        rank_leaderboard(&mut entries);
        assert_eq!(order(&mut entries), vec!["Ana", "Cauê", "Bruno"]);
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.position)
                .collect::<Vec<_>>(),
            vec![1, 1, 3]
        );
    }

    // Empate em pontos → quem tem mais placares exatos sobe.
    #[test]
    fn breaks_tie_by_exact_scores() {
        let mut entries = vec![entry("ana", 30, 2, 5, 1), entry("bia", 30, 3, 4, 0)];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    // Empate em pontos e placares exatos → mais acertos de resultado.
    #[test]
    fn breaks_tie_by_correct_results() {
        let mut entries = vec![entry("ana", 30, 2, 4, 5), entry("bia", 30, 2, 6, 0)];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    // Empate em pontos, exatos e resultados → mais bônus de precisão.
    #[test]
    fn breaks_tie_by_bonus_points() {
        let mut entries = vec![entry("ana", 30, 2, 5, 1), entry("bia", 30, 2, 5, 4)];
        assert_eq!(order(&mut entries), vec!["bia", "ana"]);
    }

    // Empate total → ordem alfabética determinística.
    #[test]
    fn breaks_full_tie_by_username() {
        let mut entries = vec![entry("bia", 30, 2, 5, 1), entry("ana", 30, 2, 5, 1)];
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

    /// Matriz mínima das faixas da fase de grupos. Mantém explícitos os casos
    /// assimétricos (vitória visitante) e o detalhe de que gol zero não gera o
    /// ponto adicional.
    #[test]
    fn group_stage_scoring_bands_cover_home_away_draw_and_zero_goals() {
        assert_eq!(base_points(2, 1, 2, 1), 7, "placar exato");
        assert_eq!(
            base_points(0, 2, 1, 2),
            4,
            "vitória visitante e gols visitantes exatos"
        );
        assert_eq!(
            base_points(1, 3, 2, 4),
            3,
            "vitória visitante sem gol exato"
        );
        assert_eq!(
            base_points(3, 3, 0, 0),
            3,
            "empate correto sem bônus para zero gol"
        );
        assert_eq!(base_points(0, 1, 1, 0), 0, "resultado incorreto");
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
        assert_eq!(match_points(true, &real, &ko(0, 1, "away", false, None)), 0);
        // errou o vencedor
    }

    // Mata-mata — empate decidido nos pênaltis: Brasil 1x1 Argentina,
    // pênaltis 5x4 (mandante avança).
    #[test]
    fn knockout_penalties() {
        let real = ko(1, 1, "home", true, Some((5, 4)));
        // Placar exato 1x1 (7) + pênaltis exatos 5x4 (+3) = 10.
        assert_eq!(
            match_points(true, &real, &ko(1, 1, "home", true, Some((5, 4)))),
            10
        );
        // Placar exato 1x1 (7) + vencedor dos pênaltis (+2) = 9.
        assert_eq!(
            match_points(true, &real, &ko(1, 1, "home", true, Some((4, 3)))),
            9
        );
        // Placar exato 1x1 (7) + errou o vencedor dos pênaltis (0) = 7.
        assert_eq!(
            match_points(true, &real, &ko(1, 1, "home", true, Some((3, 5)))),
            7
        );
        // Empate não exato (3) + vencedor certo dos pênaltis (+1) = 4.
        assert_eq!(
            match_points(true, &real, &ko(2, 2, "home", true, Some((5, 4)))),
            4
        );
        assert_eq!(
            match_points(true, &real, &ko(2, 2, "home", true, Some((4, 3)))),
            4
        );
        // Empate não exato (3) + errou o vencedor (0) = 3.
        assert_eq!(
            match_points(true, &real, &ko(2, 2, "away", true, Some((3, 5)))),
            3
        );
        // Palpitou vitória: errou o resultado (era empate) = 0.
        assert_eq!(match_points(true, &real, &ko(2, 1, "home", false, None)), 0);
        assert_eq!(match_points(true, &real, &ko(2, 1, "away", false, None)), 0);
    }
}
