use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::sync::OnceLock;
use std::time::Duration;

use crate::config::settings;

static DB: OnceLock<SqlitePool> = OnceLock::new();
// Recompile this module whenever the migration set changes; the macro embeds
// the complete directory into the binary used by local and production startup.
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
    let database_path = settings().database_path.clone();

    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .busy_timeout(Duration::from_millis(settings().database_busy_timeout_ms));

    // As migrations precisam iniciar com a configuração SQLite padrão: a
    // migration de eventos adiciona uma coluna REFERENCES com default de
    // backfill, combinação que o SQLite não permite com FKs já ativadas.
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
            panic!(
                "schema de produção incompatível: aplicadas={}, esperadas={}, pendentes={}, dirty={}, checksum_mismatch={}",
                report.applied, report.expected, report.pending, report.dirty, report.checksum_mismatch
            );
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

/// Opens the database for an operational snapshot without applying or
/// requiring the current migration set. A pre-deploy backup must work before
/// the new image's migrations are applied.
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

pub async fn quick_check() -> Result<String, sqlx::Error> {
    let row: (String,) = sqlx::query_as("PRAGMA quick_check")
        .fetch_one(pool())
        .await?;
    Ok(row.0)
}

pub async fn migration_status() -> Result<(i64, i64), sqlx::Error> {
    let applied: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool())
        .await?;
    let expected = MIGRATOR.iter().count() as i64;
    Ok((applied.0, expected))
}

