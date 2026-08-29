//! Corpos e queries da API; os nomes serializados são parte do contrato HTTP.

#[path = "dto/admin.rs"]
mod admin;
#[path = "dto/auth.rs"]
mod auth;
#[path = "dto/events.rs"]
mod events;
#[path = "dto/predictions.rs"]
mod predictions;

pub(crate) use admin::*;
pub(crate) use auth::*;
pub(crate) use events::*;
pub(crate) use predictions::*;
