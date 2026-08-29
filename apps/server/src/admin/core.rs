#[cfg(feature = "server")]
#[path = "settings.rs"]
mod settings;
#[cfg(feature = "server")]
pub use settings::{load_admin_settings, prediction_lock_minutes, save_admin_settings};
#[cfg(feature = "server")]
#[path = "events.rs"]
mod events;
#[cfg(feature = "server")]
pub use events::{finish_event, list_events_admin, set_pool_creation_enabled};

#[cfg(feature = "server")]
#[path = "core/audit.rs"]
mod audit;
#[cfg(feature = "server")]
#[path = "core/matches.rs"]
mod matches;
#[cfg(feature = "server")]
#[path = "core/overrides.rs"]
mod overrides;
#[cfg(feature = "server")]
#[path = "core/predictions.rs"]
mod predictions;
#[cfg(feature = "server")]
#[path = "core/scoring.rs"]
mod scoring;
#[cfg(feature = "server")]
#[path = "core/shared.rs"]
mod shared;
#[cfg(feature = "server")]
#[path = "core/users.rs"]
mod users;

#[cfg(feature = "server")]
pub use audit::*;
#[cfg(feature = "server")]
pub use matches::*;
#[cfg(feature = "server")]
pub use overrides::*;
#[cfg(feature = "server")]
pub use predictions::*;
#[cfg(feature = "server")]
pub use scoring::*;
#[cfg(feature = "server")]
pub(crate) use shared::*;
#[cfg(feature = "server")]
pub use users::*;
