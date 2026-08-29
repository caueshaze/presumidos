#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupMetadata {
    pub format_version: u32,
    pub created_at: String,
    pub application_version: String,
    pub migration_count: i64,
    #[serde(default)]
    pub expected_migration_count: Option<i64>,
    pub database_sha256: String,
    pub assets_sha256: String,
    pub database_file: String,
    pub assets_file: String,
    pub counts: BackupCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCounts {
    pub users: i64,
    pub events: i64,
    pub pools: i64,
    pub predictions: i64,
    pub assets: i64,
}

fn escaped_sqlite_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

pub async fn create_backup(output: &Path) -> Result<PathBuf, String> {
    if output.exists() && output.is_file() {
        return Err("o destino de backup precisa ser um diretório".to_string());
    }
    if !output.exists() {
        fs::create_dir_all(output)
            .map_err(|e| format!("falha ao criar diretório de backup: {e}"))?;
    }
    let staging = output.join(format!(".staging-{}", Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|e| format!("falha ao criar staging do backup: {e}"))?;
    let result = async {
        let database_file = staging.join("database.db");
        let assets_file = staging.join("assets.zip");
        let statement = format!("VACUUM INTO '{}'", escaped_sqlite_path(&database_file));
        sqlx::query(&statement)
            .execute(db::pool())
            .await
            .map_err(|e| format!("falha ao criar snapshot SQLite: {e}"))?;
        archive_assets(Path::new(&settings().asset_dir), &assets_file)?;
        let (migration_count, expected) = db::migration_status()
            .await
            .map_err(|e| format!("falha ao ler migrations: {e}"))?;
        let metadata = BackupMetadata {
            format_version: 1,
            created_at: Utc::now().to_rfc3339(),
            application_version: env!("CARGO_PKG_VERSION").to_string(),
            migration_count,
            expected_migration_count: Some(expected),
            database_sha256: sha256_file(&database_file)?,
            assets_sha256: sha256_file(&assets_file)?,
            database_file: "database.db".to_string(),
            assets_file: "assets.zip".to_string(),
            counts: BackupCounts {
                users: count_rows_if_present(db::pool(), "users").await?,
                events: count_rows_if_present(db::pool(), "events").await?,
                pools: count_rows_if_present(db::pool(), "pools").await?,
                predictions: count_rows_if_present(db::pool(), "predictions").await?,
                assets: count_rows_if_present(db::pool(), "assets").await?,
            },
        };
        fs::write(
            staging.join("backup.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .map_err(|e| format!("falha ao gravar metadata: {e}"))?;
        verify_backup(&staging).await?;
        let final_dir = output.join(format!(
            "backup-{}-{}",
            Utc::now().format("%Y%m%dT%H%M%SZ"),
            &Uuid::new_v4().to_string()[..8]
        ));
        fs::rename(&staging, &final_dir).map_err(|e| format!("falha ao publicar backup: {e}"))?;
        Ok(final_dir)
    }
    .await;
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

async fn open_backup_database(path: &Path) -> Result<SqlitePool, String> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .busy_timeout(Duration::from_millis(5_000));
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| format!("não foi possível abrir DB do backup: {e}"))
}

pub async fn verify_backup(backup: &Path) -> Result<(), String> {
    let metadata_bytes = fs::read(backup.join("backup.json"))
        .map_err(|e| format!("backup.json ausente ou ilegível: {e}"))?;
    let metadata: BackupMetadata = serde_json::from_slice(&metadata_bytes)
        .map_err(|e| format!("backup.json inválido: {e}"))?;
    if metadata.format_version != 1 {
        return Err("versão de formato de backup não suportada".to_string());
    }
    let database = backup.join(&metadata.database_file);
    let assets = backup.join(&metadata.assets_file);
    if sha256_file(&database)? != metadata.database_sha256 {
        return Err("checksum do database.db não confere".to_string());
    }
    if sha256_file(&assets)? != metadata.assets_sha256 {
        return Err("checksum do assets.zip não confere".to_string());
    }
    let pool = open_backup_database(&database).await?;
    database_check(&pool).await?;
    let mut archive = ZipArchive::new(File::open(&assets).map_err(|e| e.to_string())?)
        .map_err(|e| format!("archive de assets inválido: {e}"))?;
    let mut names = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| format!("archive de assets inválido: {e}"))?;
        if entry.name().starts_with('/') || entry.name().contains("..") {
            return Err("archive de assets contém caminho inválido".to_string());
        }
        names.insert(entry.name().to_string());
    }
    let references: Vec<(String,)> =
        if table_exists(&pool, "assets").await? && table_exists(&pool, "asset_variants").await? {
            sqlx::query_as(
                "SELECT storage_key FROM assets UNION SELECT storage_key FROM asset_variants",
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("não foi possível validar referências de assets: {e}"))?
        } else {
            Vec::new()
        };
    for (reference,) in references {
        if !names.contains(&reference) {
            return Err(format!("asset referenciado ausente no backup: {reference}"));
        }
    }
    pool.close().await;
    Ok(())
}

