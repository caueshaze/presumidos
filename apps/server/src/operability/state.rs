
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

