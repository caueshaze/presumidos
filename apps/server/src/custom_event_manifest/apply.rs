use super::*;
use crate::error::ServerFnError;
#[cfg(feature = "server")]
use sqlx::Sqlite;

pub(crate) async fn resolve_plan_without_asset_check(
    m: &CustomEventManifest,
) -> Result<ResolvedPlan, ServerFnError> {
    resolve_plan_inner(m).await
}

#[cfg(feature = "server")]
pub(crate) async fn insert_option(
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
pub(crate) async fn insert_items(
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
pub(crate) async fn apply_manifest(
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
