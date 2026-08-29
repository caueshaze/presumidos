use super::super::legacy_reconciliation::reconcile_legacy_predictions;
use sqlx::sqlite::SqlitePoolOptions;

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

    sqlx::raw_sql(include_str!(
        "../../../migrations/0021_generic_predictions.sql"
    ))
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

    sqlx::raw_sql(include_str!(
        "../../../migrations/0021_generic_predictions.sql"
    ))
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
