//! Consultas de saúde do SQLite sem alterar schema.

use super::MIGRATOR;

pub async fn quick_check() -> Result<String, sqlx::Error> {
    let row: (String,) = sqlx::query_as("PRAGMA quick_check")
        .fetch_one(super::pool())
        .await?;
    Ok(row.0)
}

pub async fn migration_status() -> Result<(i64, i64), sqlx::Error> {
    let applied: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(super::pool())
        .await?;
    Ok((applied.0, MIGRATOR.iter().count() as i64))
}

pub async fn integrity_check_without_migration() -> Result<String, String> {
    let pool = super::migrations::operation_pool()
        .await
        .map_err(|e| format!("não foi possível abrir o banco: {e}"))?;
    let result: Result<(String,), sqlx::Error> = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await;
    pool.close().await;
    result
        .map(|row| row.0)
        .map_err(|e| format!("integrity_check indisponível: {e}"))
}
