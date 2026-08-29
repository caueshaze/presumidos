use crate::error::ServerFnError;
use crate::models::Event;
use uuid::Uuid;

use super::core::{event, get_owned, slugify, text, EventRow};

pub async fn create(
    token: String,
    name: String,
    starts_at: Option<String>,
    ends_at: Option<String>,
    csrf: String,
) -> Result<Event, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let name = text("Nome do evento", name, 120)?;
    crate::custom_event_manifest::validate_event_window(&starts_at, &ends_at)
        .map_err(crate::security::public_error)?;
    let db = crate::db::pool();
    let base = slugify(&name);
    let mut slug = base.clone();
    let mut n = 2;
    while sqlx::query_as::<_, (String,)>("SELECT id FROM events WHERE slug=?1")
        .bind(&slug)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("event_slug", e))?
        .is_some()
    {
        slug = format!("{base}-{n}");
        n += 1;
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by,starts_at,ends_at) VALUES(?1,?2,?3,'custom','draft',?4,?5,?6)").bind(&id).bind(&name).bind(&slug).bind(&session.user_id).bind(&starts_at).bind(&ends_at).execute(db).await.map_err(|e|crate::security::internal_error("event_create",e))?;
    let initial = crate::custom_event_manifest::CustomEventManifest {
        schema_version: crate::custom_event_manifest::CURRENT_SCHEMA_VERSION,
        name: name.clone(),
        slug: slug.clone(),
        kind: "custom".into(),
        description: None,
        starts_at: starts_at.clone(),
        ends_at: ends_at.clone(),
        cover_url: None,
        cover_asset: None,
        external_url: None,
        items: Vec::new(),
    };
    let initial_fingerprint = crate::custom_event_manifest::draft_fingerprint(&initial.slug);
    sqlx::query("INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,external_url,fingerprint,base_fingerprint,created_by) VALUES(?1,?2,1,'working',0,?3,NULL,NULL,NULL,?4,?5,?6)")
        .bind(Uuid::new_v4().to_string())
        .bind(&id)
        .bind(&name)
        .bind(&initial_fingerprint)
        .bind(&initial_fingerprint)
        .bind(&session.user_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("event_create_version", e))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_created",
        "event",
        Some(&id),
        None,
        serde_json::json!({"name":name}),
    )
    .await?;
    get_owned(&session.user_id, &id).await
}

pub async fn mine(token: String) -> Result<Vec<Event>, ServerFnError> {
    let s = crate::auth::require_user(&token).await?;
    let rows: Vec<EventRow> = sqlx::query_as("SELECT e.id,COALESCE(v.name,e.name) AS name,e.slug,e.kind,e.status,e.created_by,e.starts_at,e.ends_at,e.created_at,e.updated_at,COALESCE(v.description,e.description) AS description,COALESCE(v.cover_url,e.cover_url) AS cover_url,COALESCE(v.cover_asset_id,e.cover_asset_id) AS cover_asset_id,COALESCE(v.external_url,e.external_url) AS external_url,e.pool_creation_enabled,e.current_published_version_id,e.archived_at FROM events e LEFT JOIN event_versions v ON v.id=e.current_published_version_id WHERE e.created_by=?1 AND e.archived_at IS NULL ORDER BY e.created_at DESC").bind(s.user_id).fetch_all(crate::db::pool()).await.map_err(|e|crate::security::internal_error("events_mine",e))?;
    Ok(rows.into_iter().map(event).collect())
}

/// Eventos publicados são o catálogo compartilhado para criar bolões. Não
/// concede edição: rascunhos continuam privados e a ownership segue intacta.
pub async fn available(token: String) -> Result<Vec<Event>, ServerFnError> {
    crate::auth::require_user(&token).await?;
    let rows: Vec<EventRow> = sqlx::query_as(
        "SELECT e.id,COALESCE(v.name,e.name) AS name,e.slug,e.kind,e.status,e.created_by,e.starts_at,e.ends_at,e.created_at,e.updated_at,COALESCE(v.description,e.description) AS description,COALESCE(v.cover_url,e.cover_url) AS cover_url,COALESCE(v.cover_asset_id,e.cover_asset_id) AS cover_asset_id,COALESCE(v.external_url,e.external_url) AS external_url,e.pool_creation_enabled,e.current_published_version_id,e.archived_at
         FROM events e LEFT JOIN event_versions v ON v.id=e.current_published_version_id
         WHERE e.status='active' AND e.archived_at IS NULL AND e.pool_creation_enabled=1 AND (e.ends_at IS NULL OR datetime(e.ends_at) > datetime('now'))
         ORDER BY CASE WHEN e.starts_at IS NULL THEN 1 ELSE 0 END, datetime(e.starts_at) ASC, COALESCE(v.name,e.name) COLLATE NOCASE",
    )
    .fetch_all(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("events_available", e))?;
    Ok(rows.into_iter().map(event).collect())
}
pub async fn get(token: String, id: String) -> Result<Event, ServerFnError> {
    let s = crate::auth::require_user(&token).await?;
    get_owned(&s.user_id, &id).await
}
