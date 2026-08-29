#[cfg(feature = "server")]
mod env;
#[cfg(feature = "server")]
mod health;
#[cfg(feature = "server")]
mod loader;
#[cfg(all(test, feature = "server"))]
mod tests;
#[cfg(feature = "server")]
mod types;
#[cfg(feature = "server")]
mod validation;

#[cfg(feature = "server")]
pub use health::check_config;
#[cfg(feature = "server")]
pub use loader::settings;
#[cfg(feature = "server")]
pub use types::{AppConfig, FootballConfig, RateLimitBackendKind, WebPushConfig};
