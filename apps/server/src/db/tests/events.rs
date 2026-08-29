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

    sqlx::raw_sql(include_str!("../../../migrations/0019_events.sql"))
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

    let event_id: (String,) = sqlx::query_as("SELECT event_id FROM pools WHERE id = 'old-pool'")
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
