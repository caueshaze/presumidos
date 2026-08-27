use crate::error::ServerFnError;
use serde::Serialize;

mod core;
pub(crate) use core::*;

#[cfg(feature = "server")]
use sqlx::{sqlite::SqliteConnection, Sqlite, SqlitePool};

#[cfg(feature = "server")]
async fn load_manifest_conn(
    conn: &mut SqliteConnection,
    event_id: &str,
    version_id: Option<&str>,
) -> Result<CustomEventManifest, ServerFnError> {
    let event: Option<(String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>)> = sqlx::query_as("SELECT v.name,e.slug,e.kind,v.description,e.starts_at,e.ends_at,v.cover_url,v.external_url,v.cover_asset_id,a.sha256,a.media_type FROM events e JOIN event_versions v ON v.id=COALESCE(?2,e.current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=e.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) LEFT JOIN assets a ON a.id=v.cover_asset_id WHERE e.id=?1").bind(event_id).bind(version_id).fetch_optional(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_event", e))?;
    let Some((
        name,
        slug,
        kind,
        description,
        starts_at,
        ends_at,
        cover_url,
        external_url,
        _cover_asset_id,
        cover_sha256,
        cover_media_type,
    )) = event
    else {
        return Err(crate::security::public_error("Evento não encontrado."));
    };
    let rows: Vec<(String,String,String,Option<String>,String,String,i64,Option<i64>,Option<String>,Option<i64>,Option<i64>,Option<i64>,Option<i64>)> = sqlx::query_as("SELECT pi.external_key,pi.kind,pi.title,pi.description,pi.lock_at,pi.reveal_at,pi.sort_order,n.decimal_places,n.unit_label,n.min_value_scaled,n.max_value_scaled,mq.min_selections,mq.max_selections FROM prediction_items pi LEFT JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_version_id=COALESCE(?2,(SELECT COALESCE(current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=events.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) FROM events WHERE id=?1)) ORDER BY pi.sort_order,pi.id").bind(event_id).bind(version_id).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_items", e))?;
    let mut items = Vec::new();
    for (
        external_key,
        kind,
        title,
        item_description,
        lock_at,
        reveal_at,
        _sort,
        decimal_places,
        unit_label,
        min_scaled,
        max_scaled,
        min_selections,
        max_selections,
    ) in rows
    {
        let Some(external_key) = Some(external_key) else {
            return Err(crate::security::public_error("item custom sem externalKey"));
        };
        let options_rows: Vec<(String,String,i64,Option<String>,Option<String>,Option<String>)> = sqlx::query_as("SELECT o.external_key,o.label,o.sort_order,o.image_url,a.sha256,a.media_type FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id LEFT JOIN assets a ON a.id=o.image_asset_id WHERE pi.event_version_id=COALESCE(?3,(SELECT COALESCE(current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=events.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) FROM events WHERE id=?1)) AND pi.external_key=?2 ORDER BY o.sort_order,o.id").bind(event_id).bind(&external_key).bind(version_id).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_options", e))?;
        let mut options = Vec::new();
        for (option_key, label, _sort, image_url, image_sha256, image_media_type) in options_rows {
            let links: Vec<(String,String,String,i64)> = sqlx::query_as("SELECT l.kind,l.label,l.url,l.sort_order FROM option_links l JOIN custom_question_options o ON o.id=l.option_id JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_version_id=COALESCE(?4,(SELECT COALESCE(current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=events.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) FROM events WHERE id=?1)) AND pi.external_key=?2 AND o.external_key=?3 ORDER BY l.sort_order,l.id").bind(event_id).bind(&external_key).bind(&option_key).bind(version_id).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_links", e))?;
            options.push(CustomEventManifestOption {
                external_key: option_key,
                label,
                image_url,
                image_asset: image_sha256
                    .zip(image_media_type)
                    .map(|(sha256, media_type)| AssetReference {
                        kind: "asset".into(),
                        sha256,
                        media_type,
                    }),
                links: links
                    .into_iter()
                    .map(|(kind, label, url, _)| CustomEventManifestOptionLink { kind, label, url })
                    .collect(),
            });
        }
        let values = decimal_places
            .map(|p| {
                let p = p as u8;
                (
                    min_scaled.map(|v| crate::numeric::display_scaled(v, p)),
                    max_scaled.map(|v| crate::numeric::display_scaled(v, p)),
                )
            })
            .unwrap_or((None, None));
        items.push(CustomEventManifestItem {
            external_key,
            kind,
            title,
            description: item_description,
            lock_at,
            reveal_at,
            options,
            decimal_places,
            unit_label,
            min_value: values.0,
            max_value: values.1,
            min_selections,
            max_selections,
        });
    }
    let m = CustomEventManifest {
        schema_version: CURRENT_SCHEMA_VERSION,
        name,
        slug,
        kind,
        description,
        starts_at,
        ends_at,
        cover_url,
        cover_asset: cover_sha256
            .zip(cover_media_type)
            .map(|(sha256, media_type)| AssetReference {
                kind: "asset".into(),
                sha256,
                media_type,
            }),
        external_url,
        items,
    };
    parse_and_validate(
        &serde_json::to_string(&m)
            .map_err(|e| crate::security::internal_error("manifest_export_serialize", e))?,
    )
    .map_err(crate::security::public_error)
}

#[cfg(feature = "server")]
async fn load_manifest(
    db: &SqlitePool,
    event_id: &str,
) -> Result<CustomEventManifest, ServerFnError> {
    let mut conn = db
        .acquire()
        .await
        .map_err(|e| crate::security::internal_error("manifest_export_connection", e))?;
    load_manifest_conn(&mut conn, event_id, None).await
}

/// Ensures that an editor has an isolated working copy.  The published
/// version is never edited in place; the copy gets fresh item/option IDs and
/// therefore cannot change the history of existing Pools.
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

/// Recreates a previous published definition as a fresh working revision.
/// The source version and every Pool that already points to it remain intact.
#[cfg(feature = "server")]
pub async fn restore_published_version(
    event_id: &str,
    source_version_id: &str,
    actor: &str,
) -> Result<RestoreVersionResult, ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    crate::security::validate_uuid("Versão", source_version_id)?;
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("version_restore_begin", e))?;

    let source: (String, i64, String, Option<String>, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT v.id,v.version_number,v.name,v.description,v.cover_url,v.cover_asset_id,v.external_url
         FROM event_versions v JOIN events e ON e.id=v.event_id
         WHERE v.id=?1 AND v.event_id=?2 AND v.state='published' AND e.kind='custom'",
    )
    .bind(source_version_id)
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_source", e))?
    .ok_or_else(|| crate::security::public_error("A versão publicada não foi encontrada."))?;

    let manifest = load_manifest_conn(&mut *tx, event_id, Some(source_version_id)).await?;
    let restored_fingerprint = fingerprint(&manifest).map_err(crate::security::public_error)?;
    let current_fingerprint: Option<(String,)> = sqlx::query_as(
        "SELECT fingerprint FROM event_versions WHERE event_id=?1 AND state='published' AND is_current_published=1",
    )
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_current", e))?;
    let base_fingerprint = match current_fingerprint.map(|row| row.0) {
        Some(value) if !value.is_empty() => value,
        _ => fingerprint(&load_manifest_conn(&mut *tx, event_id, None).await?)
            .map_err(crate::security::public_error)?,
    };

    let working: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_working", e))?;
    let replaced_working_version_id = working.as_ref().map(|row| row.0.clone());
    if let Some((working_id,)) = working {
        let pool_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pools WHERE event_version_id=?1")
                .bind(&working_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| crate::security::internal_error("version_restore_working_pools", e))?;
        if pool_count.0 > 0 {
            return Err(crate::security::public_error(
                "A revisão atual já está vinculada a bolões e não pode ser substituída.",
            ));
        }
        sqlx::query("DELETE FROM official_results WHERE event_version_id=?1")
            .bind(&working_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("version_restore_working_results", e))?;
        sqlx::query("DELETE FROM prediction_items WHERE event_version_id=?1")
            .bind(&working_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("version_restore_working_items", e))?;
        sqlx::query("DELETE FROM event_versions WHERE id=?1 AND state='working'")
            .bind(&working_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("version_restore_working_delete", e))?;
    }

    let version_id = uuid::Uuid::new_v4().to_string();
    let next_number: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(version_number),0)+1 FROM event_versions WHERE event_id=?1",
    )
    .bind(event_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_number", e))?;
    sqlx::query(
        "INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,base_fingerprint,created_by)
         VALUES(?1,?2,?3,'working',0,?4,?5,?6,?7,?8,?9,?10,?11)",
    )
    .bind(&version_id)
    .bind(event_id)
    .bind(next_number.0)
    .bind(&source.2)
    .bind(&source.3)
    .bind(&source.4)
    .bind(&source.5)
    .bind(&source.6)
    .bind(&restored_fingerprint)
    .bind(&base_fingerprint)
    .bind(actor)
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_insert", e))?;
    insert_items(&mut tx, event_id, &version_id, &manifest).await?;
    sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,ip_address,details_json) VALUES(?1,?2,'event_version_restored','event',?3,NULL,?4)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(actor)
        .bind(event_id)
        .bind(serde_json::json!({
            "sourceVersionId": source.0,
            "sourceVersionNumber": source.1,
            "newVersionId": version_id,
            "newVersionNumber": next_number.0,
            "replacedWorkingVersionId": replaced_working_version_id,
            "itemCount": manifest.items.len(),
        }).to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("version_restore_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("version_restore_commit", e))?;
    Ok(RestoreVersionResult {
        version_id,
        version_number: next_number.0,
        source_version_id: source.0,
        source_version_number: source.1,
        replaced_working_version_id,
    })
}

#[cfg(feature = "server")]
pub async fn publish_working_revision(
    event_id: &str,
    version_id: Option<&str>,
    actor: &str,
) -> Result<(), ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("working_publish_begin", e))?;
    let version: (String, i64) = if let Some(version_id) = version_id {
        sqlx::query_as("SELECT id,version_number FROM event_versions WHERE id=?1 AND event_id=?2 AND state='working'")
            .bind(version_id).bind(event_id).fetch_optional(&mut *tx).await
            .map_err(|e| crate::security::internal_error("working_publish_lookup", e))?
            .ok_or_else(|| crate::security::public_error("Revisão de evento não encontrada ou já publicada."))?
    } else {
        sqlx::query_as("SELECT id,version_number FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1")
            .bind(event_id).fetch_optional(&mut *tx).await
            .map_err(|e| crate::security::internal_error("working_publish_latest", e))?
            .ok_or_else(|| crate::security::public_error("Não há revisão pendente para publicar."))?
    };
    let invalid: Option<(String,)> = sqlx::query_as(
        "SELECT pi.title FROM prediction_items pi LEFT JOIN custom_question_options o ON o.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_version_id=?1 AND pi.kind IN ('single_choice','multiple_choice') GROUP BY pi.id HAVING COUNT(o.id)<2 OR (pi.kind='multiple_choice' AND (mq.min_selections<1 OR COALESCE(mq.max_selections,COUNT(o.id))<mq.min_selections OR COALESCE(mq.max_selections,COUNT(o.id))>COUNT(o.id))) LIMIT 1",
    )
    .bind(&version.0).fetch_optional(&mut *tx).await
    .map_err(|e| crate::security::internal_error("working_publish_validate", e))?;
    if let Some((title,)) = invalid {
        return Err(crate::security::public_error(&format!(
            "{title} precisa ter pelo menos 2 opções."
        )));
    }
    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1")
            .bind(&version.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("working_publish_count", e))?;
    if total.0 == 0 {
        return Err(crate::security::public_error(
            "O evento precisa ter pelo menos uma pergunta.",
        ));
    }
    sqlx::query("UPDATE event_versions SET state='published',is_current_published=0,updated_at=datetime('now') WHERE event_id=?1 AND state='published'")
        .bind(event_id).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("working_publish_previous", e))?;
    sqlx::query("UPDATE event_versions SET state='published',is_current_published=1,updated_at=datetime('now') WHERE id=?1")
        .bind(&version.0).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("working_publish_version", e))?;
    sqlx::query("UPDATE events SET current_published_version_id=?2,status='active',updated_at=datetime('now') WHERE id=?1")
        .bind(event_id).bind(&version.0).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("working_publish_event", e))?;
    sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,ip_address,details_json) VALUES(?1,?2,'event_version_published','event',?3,NULL,?4)")
        .bind(uuid::Uuid::new_v4().to_string()).bind(actor).bind(event_id)
        .bind(serde_json::json!({"versionId":version.0,"versionNumber":version.1}).to_string())
        .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("working_publish_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("working_publish_commit", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn export_for_event(
    event_id: &str,
) -> Result<(CustomEventManifest, String), ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    let m = load_manifest(crate::db::pool(), event_id).await?;
    let json = canonical_json(&m).map_err(crate::security::public_error)?;
    Ok((m, json))
}

