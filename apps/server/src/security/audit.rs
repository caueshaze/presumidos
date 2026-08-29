use crate::{config::settings, error::ServerFnError};

pub fn log_event(kind: &str, details: serde_json::Value) {
    if kind.contains("rate_limit") {
        crate::operability::metrics()
            .rate_limit_hits
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    let mut details = redact_json(details, false);
    if let Some(object) = details.as_object_mut() {
        object
            .entry("request_id".to_string())
            .or_insert_with(|| serde_json::Value::String(crate::context::request_id()));
    }
    let line = serde_json::json!({
        "kind": kind,
        "at": chrono::Utc::now().to_rfc3339(),
        "details": details,
    });
    if settings().json_logs {
        eprintln!("{line}");
    } else {
        eprintln!("[{kind}] {}", line["details"]);
    }
}

#[cfg(feature = "server")]
fn redact_json(value: serde_json::Value, inherited_sensitive: bool) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    let normalized = key.to_lowercase();
                    let sensitive = inherited_sensitive
                        || normalized.contains("password")
                        || normalized.contains("secret")
                        || normalized.contains("cookie")
                        || normalized.contains("authorization")
                        || normalized.contains("csrf")
                        || normalized.contains("token");
                    (key, redact_json(value, sensitive))
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|value| redact_json(value, inherited_sensitive))
                .collect(),
        ),
        serde_json::Value::String(_) if inherited_sensitive => {
            serde_json::Value::String("[REDACTED]".to_string())
        }
        other => other,
    }
}

#[cfg(feature = "server")]
pub fn public_error(message: impl Into<String>) -> ServerFnError {
    ServerFnError::new(message.into())
}

#[cfg(feature = "server")]
pub fn internal_error(context: &str, error: impl std::fmt::Display) -> ServerFnError {
    let error_text = error.to_string();
    log_event(
        "internal_error",
        serde_json::json!({
            "context": context,
            "error": error_text,
        }),
    );
    let storage_failure = is_storage_failure(&error_text);
    if context.starts_with("asset_") {
        if storage_failure {
            crate::operability::metrics()
                .asset_failures
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    if context.starts_with("db_") || context.contains("database") {
        crate::operability::metrics()
            .db_failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    if storage_failure {
        return public_error("STORAGE: a operação de storage não pôde ser concluída.");
    }
    public_error("O servidor nao conseguiu concluir essa operacao agora.")
}

#[cfg(feature = "server")]
fn is_storage_failure(error: &str) -> bool {
    let normalized = error.to_lowercase();
    normalized.contains("no space left")
        || normalized.contains("enospc")
        || normalized.contains("sqlite_full")
        || normalized.contains("disk full")
        || normalized.contains("disk is full")
        || normalized.contains("database or disk is full")
}

#[cfg(feature = "server")]
pub async fn append_audit_log(
    db: &sqlx::SqlitePool,
    actor_user_id: Option<&str>,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    ip_address: Option<&str>,
    details: serde_json::Value,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip_address)
    .bind(details.to_string())
    .execute(db)
    .await
    .map_err(|e| internal_error("append_audit_log", e))?;

    Ok(())
}
