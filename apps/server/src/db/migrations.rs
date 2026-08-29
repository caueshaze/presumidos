//! Inspeção e aplicação explícita de migrations SQLx.

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::time::Duration;

use super::{legacy_reconciliation::reconcile_legacy_predictions, MigrationReport, MIGRATOR};
use crate::config::settings;

pub async fn operation_pool() -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(&settings().database_path)
        .create_if_missing(false)
        .busy_timeout(Duration::from_millis(settings().database_busy_timeout_ms))
        .foreign_keys(false);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
}

pub async fn migration_report() -> Result<MigrationReport, String> {
    let pool = operation_pool()
        .await
        .map_err(|e| format!("não foi possível abrir o banco: {e}"))?;
    let exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("não foi possível consultar migrations: {e}"))?;
    if exists.0 == 0 {
        pool.close().await;
        return Ok(MigrationReport {
            applied: 0,
            expected: MIGRATOR.iter().count() as i64,
            pending: true,
            dirty: false,
            checksum_mismatch: false,
        });
    }
    let rows: Vec<(i64, Vec<u8>, i64)> =
        sqlx::query_as("SELECT version,checksum,success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("não foi possível consultar migrations: {e}"))?;
    let mismatch =
        rows.iter()
            .filter(|(_, _, success)| *success != 0)
            .any(|(version, checksum, _)| {
                MIGRATOR
                    .iter()
                    .find(|migration| migration.version == *version)
                    .map(|migration| migration.checksum.as_ref() != checksum.as_slice())
                    .unwrap_or(true)
            });
    let applied = rows.iter().filter(|(_, _, success)| *success != 0).count() as i64;
    let expected = MIGRATOR.iter().count() as i64;
    let dirty = rows.iter().any(|(_, _, success)| *success == 0);
    pool.close().await;
    Ok(MigrationReport {
        applied,
        expected,
        pending: applied != expected,
        dirty,
        checksum_mismatch: mismatch,
    })
}

pub async fn apply_migrations() -> Result<(), String> {
    let options = SqliteConnectOptions::new()
        .filename(&settings().database_path)
        .create_if_missing(true)
        .busy_timeout(Duration::from_millis(settings().database_busy_timeout_ms))
        .foreign_keys(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| format!("não foi possível abrir o banco: {e}"))?;
    let archived = reconcile_legacy_predictions(&pool).await?;
    if archived > 0 {
        eprintln!("reconciliação legada: {archived} prediction(s) sem pool preservada(s) em legacy_predictions_without_pool");
    }
    if let Err(first_error) = MIGRATOR.run(&pool).await {
        let retried = reconcile_legacy_predictions(&pool).await?;
        if retried == 0 {
            return Err(format!("falha ao aplicar migrations: {first_error}"));
        }
        MIGRATOR
            .run(&pool)
            .await
            .map_err(|error| format!("falha ao aplicar migrations após reconciliação: {error}"))?;
    }
    pool.close().await;
    Ok(())
}
