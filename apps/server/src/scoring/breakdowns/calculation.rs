use crate::{
    models::FootballScoringConfig,
    scoring::{knockout_bonus, Outcome},
};

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BreakdownPoints {
    pub(super) exact_score_points: i64,
    pub(super) outcome_points: i64,
    pub(super) goal_bonus_points: i64,
    pub(super) qualifier_points: i64,
    pub(super) penalties_points: i64,
    pub(super) incorrect_result_points: i64,
    pub(super) total_points: i64,
}

#[cfg(feature = "server")]
#[derive(Debug, sqlx::FromRow)]
pub(super) struct BreakdownRow {
    pub(super) pool_id: String,
    pub(super) user_id: String,
    pub(super) match_id: String,
    pub(super) joined_at: String,
    pub(super) phase: Option<String>,
    pub(super) lock_at: String,
    pub(super) official_home_score: Option<i64>,
    pub(super) official_away_score: Option<i64>,
    pub(super) official_qualifier: Option<String>,
    pub(super) official_went_to_penalties: bool,
    pub(super) official_penalty_home_score: Option<i64>,
    pub(super) official_penalty_away_score: Option<i64>,
    pub(super) result_source: Option<String>,
    pub(super) prediction_home_score: i64,
    pub(super) prediction_away_score: i64,
    pub(super) prediction_qualifier: Option<String>,
    pub(super) prediction_went_to_penalties: bool,
    pub(super) prediction_penalty_home_score: Option<i64>,
    pub(super) prediction_penalty_away_score: Option<i64>,
    pub(super) exact_score_points_config: i64,
    pub(super) correct_result_exact_side_points_config: i64,
    pub(super) correct_result_points_config: i64,
    pub(super) incorrect_result_points_config: i64,
    pub(super) knockout_bonus_points_config: i64,
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
    exact_score_points_config: i64,
    correct_result_exact_side_points_config: i64,
    correct_result_points_config: i64,
    incorrect_result_points_config: i64,
    knockout_bonus_points_config: i64,
}

#[cfg(feature = "server")]
pub(super) fn breakdown_points(
    is_knockout: bool,
    official: &Outcome,
    guess: &Outcome,
    config: &FootballScoringConfig,
) -> BreakdownPoints {
    let exact_score_points =
        if guess.home_score == official.home_score && guess.away_score == official.away_score {
            config.exact_score_points
        } else {
            0
        };

    let correct_outcome = (guess.home_score > guess.away_score
        && official.home_score > official.away_score)
        || (guess.home_score < guess.away_score && official.home_score < official.away_score)
        || (guess.home_score == guess.away_score && official.home_score == official.away_score);

    let outcome_points = if exact_score_points > 0 {
        0
    } else if correct_outcome {
        config.correct_result_points
    } else {
        0
    };
    let incorrect_result_points = if exact_score_points == 0 && !correct_outcome {
        config.incorrect_result_points
    } else {
        0
    };

    let goal_bonus_points = if exact_score_points > 0 {
        0
    } else if correct_outcome
        && ((guess.home_score == official.home_score && official.home_score > 0)
            || (guess.away_score == official.away_score && official.away_score > 0))
    {
        config.correct_result_exact_side_points - config.correct_result_points
    } else {
        0
    };

    // O classificado deixa de ter bônus próprio (deduzido do placar/pênaltis).
    let qualifier_points = 0;

    let penalties_points = if is_knockout {
        let legacy = knockout_bonus(official, guess);
        if legacy == 0 {
            0
        } else {
            (legacy * config.knockout_bonus_points) / 3
        }
    } else {
        0
    };

    BreakdownPoints {
        exact_score_points,
        outcome_points,
        goal_bonus_points,
        qualifier_points,
        penalties_points,
        incorrect_result_points,
        total_points: exact_score_points
            + outcome_points
            + goal_bonus_points
            + qualifier_points
            + penalties_points
            + incorrect_result_points,
    }
}

#[cfg(feature = "server")]
pub(super) fn build_eligibility(row: &BreakdownRow) -> (bool, String) {
    let joined_at = chrono::NaiveDateTime::parse_from_str(&row.joined_at, "%Y-%m-%d %H:%M:%S").ok();
    let lock_at = chrono::DateTime::parse_from_rfc3339(&row.lock_at).ok();
    match (joined_at, lock_at) {
        (Some(joined), Some(lock_at)) => {
            let joined =
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(joined, chrono::Utc);
            let lock_at = lock_at.with_timezone(&chrono::Utc);
            if lock_at >= joined {
                (true, "eligible".to_string())
            } else {
                (false, "joined_after_kickoff".to_string())
            }
        }
        _ => (false, "invalid_dates".to_string()),
    }
}
