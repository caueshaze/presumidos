use super::*;
use crate::error::ServerFnError;
#[cfg(feature = "server")]
use sqlx::{sqlite::SqliteConnection, Sqlite, SqlitePool};

pub async fn export_for_event(
    event_id: &str,
) -> Result<(CustomEventManifest, String), ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    let m = load_manifest(crate::db::pool(), event_id).await?;
    let json = canonical_json(&m).map_err(crate::security::public_error)?;
    Ok((m, json))
}

#[cfg(feature = "server")]
pub(crate) async fn resolve_plan(m: &CustomEventManifest) -> Result<ResolvedPlan, ServerFnError> {
    crate::assets::ensure_manifest_assets(m).await?;
    resolve_plan_inner(m).await
}

#[cfg(feature = "server")]
pub(crate) async fn resolve_plan_inner(
    m: &CustomEventManifest,
) -> Result<ResolvedPlan, ServerFnError> {
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
