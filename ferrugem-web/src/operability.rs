#![cfg(feature = "server")]

//! Operabilidade da aplicação: probes, espaço em disco, métricas e backup/restore.
//!
//! Este módulo não participa do domínio de Events/Pools. O backup é um artefato
//! operacional separado dos Event Packages editoriais.

use crate::{config::settings, db};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use uuid::Uuid;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessState {
    Ready,
    NotReady,
}

#[derive(Debug, Clone)]
pub struct ReadinessReport {
    pub state: ReadinessState,
    pub database: bool,
    pub migrations: bool,
    pub assets: bool,
    pub disk: bool,
}

pub struct RuntimeState {
    accepting: AtomicBool,
    in_flight: AtomicUsize,
}

static RUNTIME: OnceLock<std::sync::Arc<RuntimeState>> = OnceLock::new();

pub fn runtime_state() -> &'static std::sync::Arc<RuntimeState> {
    RUNTIME.get_or_init(|| std::sync::Arc::new(RuntimeState::new()))
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            accepting: AtomicBool::new(true),
            in_flight: AtomicUsize::new(0),
        }
    }

    pub fn is_accepting(&self) -> bool {
        self.accepting.load(Ordering::Acquire)
    }

    pub fn stop_accepting(&self) {
        self.accepting.store(false, Ordering::Release);
    }

    pub fn begin_request(&self) -> bool {
        if !self.is_accepting() {
            return false;
        }
        self.in_flight.fetch_add(1, Ordering::AcqRel);
        if !self.is_accepting() {
            self.in_flight.fetch_sub(1, Ordering::AcqRel);
            return false;
        }
        true
    }

    pub fn end_request(&self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }

    pub async fn drain(&self, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        while self.in_flight() > 0 {
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        true
    }
}

#[derive(Default)]
pub struct Metrics {
    pub requests: AtomicU64,
    pub responses_5xx: AtomicU64,
    pub request_errors: AtomicU64,
    pub rate_limit_hits: AtomicU64,
    pub db_failures: AtomicU64,
    pub asset_failures: AtomicU64,
    pub import_failures: AtomicU64,
    pub invite_preview: AtomicU64,
    pub invite_join_success: AtomicU64,
    pub invite_join_failure: AtomicU64,
    pub in_flight: AtomicUsize,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

pub fn metrics() -> &'static Metrics {
    METRICS.get_or_init(Metrics::default)
}

pub fn metrics_text() -> String {
    let m = metrics();
    format!(
        "# TYPE presumidos_http_requests_total counter\npresumidos_http_requests_total {}\n# TYPE presumidos_http_5xx_total counter\npresumidos_http_5xx_total {}\n# TYPE presumidos_http_errors_total counter\npresumidos_http_errors_total {}\n# TYPE presumidos_rate_limit_hits_total counter\npresumidos_rate_limit_hits_total {}\n# TYPE presumidos_db_failures_total counter\npresumidos_db_failures_total {}\n# TYPE presumidos_asset_failures_total counter\npresumidos_asset_failures_total {}\n# TYPE presumidos_import_failures_total counter\npresumidos_import_failures_total {}\n# TYPE presumidos_invite_preview_total counter\npresumidos_invite_preview_total {}\n# TYPE presumidos_invite_join_success_total counter\npresumidos_invite_join_success_total {}\n# TYPE presumidos_invite_join_failure_total counter\npresumidos_invite_join_failure_total {}\n# TYPE presumidos_http_in_flight gauge\npresumidos_http_in_flight {}\n",
        m.requests.load(Ordering::Relaxed),
        m.responses_5xx.load(Ordering::Relaxed),
        m.request_errors.load(Ordering::Relaxed),
        m.rate_limit_hits.load(Ordering::Relaxed),
        m.db_failures.load(Ordering::Relaxed),
        m.asset_failures.load(Ordering::Relaxed),
        m.import_failures.load(Ordering::Relaxed),
        m.invite_preview.load(Ordering::Relaxed),
        m.invite_join_success.load(Ordering::Relaxed),
        m.invite_join_failure.load(Ordering::Relaxed),
        m.in_flight.load(Ordering::Relaxed),
    )
}

