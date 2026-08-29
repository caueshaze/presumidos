use crate::error::ServerFnError;

use super::core::text;

pub async fn update_metadata(
    token: String,
    event_id: String,
    name: String,
    starts_at: Option<String>,
    ends_at: Option<String>,
    description: Option<String>,
    cover_url: Option<String>,
    external_url: Option<String>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let db = crate::db::pool();
    let access: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT e.status,e.created_by,e.starts_at,e.ends_at FROM events e WHERE e.id=?1 AND e.kind='custom' AND e.archived_at IS NULL AND (e.created_by=?2 OR EXISTS (SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1))",
    )
    .bind(&event_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("event_metadata_access", e))?;
    let Some((status, _owner_id, current_starts, current_ends)) = access else {
        return Err(crate::security::public_error(
            "Você não pode editar este evento.",
        ));
    };
    let is_admin: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id=?1")
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("event_metadata_admin", e))?;
    if status != "draft" && !is_admin.0 {
        return Err(crate::security::public_error(
            "Eventos publicados só permitem edição editorial por um administrador.",
        ));
    }
    if status != "draft" && (starts_at != current_starts || ends_at != current_ends) {
        return Err(crate::security::public_error(
            "Datas operacionais de evento publicado são imutáveis.",
        ));
    }
    let name = text("Nome do evento", name, 120)?;
    crate::custom_event_manifest::validate_event_window(&starts_at, &ends_at)
        .map_err(crate::security::public_error)?;
    let description = description
        .map(|value| crate::security::normalize_optional_text(value, 1200))
        .transpose()?
        .filter(|value| !value.is_empty());
    let cover_url = crate::custom_event_manifest::validate_optional_http_url(cover_url, "coverUrl")
        .map_err(crate::security::public_error)?;
    let external_url =
        crate::custom_event_manifest::validate_optional_http_url(external_url, "externalUrl")
            .map_err(crate::security::public_error)?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    sqlx::query("UPDATE events SET starts_at=?2,ends_at=?3,updated_at=datetime('now') WHERE id=?1")
        .bind(&event_id)
        .bind(&starts_at)
        .bind(&ends_at)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("event_update_metadata", e))?;
    sqlx::query("UPDATE event_versions SET name=?2,description=?3,cover_url=?4,external_url=?5,updated_at=datetime('now') WHERE id=?1 AND state='working'")
        .bind(&version_id).bind(&name).bind(&description).bind(&cover_url).bind(&external_url)
        .execute(db).await
        .map_err(|e| crate::security::internal_error("event_update_version_metadata", e))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_metadata_changed",
        "event",
        Some(&event_id),
        None,
        serde_json::json!({"workingVersionId": version_id, "fields": ["name", "description", "coverUrl", "externalUrl"]}),
    )
    .await?;
    Ok(())
}
