#[cfg(feature = "server")]
mod breakdowns;
mod core;
#[cfg(feature = "server")]
mod jobs;
#[cfg(feature = "server")]
mod leaderboard;

#[cfg(feature = "server")]
pub(crate) use breakdowns::{
    ensure_breakdowns_seeded, recompute_breakdowns, recompute_custom_breakdowns,
};
#[cfg(feature = "server")]
pub use breakdowns::{
    list_my_match_points, list_pool_breakdowns, list_user_breakdowns, recalculate_match_breakdowns,
};
#[cfg(test)]
pub(crate) use core::{base_points, match_points};
pub(crate) use core::{knockout_bonus, Outcome};
#[cfg(feature = "server")]
pub use jobs::{
    recalculate_all_breakdowns, recalculate_custom_breakdowns, recalculate_pool_user_breakdowns,
};
#[cfg(feature = "server")]
pub use leaderboard::get_leaderboard;
#[cfg(test)]
pub use leaderboard::rank_leaderboard;