#[cfg(feature = "server")]
async fn resolve_plan(m: &CustomEventManifest) -> Result<ResolvedPlan, ServerFnError> {
    crate::assets::ensure_manifest_assets(m).await?;
    resolve_plan_inner(m).await
}

#[cfg(feature = "server")]
async fn resolve_plan_inner(m: &CustomEventManifest) -> Result<ResolvedPlan, ServerFnError> {
    let db = crate::db::pool();
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT id,kind,status FROM events WHERE slug=?1")
            .bind(&m.slug)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("manifest_plan_lookup", e))?;
    let (item_count, option_count, link_count) = counts(m);
    let mf = fingerprint(m).map_err(crate::security::public_error)?;
    let Some((id, kind, _status)) = row else {
        return Ok(ResolvedPlan {
            preview: ManifestPreview {
                action: ImportAction::Create,
                name: m.name.clone(),
                slug: m.slug.clone(),
                schema_version: m.schema_version,
                item_count,
                option_count,
                link_count,
                manifest_fingerprint: mf,
                base_fingerprint: absent_fingerprint(&m.slug),
                safe_changes: Vec::new(),
                blocked_changes: Vec::new(),
            },
        });
    };
    if kind != "custom" {
        return Ok(ResolvedPlan {
            preview: ManifestPreview {
                action: ImportAction::Conflict,
                name: m.name.clone(),
                slug: m.slug.clone(),
                schema_version: m.schema_version,
                item_count,
                option_count,
                link_count,
                manifest_fingerprint: mf,
                base_fingerprint: absent_fingerprint(&m.slug),
                safe_changes: Vec::new(),
                blocked_changes: vec![ManifestDiffEntry {
                    category: "blocked".into(),
                    path: "Event.slug".into(),
                    change: "já pertence a outro tipo de evento".into(),
                }],
            },
        });
    }
    let current = load_manifest(db, &id).await?;
    let base = fingerprint(&current).map_err(crate::security::public_error)?;
    let mut changes = safe_diff(&current, m);
    let structural = structural_diff(&current, m);
    let blocked: Vec<_> = structural
        .iter()
        .filter(|entry| entry.path == "Event.slug")
        .cloned()
        .collect();
    for mut entry in structural {
        if entry.path != "Event.slug" {
            entry.category = "revision".into();
            changes.push(entry);
        }
    }
    let action = if current == *m {
        ImportAction::NoChange
    } else if !blocked.is_empty() {
        ImportAction::Conflict
    } else {
        ImportAction::SafeUpdate
    };
    Ok(ResolvedPlan {
        preview: ManifestPreview {
            action,
            name: m.name.clone(),
            slug: m.slug.clone(),
            schema_version: m.schema_version,
            item_count,
            option_count,
            link_count,
            manifest_fingerprint: mf,
            base_fingerprint: base,
            safe_changes: changes,
            blocked_changes: blocked,
        },
    })
}

