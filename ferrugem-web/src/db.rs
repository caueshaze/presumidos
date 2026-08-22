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

    // As migrations precisam iniciar com a configuração SQLite padrão: a
    // migration de eventos adiciona uma coluna REFERENCES com default de
    // backfill, combinação que o SQLite não permite com FKs já ativadas.
    let migration_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .expect("falha ao conectar ao banco para migrations");

    sqlx::migrate!("./migrations")
        .run(&migration_pool)
        .await
        .expect("falha ao executar migrations");
    migration_pool.close().await;

    let pool = SqlitePoolOptions::new()
        .connect_with(options.foreign_keys(true))
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

    DB.set(pool).expect("banco já inicializado");
}

pub fn pool() -> &'static SqlitePool {
    DB.get().expect("banco não inicializado")
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

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

        sqlx::raw_sql(include_str!("../migrations/0018_events.sql"))
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

        sqlx::raw_sql(include_str!("../migrations/0020_generic_predictions.sql"))
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
        sqlx::raw_sql(include_str!("../migrations/0021_custom_questions.sql"))
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
        sqlx::raw_sql(include_str!("../migrations/0022_pool_scoring.sql"))
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
}
