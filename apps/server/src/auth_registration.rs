#[cfg(feature = "server")]
#[path = "auth_registration/helpers.rs"]
mod helpers;
#[cfg(feature = "server")]
#[path = "auth_registration/maintenance.rs"]
mod maintenance;
#[cfg(feature = "server")]
#[path = "auth_registration/password_reset.rs"]
mod password_reset;
#[cfg(feature = "server")]
#[path = "auth_registration/registration.rs"]
mod registration;

#[cfg(feature = "server")]
pub use maintenance::{cleanup_expired_auth_data, run_bootstrap_admin};
#[cfg(feature = "server")]
pub use password_reset::{confirm_password_reset, request_password_reset};
#[cfg(feature = "server")]
pub use registration::{confirm_registration, request_registration};
