use crate::error::ServerFnError;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDeletionResult {
    pub operation: String,
}

async fn delete_for_actor(
    actor_id: &str,
    event_id: &str,
    is_admin: bool,
) -> Result<EventDeletionResult, ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    let db = crate::db::pool();
    let event: Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        i64,
    )> = sqlx::query_as(
        "SELECT e.kind, e.status, e.created_by, e.archived_at, e.name, e.slug,
                    (SELECT COUNT(*) FROM pools p WHERE p.event_id=e.id)
             FROM events e WHERE e.id=?1",
    )
    .bind(event_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("event_delete_load", e))?;
    let Some((kind, _status, created_by, archived_at, name, slug, pool_count)) = event else {
        return Err(crate::security::public_error("Evento não encontrado."));
    };
    if !is_admin && (kind != "custom" || created_by.as_deref() != Some(actor_id)) {
        return Err(crate::security::public_error(
            "Somente o dono pode excluir este evento.",
        ));
    }
    if archived_at.is_some() {
        return Err(crate::security::public_error(
            "Este evento já foi arquivado.",
        ));
    }

    let operation = if kind == "custom" && created_by.is_some() && pool_count == 0 {
        "deleted"
    } else {
        "archived"
    };
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("event_delete_begin", e))?;
    if operation == "deleted" {
        // `events.current_published_version_id` points back to one of this
        // event's versions. Clear it before removing the versioned content so
        // SQLite can enforce every FK during the transaction.
        sqlx::query("UPDATE events SET current_published_version_id=NULL WHERE id=?1")
            .bind(event_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_delete_unlink_version", e))?;
        // `prediction_items.event_id` is intentionally not cascading: items
        // are preserved when an EventVersion is published and used by a Pool.
        // A no-Pool event is the one safe permanent-delete case, so remove its
        // items explicitly; their question/result children cascade.
        sqlx::query("DELETE FROM prediction_items WHERE event_id=?1")
            .bind(event_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_delete_items", e))?;
        sqlx::query("DELETE FROM events WHERE id=?1 AND archived_at IS NULL")
            .bind(event_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_delete", e))?;
    } else {
        sqlx::query(
            "UPDATE events
             SET archived_at=datetime('now'), archived_by=?2,
                 pool_creation_enabled=0, updated_at=datetime('now')
             WHERE id=?1 AND archived_at IS NULL",
        )
        .bind(event_id)
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("event_archive", e))?;
    }
    sqlx::query(
        "INSERT INTO audit_logs
            (id, actor_user_id, action, target_type, target_id, details_json)
         VALUES (?1, ?2, ?3, 'event', ?4, ?5)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(actor_id)
    .bind(if operation == "deleted" {
        "event_deleted"
    } else {
        "event_archived"
    })
    .bind(event_id)
    .bind(
        serde_json::json!({
            "name": name,
            "slug": slug,
            "origin": if created_by.is_some() { "user" } else { "system" },
            "pool_count": pool_count,
            "operation": operation,
        })
        .to_string(),
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("event_delete_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("event_delete_commit", e))?;
    Ok(EventDeletionResult {
        operation: operation.to_string(),
    })
}

pub async fn delete(
    token: String,
    event_id: String,
    csrf: String,
) -> Result<EventDeletionResult, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    delete_for_actor(&session.user_id, &event_id, false).await
}

pub async fn delete_admin(
    token: String,
    event_id: String,
    csrf: String,
) -> Result<EventDeletionResult, ServerFnError> {
    let session = crate::auth::require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    delete_for_actor(&session.user_id, &event_id, true).await
}
