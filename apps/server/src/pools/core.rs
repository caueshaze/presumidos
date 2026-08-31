//! Fachada do domínio de bolões. As responsabilidades vivem em submódulos.

#[cfg(feature = "server")]
#[path = "reactions.rs"]
mod reactions;
#[cfg(feature = "server")]
pub use reactions::{mark_prediction_reactions_seen, react_to_prediction};

#[cfg(feature = "server")]
#[path = "core/adjustments.rs"]
mod adjustments;
#[cfg(feature = "server")]
#[path = "core/admin.rs"]
mod admin;
#[cfg(feature = "server")]
#[path = "core/closure.rs"]
mod closure;
#[cfg(feature = "server")]
#[path = "core/deletion.rs"]
mod deletion;
#[cfg(feature = "server")]
#[path = "core/editorial.rs"]
mod editorial;
#[cfg(feature = "server")]
#[path = "core/invites.rs"]
mod invites;
#[cfg(feature = "server")]
#[path = "core/lifecycle.rs"]
mod lifecycle;
#[cfg(feature = "server")]
#[path = "core/listing.rs"]
mod listing;
#[cfg(feature = "server")]
#[path = "core/predictions.rs"]
mod predictions;
#[cfg(feature = "server")]
#[path = "core/reports.rs"]
mod reports;
#[cfg(feature = "server")]
#[path = "core/shared.rs"]
mod shared;

#[cfg(feature = "server")]
pub use adjustments::*;
#[cfg(feature = "server")]
pub use admin::*;
#[cfg(feature = "server")]
pub use closure::*;
#[cfg(feature = "server")]
pub use deletion::*;
#[cfg(feature = "server")]
pub use editorial::*;
#[cfg(feature = "server")]
pub use invites::*;
#[cfg(feature = "server")]
pub use lifecycle::*;
#[cfg(feature = "server")]
pub use listing::*;
#[cfg(feature = "server")]
pub use predictions::*;
#[cfg(feature = "server")]
pub use reports::*;
#[cfg(feature = "server")]
pub(crate) use shared::*;

pub(crate) use crate::pool_scoring::{
    custom_item_scoring_config, football_scoring_config, multiple_choice_item_scoring_config,
    numeric_item_scoring_config, update_custom_item_scoring_config, update_football_scoring_config,
    update_multiple_choice_item_scoring_config, update_numeric_item_scoring_config,
};
