//! Fachada do domínio de bolões.
//!
//! A implementação foi deslocada para submódulos a fim de manter este ponto de
//! entrada estável para handlers e demais serviços.
#[path = "pools/core.rs"]
mod core;

pub use core::*;
pub(crate) use core::{ensure_pool_membership, sqlite_now};
