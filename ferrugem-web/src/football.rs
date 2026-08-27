//! Boundary público do suporte de football.
//!
//! A semântica pura vive em `domain`; provider ESPN, aplicação, poller e
//! sync-fixtures vivem na integração e podem ser avaliados separadamente.

#![cfg(feature = "server")]

mod domain;
mod integration;
pub(crate) use integration::*;
