use crate::error::ServerFnError;
use crate::models::Event;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_ITEMS: usize = 100;
pub(crate) const MAX_OPTIONS: usize = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderOption {
    pub id: String,
    pub label: String,
    pub sort_order: i64,
    pub image_url: Option<String>,
    pub image_asset_url: Option<String>,
    pub links: Vec<BuilderOptionLink>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderOptionLink {
    pub kind: String,
    pub label: String,
    pub url: String,
    pub sort_order: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub lock_at: String,
    pub reveal_at: String,
    pub sort_order: i64,
    pub correct_option_id: Option<String>,
    pub decimal_places: Option<i64>,
    pub unit_label: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub result_value: Option<String>,
    pub min_selections: Option<i64>,
    pub max_selections: Option<i64>,
    pub options: Vec<BuilderOption>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderDraft {
    pub event: Event,
    pub items: Vec<BuilderItem>,
    pub versions: Vec<BuilderVersion>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderVersion {
    pub id: String,
    pub version_number: i64,
    pub state: String,
    pub is_current_published: bool,
    pub name: String,
    pub fingerprint: String,
    pub base_fingerprint: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub item_count: i64,
    pub option_count: i64,
    pub pool_count: i64,
}

pub(crate) fn text(label: &str, value: String, max: usize) -> Result<String, ServerFnError> {
    crate::security::normalize_required_text(label, value, 1, max)
}

pub(crate) fn slugify(name: &str) -> String {
    let folded = name
        .replace(['á', 'à', 'â', 'ã', 'ä'], "a")
        .replace(['é', 'ê', 'ë'], "e")
        .replace(['í', 'ï'], "i")
        .replace(['ó', 'ô', 'õ', 'ö'], "o")
        .replace(['ú', 'ü'], "u")
        .replace('ç', "c");
    let mut slug = String::new();
    for c in folded.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "evento".into()
    } else {
        slug.to_string()
    }
}

pub(crate) async fn owner(
    token: String,
    event_id: &str,
    csrf: Option<String>,
) -> Result<(crate::auth::AuthSession, &'static sqlx::SqlitePool, String), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    if let Some(csrf) = csrf {
        crate::security::require_csrf(&session.csrf_token, &csrf)?;
    }
    let db = crate::db::pool();
    let allowed: Option<(String, String)> = sqlx::query_as(
        "SELECT e.id,e.status FROM events e WHERE e.id=?1 AND e.kind='custom' AND e.archived_at IS NULL AND
         ((e.status='draft' AND (e.created_by=?2 OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)))
          OR (e.status IN ('active','finished') AND EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)))",
    )
    .bind(&event_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("event_builder_owner", e))?;
    let Some((_id, status)) = allowed else {
        return Err(crate::security::public_error(
            "Somente o dono pode editar este rascunho.",
        ));
    };
    let is_admin: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id=?1")
        .bind(&session.user_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("event_builder_admin", e))?;
    let version_id: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("event_builder_working", e))?;
    let version_id = match version_id {
        Some((id,)) => id,
        None if status == "draft" || is_admin.0 => {
            crate::custom_event_manifest::ensure_working_revision(event_id, &session.user_id)
                .await?
        }
        None => {
            return Err(crate::security::public_error(
                "Este evento publicado só pode ser editado por um administrador.",
            ))
        }
    };
    Ok((session, db, version_id))
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct EventRow {
    id: String,
    name: String,
    slug: String,
    kind: String,
    status: String,
    created_by: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    created_at: String,
    updated_at: String,
    description: Option<String>,
    cover_url: Option<String>,
    cover_asset_id: Option<String>,
    external_url: Option<String>,
    pool_creation_enabled: i64,
    current_published_version_id: Option<String>,
    archived_at: Option<String>,
}
pub(crate) fn event(row: EventRow) -> Event {
    Event {
        id: row.id,
        name: row.name,
        slug: row.slug,
        kind: if row.kind == "custom" {
            crate::models::EventKind::Custom
        } else {
            crate::models::EventKind::Football
        },
        origin: if row.created_by.is_some() {
            crate::models::EventOrigin::User
        } else {
            crate::models::EventOrigin::System
        },
        status: match row.status.as_str() {
            "draft" => crate::models::EventStatus::Draft,
            "finished" => crate::models::EventStatus::Finished,
            _ => crate::models::EventStatus::Active,
        },
        created_by: row.created_by,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        description: row.description,
        cover_url: row.cover_url,
        cover_asset_id: row.cover_asset_id.clone(),
        cover_asset_url: row
            .cover_asset_id
            .map(|asset_id| format!("/media/assets/{asset_id}/cover")),
        external_url: row.external_url,
        pool_creation_enabled: row.pool_creation_enabled != 0,
        current_published_version_id: row.current_published_version_id,
        archived_at: row.archived_at,
    }
}
pub(crate) async fn get_owned(user: &str, id: &str) -> Result<Event, ServerFnError> {
    let row: Option<EventRow> = sqlx::query_as("SELECT e.id,COALESCE(v.name,e.name) AS name,e.slug,e.kind,e.status,e.created_by,e.starts_at,e.ends_at,e.created_at,e.updated_at,COALESCE(v.description,e.description) AS description,COALESCE(v.cover_url,e.cover_url) AS cover_url,COALESCE(v.cover_asset_id,e.cover_asset_id) AS cover_asset_id,COALESCE(v.external_url,e.external_url) AS external_url,e.pool_creation_enabled,e.current_published_version_id,e.archived_at FROM events e LEFT JOIN event_versions v ON v.id=e.current_published_version_id WHERE e.id=?1 AND e.archived_at IS NULL AND (e.created_by=?2 OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1))").bind(id).bind(user).fetch_optional(crate::db::pool()).await.map_err(|e|crate::security::internal_error("event_get",e))?;
    row.map(event)
        .ok_or_else(|| crate::security::public_error("Evento não encontrado."))
}
