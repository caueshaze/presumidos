use super::*;
use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub async fn create_pool_report(
    token: String,
    pool_id: String,
    category: String,
    details: String,
    csrf_token: String,
) -> Result<PoolReport, ServerFnError> {
    use crate::auth::require_user;
    use crate::db::pool;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Bolao", &pool_id)?;
    let headers = crate::security::current_headers();
    let session = require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let category = crate::security::normalize_required_text("Motivo", category, 1, 40)?;
    if !matches!(
        category.as_str(),
        "inappropriate_content" | "spam_or_fraud" | "harassment" | "other"
    ) {
        return Err(crate::security::public_error(
            "Motivo de denuncia invalido.",
        ));
    }
    let details = crate::security::normalize_optional_text(details, 1000)?;
    let db = pool();
    let Some((pool_name, invite_code)) = sqlx::query_as::<_, (String, String)>(
        "SELECT p.name, p.invite_code
         FROM pools p
         JOIN pool_members pm ON pm.pool_id = p.id
         WHERE p.id = ?1 AND pm.user_id = ?2",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_membership", e))?
    else {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    };
    let duplicate: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM pool_reports
         WHERE pool_id = ?1 AND reporter_user_id = ?2 AND status IN ('open', 'reviewing')
         LIMIT 1",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_duplicate", e))?;
    if duplicate.is_some() {
        return Err(crate::security::public_error(
            "Voce ja possui uma denuncia aberta para este bolao.",
        ));
    }

    let report_id = uuid::Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("create_pool_report_begin_tx", e))?;
    sqlx::query(
        "INSERT INTO pool_reports
            (id, pool_id, pool_name, invite_code, reporter_user_id, category, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&report_id)
    .bind(&pool_id)
    .bind(&pool_name)
    .bind(&invite_code)
    .bind(&session.user_id)
    .bind(&category)
    .bind(&details)
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_insert", e))?;
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, ip_address, details_json)
         VALUES (?1, ?2, 'pool_report_created', 'pool', ?3, ?4, ?5)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&session.user_id)
    .bind(&pool_id)
    .bind(crate::security::client_ip(&headers))
    .bind(
        serde_json::json!({ "report_id": report_id.clone(), "category": category.clone() })
            .to_string(),
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("create_pool_report_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("create_pool_report_commit", e))?;

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
    .map_err(|e| crate::security::internal_error("create_pool_report_load", e))?;
    Ok(pool_report_from_row(row))
}

#[cfg(feature = "server")]
pub(crate) fn pool_report_from_row(row: PoolReportRow) -> PoolReport {
    PoolReport {
        id: row.id,
        pool_id: row.pool_id,
        pool_name: row.pool_name,
        invite_code: row.invite_code,
        reporter_user_id: row.reporter_user_id,
        reporter_username: row.reporter_username,
        category: row.category,
        details: row.details,
        status: row.status,
        reviewed_by: row.reviewed_by,
        reviewed_by_username: row.reviewed_by_username,
        reviewed_at: row.reviewed_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