#[cfg(feature = "server")]
pub async fn preview(bytes: &str) -> Result<ManifestPreview, ServerFnError> {
    let mut m = parse_and_validate(bytes).map_err(crate::security::public_error)?;
    m.schema_version = CURRENT_SCHEMA_VERSION;
    Ok(resolve_plan(&m).await?.preview)
}

#[cfg(feature = "server")]
pub(crate) async fn preview_manifest(
    m: &CustomEventManifest,
    available_assets: &std::collections::HashSet<String>,
) -> Result<ManifestPreview, ServerFnError> {
    crate::assets::ensure_manifest_assets_available(m, available_assets).await?;
    Ok(resolve_plan_without_asset_check(m).await?.preview)
}

#[cfg(feature = "server")]
async fn resolve_plan_without_asset_check(
    m: &CustomEventManifest,
) -> Result<ResolvedPlan, ServerFnError> {
    resolve_plan_inner(m).await
}

#[cfg(feature = "server")]
async fn insert_option(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    item_id: &str,
    option: &CustomEventManifestOption,
    sort: usize,
) -> Result<(), ServerFnError> {
    let id = uuid::Uuid::new_v4().to_string();
    let image_asset_id = if let Some(asset) = &option.image_asset {
        let found: Option<(String,)> = sqlx::query_as("SELECT id FROM assets WHERE sha256=?1")
            .bind(&asset.sha256)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| crate::security::internal_error("manifest_option_asset", e))?;
        Some(
            found
                .ok_or_else(|| crate::security::public_error("Asset referenciado não encontrado."))?
                .0,
        )
    } else {
        None
    };
    sqlx::query("INSERT INTO custom_question_options(id,item_id,external_key,label,sort_order,image_url,image_asset_id) VALUES(?1,?2,?3,?4,?5,?6,?7)")
        .bind(&id).bind(item_id).bind(&option.external_key).bind(&option.label).bind(sort as i64).bind(&option.image_url).bind(&image_asset_id)
        .execute(&mut **tx).await.map_err(|e| crate::security::internal_error("manifest_option",e))?;
    for (sort, link) in option.links.iter().enumerate() {
        sqlx::query("INSERT INTO option_links(id,option_id,kind,label,url,sort_order) VALUES(?1,?2,?3,?4,?5,?6)").bind(uuid::Uuid::new_v4().to_string()).bind(&id).bind(&link.kind).bind(&link.label).bind(&link.url).bind(sort as i64).execute(&mut **tx).await.map_err(|e| crate::security::internal_error("manifest_link",e))?;
    }
    Ok(())
}

