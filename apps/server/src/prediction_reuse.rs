//! Reuso pontual de Predictions entre Pools da mesma EventVersion.

#[path = "prediction_reuse/actions.rs"]
mod actions;
#[path = "prediction_reuse/helpers.rs"]
mod helpers;
#[path = "prediction_reuse/suggestion.rs"]
mod suggestion;

pub use actions::{copy, start_empty};
pub use suggestion::suggestion;
