//! Fachada do acesso SQLite.
//!
//! A implementação está isolada em submódulo para manter este ponto de
//! compatibilidade pequeno para os serviços, CLI e testes.

#[path = "db/bootstrap.rs"]
mod bootstrap;
#[path = "db/health.rs"]
mod health;
#[path = "db/legacy_reconciliation.rs"]
mod legacy_reconciliation;
#[path = "db/migrations.rs"]
mod migrations;
#[cfg(all(test, feature = "server"))]
#[path = "db/tests.rs"]
mod tests;

pub use bootstrap::{init, init_for_backup, pool, MigrationReport, MIGRATOR};
pub use health::{integrity_check_without_migration, migration_status, quick_check};
pub use migrations::{apply_migrations, migration_report};
