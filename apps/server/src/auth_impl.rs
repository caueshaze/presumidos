use super::*;

#[path = "auth_support.rs"]
mod support;
pub(crate) use support::*;

#[path = "auth_ops.rs"]
mod ops;
pub(crate) use ops::*;

#[path = "auth_registration.rs"]
mod registration;
pub(crate) use registration::*;

#[path = "auth_login.rs"]
mod login;
pub(crate) use login::*;

#[cfg(all(test, feature = "server"))]
#[path = "auth_tests.rs"]
mod tests;
