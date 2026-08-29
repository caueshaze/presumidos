fn ensure_empty_or_replace(path: &Path, replace: bool) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        if replace {
            return Ok(());
        }
        return Err(format!(
            "destino já existe: {} (use --replace)",
            path.display()
        ));
    }
    let mut entries = path.read_dir().map_err(|e| e.to_string())?;
    if entries.next().is_some() && !replace {
        return Err(format!(
            "destino não está vazio: {} (use --replace)",
            path.display()
        ));
    }
    Ok(())
}

pub fn restore_backup(
    backup: &Path,
    database: &Path,
    assets: &Path,
    replace: bool,
) -> Result<(), String> {
    ensure_empty_or_replace(database, replace)?;
    ensure_empty_or_replace(assets, replace)?;
    let db_parent = database.parent().unwrap_or_else(|| Path::new("."));
    let assets_parent = assets.parent().unwrap_or_else(|| Path::new("."));
    let staging_db = db_parent.join(format!(".restore-db-{}", Uuid::new_v4()));
    let staging_assets = assets_parent.join(format!(".restore-assets-{}", Uuid::new_v4()));
    fs::create_dir_all(&staging_assets)
        .map_err(|e| format!("falha ao criar staging de restore: {e}"))?;
    fs::create_dir_all(&staging_db)
        .map_err(|e| format!("falha ao criar staging de restore: {e}"))?;
    let old_db = db_parent.join(format!(".restore-old-db-{}", Uuid::new_v4()));
    let old_assets = assets_parent.join(format!(".restore-old-assets-{}", Uuid::new_v4()));
    let result = (|| {
        fs::copy(backup.join("database.db"), staging_db.join("database.db"))
            .map_err(|e| format!("falha ao preparar database: {e}"))?;
        let mut archive =
            ZipArchive::new(File::open(backup.join("assets.zip")).map_err(|e| e.to_string())?)
                .map_err(|e| format!("archive de assets inválido: {e}"))?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            if entry.name().starts_with('/') || entry.name().contains("..") {
                return Err("archive de assets contém caminho inválido".to_string());
            }
            let output = staging_assets.join(entry.name());
            if entry.is_dir() {
                fs::create_dir_all(&output).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut target = File::create(&output).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut target).map_err(|e| e.to_string())?;
            }
        }
        let pool = futures_lite_check(&staging_db.join("database.db"))?;
        drop(pool);
        let had_database = database.exists();
        let had_assets = assets.exists();
        if had_database {
            fs::rename(database, &old_db).map_err(|e| e.to_string())?;
        }
        if had_assets {
            if let Err(error) = fs::rename(assets, &old_assets) {
                if had_database {
                    let _ = fs::rename(&old_db, database);
                }
                return Err(format!("falha ao preparar troca de assets: {error}"));
            }
        }
        if let Err(error) = fs::rename(staging_db.join("database.db"), database) {
            if had_database {
                let _ = fs::rename(&old_db, database);
            }
            if had_assets {
                let _ = fs::rename(&old_assets, assets);
            }
            return Err(format!("falha ao ativar database: {error}"));
        }
        if let Err(error) = fs::rename(&staging_assets, assets) {
            let _ = fs::remove_file(database);
            if had_database {
                let _ = fs::rename(&old_db, database);
            }
            if had_assets {
                let _ = fs::rename(&old_assets, assets);
            }
            return Err(format!("falha ao ativar assets: {error}"));
        }
        if had_database {
            let _ = fs::remove_file(&old_db);
        }
        if had_assets {
            let _ = fs::remove_dir_all(&old_assets);
        }
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_db);
        let _ = fs::remove_dir_all(&staging_assets);
    }
    result
}