#[cfg(feature = "server")]
async fn insert_items(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    event_id: &str,
    version_id: &str,
    m: &CustomEventManifest,
) -> Result<(), ServerFnError> {
    for (sort, item) in m.items.iter().enumerate() {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO prediction_items(id,event_id,event_version_id,external_key,kind,title,description,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'open')").bind(&id).bind(event_id).bind(version_id).bind(&item.external_key).bind(&item.kind).bind(&item.title).bind(&item.description).bind(&item.lock_at).bind(&item.reveal_at).bind(sort as i64).execute(&mut **tx).await.map_err(|e| crate::security::internal_error("manifest_item",e))?;
        match item.kind.as_str() {
            "single_choice" => {
                sqlx::query("INSERT INTO custom_questions(item_id,points) VALUES(?1,1)")
                    .bind(&id)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| crate::security::internal_error("manifest_question", e))?;
                for (s, o) in item.options.iter().enumerate() {
                    insert_option(tx, &id, o, s).await?;
                }
            }
            "multiple_choice" => {
                sqlx::query("INSERT INTO multiple_choice_questions(item_id,min_selections,max_selections) VALUES(?1,?2,?3)").bind(&id).bind(item.min_selections.unwrap_or(1)).bind(item.max_selections).execute(&mut **tx).await.map_err(|e| crate::security::internal_error("manifest_multiple_choice",e))?;
                for (s, o) in item.options.iter().enumerate() {
                    insert_option(tx, &id, o, s).await?;
                }
            }
            "numeric" => {
                let p = item.decimal_places.expect("validated") as u8;
                let min = item
                    .min_value
                    .as_ref()
                    .map(|v| crate::numeric::parse_scaled(v, p))
                    .transpose()
                    .map_err(crate::security::public_error)?;
                let max = item
                    .max_value
                    .as_ref()
                    .map(|v| crate::numeric::parse_scaled(v, p))
                    .transpose()
                    .map_err(crate::security::public_error)?;
                sqlx::query("INSERT INTO numeric_questions(item_id,decimal_places,unit_label,min_value_scaled,max_value_scaled) VALUES(?1,?2,?3,?4,?5)").bind(&id).bind(p).bind(&item.unit_label).bind(min).bind(max).execute(&mut **tx).await.map_err(|e| crate::security::internal_error("manifest_numeric",e))?;
            }
            _ => return Err(crate::security::public_error("tipo de item inválido")),
        }
    }
    Ok(())
}

