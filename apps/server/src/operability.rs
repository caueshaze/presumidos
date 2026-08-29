#![cfg(feature = "server")]

//! Operabilidade: runtime, probes, métricas, backup e restauração.

include!("operability/state.rs");
include!("operability/readiness.rs");
include!("operability/archive.rs");
include!("operability/backups.rs");
include!("operability/restore.rs");
