//! Construção do pool SQLite usado pela aplicação.

use std::{sync::OnceLock, time::Duration};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use crate::config::settings;

use super::migrations::migration_report;

static DB: OnceLock<SqlitePool> = OnceLock::new();
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationReport {
    pub applied: i64,
    pub expected: i64,
    pub pending: bool,
    pub dirty: bool,
    pub checksum_mismatch: bool,
}

pub async fn init() {
    let options = SqliteConnectOptions::new()
        .filename(settings().database_path.clone())
        .create_if_missing(true)
        .busy_timeout(Duration::from_millis(settings().database_busy_timeout_ms));
    let migration_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone().foreign_keys(false))
        .await
        .expect("falha ao conectar ao banco para migrations");
    if settings().app_env == "production" {
        let report = migration_report()
            .await
            .unwrap_or_else(|error| panic!("falha ao verificar schema de produção: {error}"));
        if report.pending || report.dirty || report.checksum_mismatch {
            panic!("schema de produção incompatível: aplicadas={}, esperadas={}, pendentes={}, dirty={}, checksum_mismatch={}", report.applied, report.expected, report.pending, report.dirty, report.checksum_mismatch);
        }
    } else {
        MIGRATOR
            .run(&migration_pool)
            .await
            .expect("falha ao executar migrations");
    }
    migration_pool.close().await;
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options.foreign_keys(true))
        .await
        .expect("falha ao conectar ao banco de dados");
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await
        .expect("falha ao ativar WAL");
    sqlx::query(&format!(
        "PRAGMA busy_timeout = {}",
        settings().database_busy_timeout_ms
    ))
    .execute(&pool)
    .await
    .expect("falha ao configurar busy_timeout");
    DB.set(pool).expect("banco já inicializado");
}

pub async fn init_for_backup() {
    let options = SqliteConnectOptions::new()
        .filename(&settings().database_path)
        .create_if_missing(false)
        .busy_timeout(Duration::from_millis(settings().database_busy_timeout_ms))
        .foreign_keys(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("falha ao conectar ao banco para backup");
    DB.set(pool).expect("banco já inicializado");
}

pub fn pool() -> &'static SqlitePool {
    DB.get().expect("banco não inicializado")
}