fn filesystem_free_bytes(path: &Path) -> std::io::Result<u64> {
    let path = if path.exists() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path inválido"))?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `stats` points to writable storage and the C string is NUL terminated.
    let result = unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: statvfs initialized the structure when it returned success.
    let stats = unsafe { stats.assume_init() };
    Ok(stats.f_bavail as u64 * stats.f_frsize as u64)
}

fn probe_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("storage indisponível: {e}"))?;
    let probe = path.join(format!(".readiness-{}", Uuid::new_v4()));
    let result = (|| {
        let mut file = File::create(&probe).map_err(|e| format!("storage não gravável: {e}"))?;
        file.write_all(b"probe")
            .map_err(|e| format!("storage não gravável: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("storage não sincronizável: {e}"))?;
        Ok::<(), String>(())
    })();
    let _ = fs::remove_file(&probe);
    result
}

pub fn cleanup_known_staging() -> usize {
    let mut removed = 0;
    let mut roots = vec![
        PathBuf::from(&settings().asset_dir),
        PathBuf::from(&settings().backup_dir),
        Path::new(&settings().database_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
    ];
    roots.sort();
    roots.dedup();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let known = name == ".staging"
                || name.starts_with(".staging-")
                || name.starts_with(".restore-staging-")
                || name.starts_with(".restore-db-")
                || name.starts_with(".restore-assets-")
                || name.starts_with(".restore-old-");
            if !known {
                continue;
            }
            let path = entry.path();
            let result = if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };
            if result.is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

fn distinct_filesystems(paths: &[&Path]) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for path in paths {
        let candidate = path
            .canonicalize()
            .unwrap_or_else(|_| (*path).to_path_buf());
        if !result.iter().any(|known: &PathBuf| known == &candidate) {
            result.push(candidate);
        }
    }
    result
}

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

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("não foi possível ler {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("falha ao calcular checksum: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn collect_files(
    root: &Path,
    current: &Path,
    output: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
    let entries = fs::read_dir(current).map_err(|e| format!("falha ao listar assets: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("falha ao listar assets: {e}"))?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".staging") {
            // staging incompleto não faz parte do estado publicado do AssetStore.
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, output)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "asset fora da raiz configurada".to_string())?
                .to_path_buf();
            output.push((path, relative));
        }
    }
    Ok(())
}

fn archive_assets(root: &Path, output: &Path) -> Result<(), String> {
    let file =
        File::create(output).map_err(|e| format!("falha ao criar archive de assets: {e}"))?;
    let mut writer = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    if root.exists() {
        let mut entries = Vec::new();
        collect_files(root, root, &mut entries)?;
        entries.sort_by(|left, right| left.1.cmp(&right.1));
        for (source, relative) in entries {
            let name = relative.to_string_lossy().replace('\\', "/");
            writer
                .start_file(name, options)
                .map_err(|e| format!("falha ao criar archive de assets: {e}"))?;
            let mut input = File::open(source).map_err(|e| format!("falha ao ler asset: {e}"))?;
            std::io::copy(&mut input, &mut writer)
                .map_err(|e| format!("falha ao compactar asset: {e}"))?;
        }
    }
    writer
        .finish()
        .map_err(|e| format!("falha ao finalizar archive de assets: {e}"))?;
    Ok(())
}

async fn count_rows(pool: &SqlitePool, table: &str) -> Result<i64, sqlx::Error> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    let row: (i64,) = sqlx::query_as(&query).fetch_one(pool).await?;
    Ok(row.0)
}

async fn table_exists(pool: &SqlitePool, table: &str) -> Result<bool, String> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .bind(table)
            .fetch_one(pool)
            .await
            .map_err(|error| format!("falha ao verificar tabela {table}: {error}"))?;
    Ok(row.0 != 0)
}

async fn count_rows_if_present(pool: &SqlitePool, table: &str) -> Result<i64, String> {
    if table_exists(pool, table).await? {
        count_rows(pool, table)
            .await
            .map_err(|error| format!("falha ao contar {table}: {error}"))
    } else {
        Ok(0)
    }
}

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
