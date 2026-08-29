use super::super::*;
use crate::error::ServerFnError;
use serde::Serialize;
#[cfg(feature = "server")]

pub async fn ensure_working_revision(event_id: &str, actor: &str) -> Result<String, ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    let db = crate::db::pool();
    if let Some((id, base_fingerprint, fingerprint_value)) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,COALESCE(base_fingerprint,''),fingerprint FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("working_revision_lookup", e))?
    {
        // A previous process could have persisted the revision header before
        // failing while copying items. Repair only that exact incomplete copy;
        // a legitimately edited empty revision has a different fingerprint.
        let published_fingerprint = if sqlx::query_as::<_, (String,)>(
            "SELECT id FROM event_versions WHERE event_id=?1 AND state='published' ORDER BY version_number DESC LIMIT 1",
        )
        .bind(event_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("working_revision_repair_published", e))?
        .is_some()
        {
            Some(fingerprint(&load_manifest(db, event_id).await?).map_err(crate::security::public_error)?)
        } else {
            None
        };
        let item_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1",
        )
        .bind(&id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("working_revision_repair_items", e))?;
        if item_count.0 == 0
            && published_fingerprint
                .as_ref()
                .is_some_and(|published| published == &base_fingerprint && published == &fingerprint_value)
        {
            let (manifest, _) = export_for_event(event_id).await?;
            let mut tx = db
                .begin()
                .await
                .map_err(|e| crate::security::internal_error("working_revision_repair_begin", e))?;
            insert_items(&mut tx, event_id, &id, &manifest).await?;
            tx.commit()
                .await
                .map_err(|e| crate::security::internal_error("working_revision_repair_commit", e))?;
        }
        return Ok(id);
    }

    let event: (String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) =
        sqlx::query_as("SELECT id,name,slug,status,starts_at,ends_at,description,cover_url FROM events WHERE id=?1 AND kind='custom'")
            .bind(event_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("working_revision_event", e))?
            .ok_or_else(|| crate::security::public_error("Evento não encontrado."))?;

    let current: Option<CustomEventManifest> = if sqlx::query_as::<_, (String,)>(
        "SELECT id FROM event_versions WHERE event_id=?1 AND state='published' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("working_revision_published", e))?
    .is_some()
    {
        Some(load_manifest(db, event_id).await?)
    } else {
        None
    };
    let current_cover_asset_id: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT cover_asset_id FROM event_versions WHERE event_id=?1 AND state='published' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("working_revision_cover", e))?;

    let version_id = uuid::Uuid::new_v4().to_string();
    let number: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(version_number),0)+1 FROM event_versions WHERE event_id=?1",
    )
    .bind(event_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("working_revision_number", e))?;
    let (name, description, cover_url, external_url, fingerprint_value, base) =
        if let Some(m) = current.as_ref() {
            (
                m.name.clone(),
                m.description.clone(),
                m.cover_url.clone(),
                m.external_url.clone(),
                fingerprint(&m).map_err(crate::security::public_error)?,
                fingerprint(&m).map_err(crate::security::public_error)?,
            )
        } else {
            let m = CustomEventManifest {
                schema_version: CURRENT_SCHEMA_VERSION,
                name: event.1.clone(),
                slug: event.2.clone(),
                kind: "custom".into(),
                description: event.6.clone(),
                starts_at: event.4.clone(),
                ends_at: event.5.clone(),
                cover_url: event.7.clone(),
                cover_asset: None,
                external_url: None,
                items: Vec::new(),
            };
            let fp = draft_fingerprint(&m.slug);
            (
                m.name,
                m.description,
                m.cover_url,
                m.external_url,
                fp.clone(),
                fp,
            )
        };
    let current_manifest = if current.is_some() {
        Some(export_for_event(event_id).await?.0)
    } else {
        None
    };
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("working_revision_create_begin", e))?;
    sqlx::query("INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,base_fingerprint,created_by) VALUES(?1,?2,?3,'working',0,?4,?5,?6,?7,?8,?9,?10,?11)")
        .bind(&version_id).bind(event_id).bind(number.0).bind(name).bind(description).bind(cover_url).bind(current_cover_asset_id.and_then(|v| v.0)).bind(external_url).bind(&fingerprint_value).bind(&base).bind(actor)
        .execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("working_revision_create", e))?;
    if let Some(manifest) = current_manifest.as_ref() {
        insert_items(&mut tx, event_id, &version_id, manifest).await?;
    } else {
        // Compatibilidade com drafts legados criados antes da associação
        // obrigatória de itens a uma EventVersion. A revisão de trabalho nova
        // passa a ser a dona desses itens, permitindo que o Builder e o
        // upload de assets continuem operando sem editar conteúdo publicado.
        sqlx::query(
            "UPDATE prediction_items SET event_version_id=?2
             WHERE event_id=?1 AND event_version_id IS NULL",
        )
        .bind(event_id)
        .bind(&version_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("working_revision_adopt_legacy_items", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("working_revision_create_commit", e))?;
    Ok(version_id)
}

#[cfg(feature = "server")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreVersionResult {
    pub version_id: String,
    pub version_number: i64,
    pub source_version_id: String,
    pub source_version_number: i64,
    pub replaced_working_version_id: Option<String>,
}
