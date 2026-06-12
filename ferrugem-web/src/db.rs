use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::sync::OnceLock;

use crate::config::settings;

static DB: OnceLock<SqlitePool> = OnceLock::new();

pub async fn init() {
    let database_path = settings().database_path.clone();

    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("falha ao conectar ao banco de dados");

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await
        .expect("falha ao ativar WAL");
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(&pool)
        .await
        .expect("falha ao configurar busy_timeout");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("falha ao executar migrations");

    DB.set(pool).expect("banco já inicializado");
}

pub fn pool() -> &'static SqlitePool {
    DB.get().expect("banco não inicializado")
}
