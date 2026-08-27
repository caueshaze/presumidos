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
