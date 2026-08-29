use super::super::MIGRATOR;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqlitePoolOptions;
use std::borrow::Cow;

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
    sqlx::raw_sql(include_str!(
        "../../../migrations/0022_custom_questions.sql"
    ))
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
    sqlx::raw_sql(include_str!("../../../migrations/0023_pool_scoring.sql"))
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
