use super::super::*;
use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub async fn list_pool_reports_admin(
    token: String,
    status: Option<String>,
) -> Result<Vec<PoolReport>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    if let Some(value) = status.as_deref() {
        if !matches!(value, "open" | "reviewing" | "resolved" | "dismissed") {
            return Err(crate::security::public_error(
                "Status de denuncia invalido.",
            ));
        }
    }
    let rows: Vec<PoolReportRow> = sqlx::query_as(
        "SELECT r.id, r.pool_id, r.pool_name, r.invite_code, r.reporter_user_id,
                reporter.username AS reporter_username, r.category, r.details, r.status, r.reviewed_by,
                reviewer.username AS reviewed_by_username, r.reviewed_at, r.created_at, r.updated_at
         FROM pool_reports r
         LEFT JOIN users reporter ON reporter.id = r.reporter_user_id
         LEFT JOIN users reviewer ON reviewer.id = r.reviewed_by
         WHERE (?1 IS NULL OR r.status = ?1)
         ORDER BY CASE r.status WHEN 'open' THEN 0 WHEN 'reviewing' THEN 1 ELSE 2 END,
                  datetime(r.created_at) DESC",
    )
    .bind(status)
    .fetch_all(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("list_pool_reports_admin", e))?;
    Ok(rows.into_iter().map(pool_report_from_row).collect())
}

#[cfg(feature = "server")]
pub async fn update_pool_report_status_admin(
    token: String,
    report_id: String,
    status: String,
    csrf_token: String,
) -> Result<PoolReport, ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Denuncia", &report_id)?;
    if !matches!(
        status.as_str(),
        "open" | "reviewing" | "resolved" | "dismissed"
    ) {
        return Err(crate::security::public_error(
            "Status de denuncia invalido.",
        ));
    }
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let result = sqlx::query(
        "UPDATE pool_reports
         SET status = ?2,
             reviewed_by = CASE WHEN ?2 = 'open' THEN NULL ELSE ?3 END,
             reviewed_at = CASE WHEN ?2 = 'open' THEN NULL ELSE datetime('now') END,
             updated_at = datetime('now')
         WHERE id = ?1",
    )
    .bind(&report_id)
    .bind(&status)
    .bind(&session.user_id)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("update_pool_report_status", e))?;
    if result.rows_affected() == 0 {
        return Err(crate::security::public_error("Denuncia nao encontrada."));
    }
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "pool_report_status_changed",
        "pool_report",
        Some(&report_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({ "status": status }),
    )
    .await?;
    let row: PoolReportRow = sqlx::query_as(
        "SELECT r.id, r.pool_id, r.pool_name, r.invite_code, r.reporter_user_id,
                reporter.username AS reporter_username, r.category, r.details, r.status, r.reviewed_by,
                reviewer.username AS reviewed_by_username, r.reviewed_at, r.created_at, r.updated_at
         FROM pool_reports r
         LEFT JOIN users reporter ON reporter.id = r.reporter_user_id
         LEFT JOIN users reviewer ON reviewer.id = r.reviewed_by
         WHERE r.id = ?1",
    )
    .bind(&report_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("update_pool_report_load", e))?;
    Ok(pool_report_from_row(row))
}

// ---------------------------------------------------------------------------
// Ajustes manuais de pontos (organizador do bolão ou admin global)
// ---------------------------------------------------------------------------