fn futures_lite_check(path: &Path) -> Result<File, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 16];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;
    if &header[..6] != b"SQLite" {
        return Err("database restaurado não é SQLite".to_string());
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verify_backup_accepts_legacy_schema_without_asset_tables() {
        let root =
            std::env::temp_dir().join(format!("presumidos-legacy-backup-{}", Uuid::new_v4()));
        let backup = root.join("backup");
        let assets = root.join("assets");
        let database = backup.join("database.db");
        fs::create_dir_all(&backup).expect("backup dir");
        fs::create_dir_all(&assets).expect("assets dir");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&database)
                    .create_if_missing(true),
            )
            .await
            .expect("legacy database");
        sqlx::raw_sql("CREATE TABLE users(id TEXT PRIMARY KEY);")
            .execute(&pool)
            .await
            .expect("legacy table");
        pool.close().await;

        archive_assets(&assets, &backup.join("assets.zip")).expect("empty assets archive");
        let metadata = BackupMetadata {
            format_version: 1,
            created_at: Utc::now().to_rfc3339(),
            application_version: "legacy-test".to_string(),
            migration_count: 18,
            expected_migration_count: Some(32),
            database_sha256: sha256_file(&database).expect("db hash"),
            assets_sha256: sha256_file(&backup.join("assets.zip")).expect("assets hash"),
            database_file: "database.db".to_string(),
            assets_file: "assets.zip".to_string(),
            counts: BackupCounts {
                users: 1,
                events: 0,
                pools: 0,
                predictions: 0,
                assets: 0,
            },
        };
        fs::write(
            backup.join("backup.json"),
            serde_json::to_vec(&metadata).expect("metadata"),
        )
        .expect("write metadata");

        verify_backup(&backup)
            .await
            .expect("legacy backup should verify");
        fs::remove_dir_all(root).expect("cleanup legacy backup test");
    }

    #[tokio::test]
    async fn backup_verify_and_restore_round_trip_preserves_database_and_assets() {
        let root = std::env::temp_dir().join(format!("presumidos-ops-test-{}", Uuid::new_v4()));
        let source = root.join("source");
        let backup = root.join("backup");
        let restored = root.join("restored");
        fs::create_dir_all(source.join("assets/hash")).expect("source assets");
        fs::create_dir_all(&backup).expect("backup dir");
        fs::write(source.join("assets/hash/master.webp"), b"asset-bytes").expect("asset");
        let database = source.join("database.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&database)
                    .create_if_missing(true),
            )
            .await
            .expect("source database");
        sqlx::raw_sql(
            "CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE events(id TEXT PRIMARY KEY);
             CREATE TABLE pools(id TEXT PRIMARY KEY);
             CREATE TABLE predictions(id TEXT PRIMARY KEY);
             CREATE TABLE assets(id TEXT PRIMARY KEY, storage_key TEXT NOT NULL);
             CREATE TABLE asset_variants(asset_id TEXT NOT NULL, storage_key TEXT NOT NULL);
             CREATE TABLE _sqlx_migrations(version INTEGER, description TEXT, installed_on TEXT, success INTEGER, checksum BLOB, execution_time INTEGER);
             INSERT INTO users VALUES('u'); INSERT INTO events VALUES('e'); INSERT INTO pools VALUES('p'); INSERT INTO predictions VALUES('pr');
             INSERT INTO assets VALUES('a','hash/master.webp'); INSERT INTO asset_variants VALUES('a','hash/master.webp');",
        )
        .execute(&pool)
        .await
        .expect("seed source database");
        pool.close().await;
        archive_assets(&source.join("assets"), &backup.join("assets.zip")).expect("archive assets");
        fs::copy(&database, backup.join("database.db")).expect("copy database snapshot");
        let metadata = BackupMetadata {
            format_version: 1,
            created_at: Utc::now().to_rfc3339(),
            application_version: "test".to_string(),
            migration_count: 0,
            expected_migration_count: Some(0),
            database_sha256: sha256_file(&backup.join("database.db")).expect("db hash"),
            assets_sha256: sha256_file(&backup.join("assets.zip")).expect("assets hash"),
            database_file: "database.db".to_string(),
            assets_file: "assets.zip".to_string(),
            counts: BackupCounts {
                users: 1,
                events: 1,
                pools: 1,
                predictions: 1,
                assets: 1,
            },
        };
        fs::write(
            backup.join("backup.json"),
            serde_json::to_vec(&metadata).expect("metadata"),
        )
        .expect("write metadata");

        verify_backup(&backup).await.expect("verify backup");
        restore_backup(
            &backup,
            &restored.join("database.db"),
            &restored.join("assets"),
            false,
        )
        .expect("restore backup");
        let restored_pool = open_backup_database(&restored.join("database.db"))
            .await
            .expect("restored database");
        database_check(&restored_pool)
            .await
            .expect("restored integrity");
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&restored_pool)
            .await
            .expect("restored users");
        assert_eq!(count.0, 1);
        restored_pool.close().await;
        assert_eq!(
            fs::read(restored.join("assets/hash/master.webp")).expect("restored asset"),
            b"asset-bytes"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn runtime_state_stops_admission_and_drains() {
        let state = RuntimeState::new();
        assert!(state.begin_request());
        state.stop_accepting();
        assert!(!state.begin_request());
        state.end_request();
        assert!(state.drain(Duration::from_millis(10)).await);
    }
}