#[cfg(feature = "server")]
async fn apply_manifest(
    m: &CustomEventManifest,
    expected: &str,
    actor: Option<&str>,
) -> Result<ManifestApplyResult, ServerFnError> {
    crate::assets::ensure_manifest_assets(m).await?;
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("manifest_apply_begin", e))?;
    let cover_asset_id: Option<String> = if let Some(asset) = &m.cover_asset {
        Some(
            sqlx::query_as::<_, (String,)>("SELECT id FROM assets WHERE sha256=?1")
                .bind(&asset.sha256)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| crate::security::internal_error("manifest_cover_asset", e))?
                .0,
        )
    } else {
        None
    };
    let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id,kind,status,current_published_version_id FROM events WHERE slug=?1",
    )
    .bind(&m.slug)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("manifest_apply_lookup", e))?;
    let current = if let Some((id, kind, _, _)) = &row {
        if kind != "custom" {
            return Err(crate::security::public_error(
                "slug já pertence a um evento não customizado",
            ));
        }
        Some(load_manifest_conn(&mut *tx, id, None).await?)
    } else {
        None
    };
    let actual = current
        .as_ref()
        .map(|v| fingerprint(v))
        .transpose()
        .map_err(crate::security::public_error)?
        .unwrap_or_else(|| absent_fingerprint(&m.slug));
    if actual != expected {
        return Err(crate::security::public_error(
            "O evento mudou desde o preview. Valide o manifesto novamente.",
        ));
    }
    let action = if current.as_ref().is_some_and(|old| old == m) {
        ImportAction::NoChange
    } else if row.is_some() {
        ImportAction::SafeUpdate
    } else {
        ImportAction::Create
    };
    let (event_id, version_id, state) = if let Some((id, _kind, _status, current_version_id)) = row
    {
        if action == ImportAction::NoChange {
            (Some(id), current_version_id, "published".to_string())
        } else {
            let working: Option<(String, i64, String)> = sqlx::query_as(
                "SELECT id,version_number,base_fingerprint FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1",
            )
            .bind(&id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("manifest_working_lookup", e))?;
            let (version_id, version_number) = if let Some((version_id, number, base_fingerprint)) =
                working
            {
                if current_version_id.is_some() && base_fingerprint != expected {
                    return Err(crate::security::public_error("Já existe uma revisão pendente baseada em outra versão. Revise ou publique essa revisão antes de importar novamente."));
                }
                sqlx::query("DELETE FROM prediction_items WHERE event_version_id=?1")
                    .bind(&version_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| crate::security::internal_error("manifest_working_replace", e))?;
                (version_id, number)
            } else {
                let next: (i64,) = sqlx::query_as("SELECT COALESCE(MAX(version_number),0)+1 FROM event_versions WHERE event_id=?1")
                    .bind(&id).fetch_one(&mut *tx).await
                    .map_err(|e| crate::security::internal_error("manifest_working_number", e))?;
                let version_id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,base_fingerprint,created_by) VALUES(?1,?2,?3,'working',0,?4,?5,?6,?7,?8,?9,?10,?11)")
                    .bind(&version_id).bind(&id).bind(next.0).bind(&m.name).bind(&m.description).bind(&m.cover_url).bind(&cover_asset_id).bind(&m.external_url).bind(fingerprint(m).map_err(crate::security::public_error)?).bind(expected).bind(actor)
                    .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("manifest_working_create", e))?;
                (version_id, next.0)
            };
            sqlx::query("UPDATE event_versions SET name=?2,description=?3,cover_url=?4,cover_asset_id=?5,external_url=?6,fingerprint=?7,base_fingerprint=?8,updated_at=datetime('now') WHERE id=?1")
                .bind(&version_id).bind(&m.name).bind(&m.description).bind(&m.cover_url).bind(&cover_asset_id).bind(&m.external_url).bind(fingerprint(m).map_err(crate::security::public_error)?).bind(expected)
                .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("manifest_working_metadata", e))?;
            insert_items(&mut tx, &id, &version_id, m).await?;
            let _ = version_number;
            (Some(id), Some(version_id), "working".to_string())
        }
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        let version_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by,starts_at,ends_at,description,cover_url,external_url,cover_asset_id,pool_creation_enabled) VALUES(?1,?2,?3,'custom','draft',?4,?5,?6,?7,?8,?9,?10,1)")
            .bind(&id).bind(&m.name).bind(&m.slug).bind(actor).bind(&m.starts_at).bind(&m.ends_at).bind(&m.description).bind(&m.cover_url).bind(&m.external_url).bind(&cover_asset_id)
            .execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_create_event",e))?;
        sqlx::query("INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,base_fingerprint,created_by) VALUES(?1,?2,1,'working',0,?3,?4,?5,?6,?7,?8,?9,?10)")
            .bind(&version_id).bind(&id).bind(&m.name).bind(&m.description).bind(&m.cover_url).bind(&cover_asset_id).bind(&m.external_url).bind(fingerprint(m).map_err(crate::security::public_error)?).bind(expected).bind(actor)
            .execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_create_version",e))?;
        insert_items(&mut tx, &id, &version_id, m).await?;
        (Some(id), Some(version_id), "working".to_string())
    };
    let (i, o, l) = counts(m);
    sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,ip_address,details_json) VALUES(?1,?2,?3,?4,?5,?6,?7)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(actor)
        .bind("event_manifest_imported")
        .bind("event")
        .bind(event_id.as_deref())
        .bind(Option::<&str>::None)
        .bind(serde_json::json!({"schemaVersion":m.schema_version,"action":format!("{:?}",action),"manifestFingerprint":fingerprint(m).unwrap_or_default(),"versionId":version_id,"state":state,"itemCount":i,"optionCount":o,"linkCount":l}).to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("manifest_apply_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("manifest_apply_commit", e))?;
    Ok(ManifestApplyResult {
        action,
        event_id,
        item_count: i,
        option_count: o,
        link_count: l,
        version_id,
        state,
    })
}

#[cfg(feature = "server")]
pub async fn apply_admin(
    bytes: &str,
    expected: &str,
    actor: &str,
) -> Result<ManifestApplyResult, ServerFnError> {
    let mut m = parse_and_validate(bytes).map_err(crate::security::public_error)?;
    m.schema_version = CURRENT_SCHEMA_VERSION;
    apply_manifest(&m, expected, Some(actor)).await
}

#[cfg(feature = "server")]
pub(crate) async fn apply_normalized(
    m: &CustomEventManifest,
    expected: &str,
    actor: &str,
) -> Result<ManifestApplyResult, ServerFnError> {
    apply_manifest(m, expected, Some(actor)).await
}

/// Compatibility wrapper used by the legacy CLI. Importing is deliberately
/// revision-only now; a separate publish operation is required.
#[cfg(feature = "server")]
pub async fn import(m: CustomEventManifest, apply: bool) -> Result<(usize, usize), ServerFnError> {
    let mut m = m;
    m.schema_version = CURRENT_SCHEMA_VERSION;
    let (i, o, _) = counts(&m);
    if !apply {
        return Ok((i, o));
    }
    let db = crate::db::pool();
    let actor: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE is_admin=1 ORDER BY created_at LIMIT 1")
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_actor", e))?;
    let expected = if let Some((id,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM events WHERE slug=?1")
            .bind(&m.slug)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_lookup", e))?
    {
        fingerprint(&load_manifest(db, &id).await?).map_err(crate::security::public_error)?
    } else {
        absent_fingerprint(&m.slug)
    };
    let _ = apply_manifest(&m, &expected, actor.as_ref().map(|value| value.0.as_str())).await?;
    Ok((i, o))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> &'static str {
        r#"{"schemaVersion":1,"name":"Premiação Teste","slug":"premiacao-teste","kind":"custom","items":[{"externalKey":"melhor-filme","kind":"single_choice","title":"Melhor Filme","lockAt":"2026-09-27T19:30:00-04:00","revealAt":"2026-09-27T19:30:00-04:00","options":[{"externalKey":"a","label":"A"},{"externalKey":"b","label":"B"}]}]}"#
    }
    #[test]
    fn accepts_legacy_and_versioned() {
        assert_eq!(parse_and_validate(sample()).unwrap().schema_version, 1);
        assert_eq!(
            parse_and_validate(&sample().replace("\"schemaVersion\":1,", ""))
                .unwrap()
                .schema_version,
            1
        );
    }
    #[test]
    fn rejects_unknown_version_and_duplicates() {
        assert_eq!(
            parse_and_validate(&sample().replace("\"schemaVersion\":1", "\"schemaVersion\":2"))
                .unwrap()
                .schema_version,
            2
        );
        assert!(parse_and_validate(
            &sample().replace("\"schemaVersion\":1", "\"schemaVersion\":99")
        )
        .is_err());
        assert!(parse_and_validate(
            &sample().replace("\"externalKey\":\"b\"", "\"externalKey\":\"a\"")
        )
        .is_err());
    }
    #[test]
    fn canonical_fingerprint_ignores_json_formatting() {
        let a = parse_and_validate(sample()).unwrap();
        let b = sample().replace(
            "\"name\":\"Premiação Teste\"",
            "\"name\": \" Premiação Teste \"",
        );
        let b = parse_and_validate(&b).unwrap();
        assert_eq!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
        let reordered = r#"{"items":[{"options":[{"label":"A","externalKey":"a"},{"label":"B","externalKey":"b"}],"revealAt":"2026-09-27T19:30:00-04:00","lockAt":"2026-09-27T19:30:00-04:00","title":"Melhor Filme","kind":"single_choice","externalKey":"melhor-filme"}],"kind":"custom","slug":"premiacao-teste","name":"Premiação Teste","schemaVersion":1}"#;
        assert_eq!(
            fingerprint(&a).unwrap(),
            fingerprint(&parse_and_validate(reordered).unwrap()).unwrap()
        );
        let canonical = canonical_json(&a).unwrap();
        assert!(canonical.ends_with('\n'));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&canonical).unwrap()["schemaVersion"],
            2
        );
    }

    #[test]
    fn asset_references_are_v2_only_and_part_of_canonical_fingerprint() {
        let hash = "a".repeat(64);
        let v1 = sample().replace(
            "\"label\":\"A\"",
            &format!("\"label\":\"A\",\"imageAsset\":{{\"kind\":\"asset\",\"sha256\":\"{hash}\",\"mediaType\":\"image/webp\"}}"),
        );
        assert!(parse_and_validate(&v1).is_err());
        let v2 = v1.replace("\"schemaVersion\":1", "\"schemaVersion\":2");
        let parsed = parse_and_validate(&v2).expect("asset ref v2");
        assert_eq!(
            parsed.items[0].options[0]
                .image_asset
                .as_ref()
                .unwrap()
                .sha256,
            hash
        );
        assert_eq!(
            canonical_json(&parsed)
                .unwrap()
                .matches("imageAsset")
                .count(),
            1
        );
    }
    #[test]
    fn rejects_reversed_windows() {
        assert!(validate_event_window(
            &Some("2026-10-02T00:00:00Z".into()),
            &Some("2026-10-01T00:00:00Z".into())
        )
        .is_err());
        assert!(validate_single_choice_timing(
            "Categoria",
            "key",
            "2026-10-02T00:00:00Z",
            "2026-10-01T00:00:00Z"
        )
        .is_err());
    }

    #[test]
    fn published_name_is_safe_but_option_label_is_structural() {
        let current = parse_and_validate(sample()).unwrap();
        let renamed =
            parse_and_validate(&sample().replace("Premiação Teste", "Premiação Teste 2026"))
                .unwrap();
        assert!(projection(&current) == projection(&renamed));
        assert!(safe_diff(&current, &renamed)
            .iter()
            .any(|change| change.path == "Event.name"));

        let changed_label =
            parse_and_validate(&sample().replace("\"label\":\"A\"", "\"label\":\"Outra opção\""))
                .unwrap();
        assert!(structural_diff(&current, &changed_label)
            .iter()
            .any(|change| change.path.contains("label")));

        let mut removed_option = current.clone();
        removed_option.items[0].options.pop();
        assert!(structural_diff(&current, &removed_option)
            .iter()
            .any(|change| change.path.contains("Option 'b'") && change.change == "removida"));
    }
}
