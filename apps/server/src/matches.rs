//! Fachada do domínio de partidas, organizada por responsabilidade.

#[path = "matches/admin.rs"]
mod admin;
#[path = "matches/lifecycle.rs"]
mod lifecycle;
#[path = "matches/predictions.rs"]
mod predictions;
#[path = "matches/repository.rs"]
mod repository;
#[path = "matches/results.rs"]
mod results;

#[cfg(feature = "server")]
pub use admin::{
    create_match, delete_match, set_knockout_released, update_match_schedule, update_match_teams,
};
#[cfg(feature = "server")]
pub use lifecycle::is_knockout_released;
#[cfg(feature = "server")]
pub(crate) use lifecycle::{force_finish_matches_for_ended_events, force_finish_matches_for_event};
#[cfg(feature = "server")]
pub use predictions::{get_my_predictions, list_matches, submit_prediction};
#[cfg(feature = "server")]
pub use results::{set_match_finished, set_match_result};

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::predictions::sanitize_knockout_input;
    use crate::models::KnockoutEntry;

    fn entry(home: Option<i64>, away: Option<i64>) -> KnockoutEntry {
        KnockoutEntry {
            qualifier: None,
            went_to_penalties: false,
            penalty_home: home,
            penalty_away: away,
        }
    }

    #[test]
    fn group_match_zeroes_knockout_fields() {
        assert_eq!(
            sanitize_knockout_input(false, 1, 1, entry(Some(5), Some(4))).unwrap(),
            KnockoutEntry::default()
        );
    }

    #[test]
    fn knockout_derives_qualifier_and_validates_penalties() {
        let winner = sanitize_knockout_input(true, 2, 1, entry(None, None)).unwrap();
        assert_eq!(winner.qualifier.as_deref(), Some("home"));
        assert!(!winner.went_to_penalties);
        let penalties = sanitize_knockout_input(true, 1, 1, entry(Some(4), Some(2))).unwrap();
        assert_eq!(penalties.qualifier.as_deref(), Some("home"));
        assert!(penalties.went_to_penalties);
        assert_eq!(
            sanitize_knockout_input(true, 1, 1, entry(Some(3), Some(3)))
                .unwrap_err()
                .message(),
            "O placar dos pênaltis não pode terminar empatado."
        );
        assert_eq!(
            sanitize_knockout_input(true, 1, 1, entry(None, None))
                .unwrap_err()
                .message(),
            "Empate no tempo normal: informe o placar dos pênaltis dos dois lados."
        );
    }
}