async fn operation_pool() -> Result<SqlitePool, sqlx::Error> {
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
    let table_exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("não foi possível consultar migrations: {e}"))?;
    if table_exists.0 == 0 {
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
        sqlx::query_as("SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("não foi possível consultar migrations: {e}"))?;
    let mut checksum_mismatch = false;
    for (version, checksum, success) in &rows {
        if *success == 0 {
            continue;
        }
        let Some(migration) = MIGRATOR
            .iter()
            .find(|migration| migration.version == *version)
        else {
            checksum_mismatch = true;
            continue;
        };
        if migration.checksum.as_ref() != checksum.as_slice() {
            checksum_mismatch = true;
        }
    }
    let applied = rows.iter().filter(|(_, _, success)| *success != 0).count() as i64;
    let dirty = rows.iter().any(|(_, _, success)| *success == 0);
    let expected = MIGRATOR.iter().count() as i64;
    pool.close().await;
    Ok(MigrationReport {
        applied,
        expected,
        pending: applied != expected,
        dirty,
        checksum_mismatch,
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
    let archived_before = reconcile_legacy_predictions(&pool).await?;
    if archived_before > 0 {
        eprintln!(
            "reconciliação legada: {} prediction(s) sem pool preservada(s) em legacy_predictions_without_pool",
            archived_before
        );
    }
    if let Err(first_error) = MIGRATOR.run(&pool).await {
        // Migrations 0019/0020 may have been committed before 0021 failed.
        // Retry once after the same backend-owned reconciliation so upgrades
        // from those intermediate states remain recoverable.
        let archived_after = reconcile_legacy_predictions(&pool).await?;
        if archived_after == 0 {
            return Err(format!("falha ao aplicar migrations: {first_error}"));
        }
        eprintln!(
            "reconciliação após falha de migration: {} prediction(s) preservada(s); repetindo migrations",
            archived_after
        );
        MIGRATOR
            .run(&pool)
            .await
            .map_err(|error| format!("falha ao aplicar migrations após reconciliação: {error}"))?;
    }
    pool.close().await;
    Ok(())
}

pub async fn integrity_check_without_migration() -> Result<String, String> {
    let pool = operation_pool()
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

/// Preserves legacy global predictions for which the historical schema cannot
/// derive a legitimate pool. Such rows must not be silently dropped and must
/// not be assigned to a synthetic pool merely to satisfy the new schema.
async fn reconcile_legacy_predictions(pool: &SqlitePool) -> Result<u64, String> {
    let columns: Vec<(i64, String, String, i64, Option<String>, i64)> =
        sqlx::query_as("PRAGMA table_info(predictions)")
            .fetch_all(pool)
            .await
            .map_err(|error| format!("falha ao inspecionar predictions legadas: {error}"))?;
    if columns.is_empty()
        || columns
            .iter()
            .any(|(_, name, _, _, _, _)| name == "pool_id")
    {
        return Ok(0);
    }

    let required_tables: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN
         ('users','pools','pool_members','prediction_items','matches')",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| format!("falha ao verificar schema legado: {error}"))?;
    if required_tables.0 != 5 {
        return Ok(0);
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("falha ao iniciar reconciliação legada: {error}"))?;
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS legacy_predictions_without_pool (
            source_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            match_id TEXT NOT NULL,
            home_score INTEGER NOT NULL,
            away_score INTEGER NOT NULL,
            submitted_at TEXT NOT NULL,
            qualifier TEXT,
            went_to_penalties INTEGER NOT NULL,
            penalty_home_score INTEGER,
            penalty_away_score INTEGER,
            reason TEXT NOT NULL,
            archived_at TEXT NOT NULL DEFAULT(datetime('now'))
        )",
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("falha ao criar arquivo de predictions legadas: {error}"))?;

    sqlx::query(
        "INSERT OR IGNORE INTO legacy_predictions_without_pool (
            source_id, user_id, match_id, home_score, away_score, submitted_at,
            qualifier, went_to_penalties, penalty_home_score, penalty_away_score, reason
        )
        SELECT old.id, old.user_id, old.match_id, old.home_score, old.away_score,
               old.submitted_at, old.qualifier, old.went_to_penalties,
               old.penalty_home_score, old.penalty_away_score,
               'no_pool_for_prediction_event'
        FROM predictions old
        WHERE NOT EXISTS (
            SELECT 1
            FROM matches m
            JOIN prediction_items pi ON pi.id = m.prediction_item_id
            JOIN pools p ON p.event_id = pi.event_id
            JOIN pool_members pm ON pm.pool_id = p.id AND pm.user_id = old.user_id
            WHERE m.id = old.match_id
        )",
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("falha ao arquivar predictions sem pool: {error}"))?;

    let deleted = sqlx::query(
        "DELETE FROM predictions
         WHERE id IN (SELECT source_id FROM legacy_predictions_without_pool)",
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("falha ao remover somente predictions já arquivadas: {error}"))?;
    tx.commit()
        .await
        .map_err(|error| format!("falha ao confirmar reconciliação legada: {error}"))?;
    Ok(deleted.rows_affected())
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use std::borrow::Cow;

    use sqlx::migrate::Migrator;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::{reconcile_legacy_predictions, MIGRATOR};

    #[tokio::test]
    async fn event_migration_seeds_world_cup_backfills_pools_and_enforces_fk() {
        // Simula uma instalação existente exatamente antes da migration 0018.
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("criar sqlite em memoria");
        sqlx::raw_sql(
            "CREATE TABLE users (id TEXT PRIMARY KEY);
             CREATE TABLE pools (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                invite_code TEXT UNIQUE NOT NULL,
                created_by TEXT NOT NULL REFERENCES users(id),
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                description TEXT NOT NULL DEFAULT '',
                visible_rules TEXT NOT NULL DEFAULT '',
                join_closed_at TEXT
             );
             CREATE TABLE pool_members (
                pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                PRIMARY KEY (pool_id, user_id)
             );
             INSERT INTO users (id) VALUES ('owner');
             INSERT INTO pools (id, name, invite_code, created_by)
             VALUES ('old-pool', 'Bolao existente', 'EXISTE', 'owner');
             INSERT INTO pool_members (pool_id, user_id) VALUES ('old-pool', 'owner');",
        )
        .execute(&db)
        .await
        .expect("preparar banco historico");

        sqlx::raw_sql(include_str!("../migrations/0019_events.sql"))
            .execute(&db)
            .await
            .expect("aplicar migration de eventos");

        let event: (String, String, String) =
            sqlx::query_as("SELECT slug, kind, status FROM events WHERE slug = 'world-cup-2026'")
                .fetch_one(&db)
                .await
                .expect("seed da Copa");
        assert_eq!(
            event,
            ("world-cup-2026".into(), "football".into(), "active".into())
        );

        let event_id: (String,) =
            sqlx::query_as("SELECT event_id FROM pools WHERE id = 'old-pool'")
                .fetch_one(&db)
                .await
                .expect("pool existente com backfill");
        let seeded_id: (String,) =
            sqlx::query_as("SELECT id FROM events WHERE slug = 'world-cup-2026'")
                .fetch_one(&db)
                .await
                .expect("id do evento seed");
        assert_eq!(event_id, seeded_id);

        let child_fk: (String,) = sqlx::query_as(
            "SELECT \"table\" FROM pragma_foreign_key_list('pool_members') WHERE \"from\" = 'pool_id'",
        )
        .fetch_one(&db)
        .await
        .expect("FK de pool_members preservada");
        assert_eq!(child_fk.0, "pools");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&db)
            .await
            .expect("ativar FKs no sqlite de teste");
        let missing_event = sqlx::query(
            "INSERT INTO pools (id, name, invite_code, created_by, event_id)
             VALUES ('missing-event', 'Sem evento', 'SEMID', 'owner', NULL)",
        )
        .execute(&db)
        .await;
        assert!(
            missing_event.is_err(),
            "trigger deve impedir pool sem event_id"
        );
        let orphan = sqlx::query(
            "INSERT INTO pools (id, name, invite_code, created_by, event_id)
             VALUES ('orphan', 'Invalido', 'ORFAO', 'owner', 'evento-inexistente')",
        )
        .execute(&db)
        .await;
        assert!(orphan.is_err(), "FK deve impedir pool sem evento existente");
    }

    #[tokio::test]
    async fn generic_prediction_migration_backfills_per_pool_and_rejects_inconsistent_identity() {
        // Esquema mínimo exatamente compatível com 0019, antes da reconstrução
        // de predictions na 0020.
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("criar sqlite em memoria");
        sqlx::raw_sql(
            "CREATE TABLE users (id TEXT PRIMARY KEY);
             CREATE TABLE events (id TEXT PRIMARY KEY);
             CREATE TABLE pools (id TEXT PRIMARY KEY, event_id TEXT NOT NULL REFERENCES events(id));
             CREATE TABLE pool_members (pool_id TEXT NOT NULL, user_id TEXT NOT NULL, PRIMARY KEY(pool_id, user_id));
             CREATE TABLE prediction_items (id TEXT PRIMARY KEY, event_id TEXT NOT NULL REFERENCES events(id));
             CREATE TABLE matches (id TEXT PRIMARY KEY, prediction_item_id TEXT NOT NULL REFERENCES prediction_items(id));
             CREATE TABLE predictions (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id), match_id TEXT NOT NULL REFERENCES matches(id),
                home_score INTEGER NOT NULL, away_score INTEGER NOT NULL, submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
                qualifier TEXT, went_to_penalties INTEGER NOT NULL DEFAULT 0, penalty_home_score INTEGER, penalty_away_score INTEGER,
                UNIQUE(user_id, match_id)
             );
             INSERT INTO users VALUES ('user'), ('user-two');
             INSERT INTO events VALUES ('event-a'), ('event-b');
             INSERT INTO pools VALUES ('pool-a1', 'event-a'), ('pool-a2', 'event-a'), ('pool-b', 'event-b');
             INSERT INTO pool_members VALUES ('pool-a1', 'user'), ('pool-a2', 'user'), ('pool-b', 'user'),
                 ('pool-a1', 'user-two'), ('pool-a2', 'user-two');
             INSERT INTO prediction_items VALUES ('item-a', 'event-a'), ('item-b', 'event-b');
             INSERT INTO matches VALUES ('match-a', 'item-a'), ('match-b', 'item-b');
             INSERT INTO predictions (id, user_id, match_id, home_score, away_score)
             VALUES ('old-prediction', 'user', 'match-a', 2, 1);",
        )
        .execute(&db)
        .await
        .expect("preparar banco anterior a 0020");

        sqlx::raw_sql(include_str!("../migrations/0021_generic_predictions.sql"))
            .execute(&db)
            .await
            .expect("migrar predictions historicas");

        let copied: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM predictions WHERE user_id = 'user' AND item_id = 'item-a'",
        )
        .fetch_one(&db)
        .await
        .expect("contar previsoes migradas");
        assert_eq!(
            copied.0, 2,
            "uma previsão antiga vira uma por pool do evento"
        );

        let duplicate = sqlx::query(
            "INSERT INTO predictions (id, pool_id, user_id, item_id, match_id, home_score, away_score)
             VALUES ('duplicate', 'pool-a1', 'user', 'item-a', 'match-a', 1, 0)",
        )
        .execute(&db)
        .await;
        assert!(
            duplicate.is_err(),
            "mesmo usuário/item/pool deve ter uma só previsão"
        );

        let other_pool = sqlx::query(
            "INSERT INTO predictions (id, pool_id, user_id, item_id, match_id, home_score, away_score)
             VALUES ('other-pool-one', 'pool-a1', 'user-two', 'item-a', 'match-a', 1, 0),
                    ('other-pool-two', 'pool-a2', 'user-two', 'item-a', 'match-a', 1, 0)",
        )
        .execute(&db)
        .await;
        assert!(
            other_pool.is_ok(),
            "um usuário pode prever o mesmo item em dois pools"
        );

        let mismatch = sqlx::query(
            "INSERT INTO predictions (id, pool_id, user_id, item_id, match_id, home_score, away_score)
             VALUES ('bad-match', 'pool-a1', 'user', 'item-b', 'match-a', 1, 0)",
        )
        .execute(&db)
        .await;
        assert!(mismatch.is_err(), "item deve corresponder ao match");

        let cross_event = sqlx::query(
            "INSERT INTO predictions (id, pool_id, user_id, item_id, match_id, home_score, away_score)
             VALUES ('bad-event', 'pool-b', 'user', 'item-a', 'match-a', 1, 0)",
        )
        .execute(&db)
        .await;
        assert!(
            cross_event.is_err(),
            "pool e item devem pertencer ao mesmo evento"
        );
    }

    #[tokio::test]
    async fn legacy_prediction_without_pool_is_archived_before_generic_migration() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("criar sqlite em memoria");
        sqlx::raw_sql(
            "CREATE TABLE users (id TEXT PRIMARY KEY);
             CREATE TABLE events (id TEXT PRIMARY KEY);
             CREATE TABLE pools (id TEXT PRIMARY KEY, event_id TEXT NOT NULL);
             CREATE TABLE pool_members (pool_id TEXT NOT NULL, user_id TEXT NOT NULL, PRIMARY KEY(pool_id, user_id));
             CREATE TABLE prediction_items (id TEXT PRIMARY KEY, event_id TEXT NOT NULL);
             CREATE TABLE matches (id TEXT PRIMARY KEY, prediction_item_id TEXT NOT NULL);
             CREATE TABLE predictions (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, match_id TEXT NOT NULL,
                home_score INTEGER NOT NULL, away_score INTEGER NOT NULL,
                submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
                qualifier TEXT, went_to_penalties INTEGER NOT NULL DEFAULT 0,
                penalty_home_score INTEGER, penalty_away_score INTEGER
             );
             INSERT INTO users VALUES ('user');
             INSERT INTO events VALUES ('event');
             INSERT INTO pools VALUES ('pool', 'event');
             INSERT INTO prediction_items VALUES ('item', 'event');
             INSERT INTO matches VALUES ('match', 'item');
             INSERT INTO predictions (id, user_id, match_id, home_score, away_score)
             VALUES ('orphan', 'user', 'match', 2, 1);",
        )
        .execute(&db)
        .await
        .expect("preparar prediction sem pool");

        let archived = reconcile_legacy_predictions(&db)
            .await
            .expect("reconciliar prediction sem pool");
        assert_eq!(archived, 1);

        sqlx::raw_sql(include_str!("../migrations/0021_generic_predictions.sql"))
            .execute(&db)
            .await
            .expect("migrar após arquivar prediction sem pool");

        let preserved: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM legacy_predictions_without_pool WHERE source_id='orphan'",
        )
        .fetch_one(&db)
        .await
        .expect("consultar prediction arquivada");
        assert_eq!(preserved.0, 1);
    }

    #[tokio::test]
    async fn custom_question_migration_preserves_football_prediction_and_allows_null_match() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE users (id TEXT PRIMARY KEY); CREATE TABLE events (id TEXT PRIMARY KEY);
             CREATE TABLE pools (id TEXT PRIMARY KEY,event_id TEXT NOT NULL); CREATE TABLE prediction_items (id TEXT PRIMARY KEY,event_id TEXT NOT NULL,kind TEXT NOT NULL);
             CREATE TABLE matches (id TEXT PRIMARY KEY,prediction_item_id TEXT NOT NULL);
             CREATE TABLE predictions (id TEXT PRIMARY KEY,pool_id TEXT NOT NULL,user_id TEXT NOT NULL,item_id TEXT NOT NULL,match_id TEXT NOT NULL,home_score INTEGER NOT NULL,away_score INTEGER NOT NULL,submitted_at TEXT NOT NULL DEFAULT (datetime('now')),qualifier TEXT,went_to_penalties INTEGER NOT NULL DEFAULT 0,penalty_home_score INTEGER,penalty_away_score INTEGER,UNIQUE(pool_id,user_id,item_id));
             INSERT INTO users VALUES ('u'); INSERT INTO events VALUES ('e'); INSERT INTO pools VALUES ('p','e');
             INSERT INTO prediction_items VALUES ('football','e','football_match'),('custom','e','single_choice'); INSERT INTO matches VALUES ('m','football');
             INSERT INTO predictions (id,pool_id,user_id,item_id,match_id,home_score,away_score) VALUES ('historic','p','u','football','m',2,1);",
        ).execute(&db).await.unwrap();
        sqlx::raw_sql(include_str!("../migrations/0022_custom_questions.sql"))
            .execute(&db)
            .await
            .unwrap();
        let historic: (String, String, i64, i64) = sqlx::query_as(
            "SELECT id,match_id,home_score,away_score FROM predictions WHERE id='historic'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(historic, ("historic".into(), "m".into(), 2, 1));
        sqlx::query("INSERT INTO custom_questions (item_id) VALUES ('custom')")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO custom_question_options (id,item_id,label,sort_order) VALUES ('o','custom','A',0)").execute(&db).await.unwrap();
        sqlx::query("INSERT INTO predictions (id,pool_id,user_id,item_id) VALUES ('custom-prediction','p','u','custom')").execute(&db).await.unwrap();
        sqlx::query("INSERT INTO custom_prediction_values (prediction_id,option_id) VALUES ('custom-prediction','o')").execute(&db).await.unwrap();
    }

    #[tokio::test]
    async fn pool_scoring_migration_backfills_defaults_and_custom_question_points() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE users(id TEXT PRIMARY KEY); CREATE TABLE events(id TEXT PRIMARY KEY);
             CREATE TABLE pools(id TEXT PRIMARY KEY,event_id TEXT NOT NULL,created_by TEXT NOT NULL);
             CREATE TABLE prediction_items(id TEXT PRIMARY KEY,event_id TEXT NOT NULL,kind TEXT NOT NULL,lock_at TEXT NOT NULL);
             CREATE TABLE matches(id TEXT PRIMARY KEY,prediction_item_id TEXT NOT NULL);
             CREATE TABLE custom_questions(item_id TEXT PRIMARY KEY,points INTEGER NOT NULL,correct_option_id TEXT);
             INSERT INTO users VALUES('u'); INSERT INTO events VALUES('e'); INSERT INTO pools VALUES('p','e','u');
             INSERT INTO prediction_items VALUES('football','e','football_match','2999-01-01T00:00:00Z'),('choice','e','single_choice','2999-01-01T00:00:00Z');
             INSERT INTO custom_questions VALUES('choice',6,NULL);",
        ).execute(&db).await.unwrap();
        sqlx::raw_sql(include_str!("../migrations/0023_pool_scoring.sql"))
            .execute(&db)
            .await
            .unwrap();
        let football:(i64,i64,i64,i64,i64)=sqlx::query_as("SELECT exact_score_points,correct_result_exact_side_points,correct_result_points,incorrect_result_points,knockout_bonus_points FROM football_pool_scoring WHERE pool_id='p'").fetch_one(&db).await.unwrap();
        assert_eq!(football, (7, 4, 3, 0, 3));
        let custom:(i64,i64)=sqlx::query_as("SELECT correct_points,incorrect_points FROM custom_pool_item_scoring WHERE pool_id='p' AND item_id='choice'").fetch_one(&db).await.unwrap();
        assert_eq!(
            custom,
            (6, 0),
            "points legado é copiado somente na inicialização"
        );
    }

    #[tokio::test]
    async fn supported_schema_upgrade_preserves_domain_data_and_adds_assets() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("criar sqlite em memoria");
        let older = Migrator {
            migrations: Cow::Owned(MIGRATOR.iter().take(31).cloned().collect()),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };
        older.run(&db).await.expect("aplicar schema suportado");
        sqlx::query(
            "INSERT INTO users(id,username,email,password_hash) VALUES('upgrade-user','upgrade','upgrade@example.com','hash');
             INSERT INTO pools(id,name,invite_code,created_by,event_id) VALUES('upgrade-pool','Upgrade','UPGRADE','upgrade-user','8e4cfe71-9123-4bd1-a4a9-989eeb55b77f');
             INSERT INTO pool_members(pool_id,user_id) VALUES('upgrade-pool','upgrade-user');",
        )
        .execute(&db)
        .await
        .expect("seed de dados legados");
        let item: (String, String) = sqlx::query_as(
            "SELECT pi.id,m.id FROM prediction_items pi JOIN matches m ON m.prediction_item_id=pi.id LIMIT 1",
        )
        .fetch_one(&db)
        .await
        .expect("item legado");
        sqlx::query(
            "INSERT INTO predictions(id,pool_id,user_id,item_id,match_id,home_score,away_score)
             VALUES('upgrade-prediction','upgrade-pool','upgrade-user',?1,?2,2,1)",
        )
        .bind(&item.0)
        .bind(&item.1)
        .execute(&db)
        .await
        .expect("prediction legada");

        let before: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM events), (SELECT COUNT(*) FROM pools), (SELECT COUNT(*) FROM predictions)",
        )
        .fetch_one(&db)
        .await
        .expect("contagens legadas");
        let current = Migrator {
            migrations: Cow::Owned(MIGRATOR.iter().skip(31).cloned().collect()),
            ignore_missing: true,
            locking: true,
            no_tx: false,
        };
        current.run(&db).await.expect("aplicar migration atual");
        let after: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM events), (SELECT COUNT(*) FROM pools), (SELECT COUNT(*) FROM predictions)",
        )
        .fetch_one(&db)
        .await
        .expect("contagens atuais");
        assert_eq!(before, after, "upgrade não pode perder dados de domínio");
        sqlx::query(
            "INSERT INTO assets(id,storage_key,sha256,media_type,width,height,byte_size,uploaded_by)
             VALUES('upgrade-asset','hash/master.webp',?1,'image/webp',1,1,1,'upgrade-user')",
        )
        .bind("a".repeat(64))
        .execute(&db)
        .await
        .expect("asset pós-upgrade");
        let integrity: (String,) = sqlx::query_as("PRAGMA integrity_check")
            .fetch_one(&db)
            .await
            .expect("integrity pós-upgrade");
        assert_eq!(integrity.0, "ok");
    }
}
