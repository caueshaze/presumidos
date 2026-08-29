pub async fn readiness_report() -> ReadinessReport {
    let database = if sqlx::query("SELECT 1").execute(db::pool()).await.is_ok() {
        db::quick_check().await.is_ok_and(|result| result == "ok")
    } else {
        false
    };
    let migrations = if database {
        db::migration_status()
            .await
            .map(|(applied, expected)| applied == expected)
            .unwrap_or(false)
    } else {
        false
    };
    let assets = probe_directory(Path::new(&settings().asset_dir)).is_ok();
    let database_path = Path::new(&settings().database_path);
    let asset_path = Path::new(&settings().asset_dir);
    let disk = distinct_filesystems(&[database_path, asset_path])
        .iter()
        .all(|path| {
            filesystem_free_bytes(path).is_ok_and(|free| free >= settings().min_free_bytes)
        });
    let state = if database && migrations && assets && disk {
        ReadinessState::Ready
    } else {
        ReadinessState::NotReady
    };
    ReadinessReport {
        state,
        database,
        migrations,
        assets,
        disk,
    }
}

pub async fn database_check(pool: &SqlitePool) -> Result<(), String> {
    let row: (String,) = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("integrity_check indisponível: {e}"))?;
    if row.0 != "ok" {
        return Err(format!("integrity_check falhou: {}", row.0));
    }
    Ok(())
}

