use crate::error::ServerFnError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const CURRENT_SCHEMA_VERSION: u32 = 2;
pub const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_ITEMS: usize = 100;
pub const MAX_OPTIONS_PER_ITEM: usize = 100;
pub const MAX_LINKS_PER_OPTION: usize = 20;

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetReference {
    pub kind: String,
    pub sha256: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub description: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub cover_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_asset: Option<AssetReference>,
    pub external_url: Option<String>,
    pub items: Vec<CustomEventManifestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifestItem {
    pub external_key: String,
    pub kind: String,
    pub title: String,
    pub description: Option<String>,
    pub lock_at: String,
    pub reveal_at: String,
    #[serde(default)]
    pub options: Vec<CustomEventManifestOption>,
    pub decimal_places: Option<i64>,
    pub unit_label: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub min_selections: Option<i64>,
    pub max_selections: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifestOption {
    pub external_key: String,
    pub label: String,
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_asset: Option<AssetReference>,
    #[serde(default)]
    pub links: Vec<CustomEventManifestOptionLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifestOptionLink {
    pub kind: String,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestDiffEntry {
    pub category: String,
    pub path: String,
    pub change: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportAction {
    Create,
    NoChange,
    SafeUpdate,
    Conflict,
    Rejected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestPreview {
    pub action: ImportAction,
    pub name: String,
    pub slug: String,
    pub schema_version: u32,
    pub item_count: usize,
    pub option_count: usize,
    pub link_count: usize,
    pub manifest_fingerprint: String,
    pub base_fingerprint: String,
    pub safe_changes: Vec<ManifestDiffEntry>,
    pub blocked_changes: Vec<ManifestDiffEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestApplyResult {
    pub action: ImportAction,
    pub event_id: Option<String>,
    pub item_count: usize,
    pub option_count: usize,
    pub link_count: usize,
}

#[derive(Debug, Clone)]
struct ResolvedPlan {
    preview: ManifestPreview,
}

pub fn validate_optional_http_url(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>, String> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 2048 || !(value.starts_with("https://") || value.starts_with("http://")) {
        return Err(format!("{field} deve usar http ou https"));
    }
    let host = value
        .split_once("://")
        .map(|(_, rest)| rest.split('/').next().unwrap_or_default())
        .unwrap_or_default();
    if host.is_empty() {
        return Err(format!("{field} inválida"));
    }
    Ok(Some(value))
}

pub fn validate_event_window(
    starts_at: &Option<String>,
    ends_at: &Option<String>,
) -> Result<(), String> {
    if let Some(value) = starts_at {
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| "startsAt inválido")?;
    }
    if let Some(value) = ends_at {
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| "endsAt inválido")?;
    }
    if let (Some(a), Some(b)) = (starts_at, ends_at) {
        if chrono::DateTime::parse_from_rfc3339(a).map_err(|_| "startsAt inválido")?
            >= chrono::DateTime::parse_from_rfc3339(b).map_err(|_| "endsAt inválido")?
        {
            return Err("startsAt deve preceder endsAt".into());
        }
    }
    Ok(())
}

pub fn validate_single_choice_timing(
    title: &str,
    external_key: &str,
    lock_at: &str,
    reveal_at: &str,
) -> Result<(), String> {
    if title.trim().is_empty() || external_key.trim().is_empty() {
        return Err("item inválido".into());
    }
    let lock = chrono::DateTime::parse_from_rfc3339(lock_at).map_err(|_| "lockAt inválido")?;
    let reveal =
        chrono::DateTime::parse_from_rfc3339(reveal_at).map_err(|_| "revealAt inválido")?;
    if lock > reveal {
        return Err("lockAt deve preceder ou igualar revealAt".into());
    }
    Ok(())
}

fn optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn normalize(mut m: CustomEventManifest) -> CustomEventManifest {
    if m.schema_version == 0 {
        m.schema_version = CURRENT_SCHEMA_VERSION;
    }
    m.name = m.name.trim().to_string();
    m.slug = m.slug.trim().to_string();
    m.kind = m.kind.trim().to_string();
    m.description = optional(m.description);
    m.starts_at = optional(m.starts_at);
    m.ends_at = optional(m.ends_at);
    m.cover_url = optional(m.cover_url);
    m.external_url = optional(m.external_url);
    if let Some(asset) = &mut m.cover_asset {
        asset.kind = asset.kind.trim().to_string();
        asset.sha256 = asset.sha256.trim().to_lowercase();
        asset.media_type = asset.media_type.trim().to_lowercase();
    }
    for item in &mut m.items {
        item.external_key = item.external_key.trim().to_string();
        item.kind = item.kind.trim().to_string();
        item.title = item.title.trim().to_string();
        item.description = optional(item.description.take());
        item.lock_at = item.lock_at.trim().to_string();
        item.reveal_at = item.reveal_at.trim().to_string();
        item.unit_label = optional(item.unit_label.take());
        item.min_value = optional(item.min_value.take());
        item.max_value = optional(item.max_value.take());
        for option in &mut item.options {
            option.external_key = option.external_key.trim().to_string();
            option.label = option.label.trim().to_string();
            option.image_url = optional(option.image_url.take());
            if let Some(asset) = &mut option.image_asset {
                asset.kind = asset.kind.trim().to_string();
                asset.sha256 = asset.sha256.trim().to_lowercase();
                asset.media_type = asset.media_type.trim().to_lowercase();
            }
            for link in &mut option.links {
                link.kind = link.kind.trim().to_string();
                link.label = link.label.trim().to_string();
                link.url = link.url.trim().to_string();
            }
        }
    }
    m
}

pub fn parse_and_validate(bytes: &str) -> Result<CustomEventManifest, String> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(format!(
            "manifesto excede o limite de {} bytes",
            MAX_MANIFEST_BYTES
        ));
    }
    let mut m: CustomEventManifest =
        serde_json::from_str(bytes).map_err(|e| format!("JSON inválido: {e}"))?;
    m = normalize(m);
    if m.schema_version > CURRENT_SCHEMA_VERSION {
        return Err("Este manifesto usa uma versão ainda não suportada.".into());
    }
    if !matches!(m.schema_version, 1 | CURRENT_SCHEMA_VERSION) {
        return Err("Versão de manifesto inválida.".into());
    }
    if m.schema_version == 1
        && (m.cover_asset.is_some()
            || m.items.iter().any(|item| {
                item.options
                    .iter()
                    .any(|option| option.image_asset.is_some())
            }))
    {
        return Err("Asset refs exigem schemaVersion 2.".into());
    }
    if m.kind != "custom" || m.name.is_empty() || m.slug.is_empty() || m.items.is_empty() {
        return Err("manifesto custom inválido".into());
    }
    if m.name.len() > 120 || m.slug.len() > 120 {
        return Err("name ou slug muito longo".into());
    }
    if m.description.as_ref().is_some_and(|v| v.len() > 1200) {
        return Err("description muito longa".into());
    }
    if m.items.len() > MAX_ITEMS {
        return Err(format!("manifesto excede o limite de {MAX_ITEMS} itens"));
    }
    m.cover_url = validate_optional_http_url(m.cover_url, "coverUrl")?;
    m.external_url = validate_optional_http_url(m.external_url, "externalUrl")?;
    if let Some(asset) = &mut m.cover_asset {
        validate_asset_reference(asset, "coverAsset")?;
    }
    let mut item_keys = HashSet::new();
    for (ii, item) in m.items.iter_mut().enumerate() {
        if !matches!(
            item.kind.as_str(),
            "single_choice" | "numeric" | "multiple_choice"
        ) || item.external_key.is_empty()
            || item.external_key.len() > 120
            || item.title.is_empty()
            || item.title.len() > 240
            || !item_keys.insert(item.external_key.clone())
        {
            return Err(format!("items[{ii}] inválido ou externalKey duplicada"));
        }
        validate_single_choice_timing(
            &item.title,
            &item.external_key,
            &item.lock_at,
            &item.reveal_at,
        )?;
        if item.description.as_ref().is_some_and(|v| v.len() > 1200) {
            return Err(format!("items[{ii}].description muito longa"));
        }
        if item.kind == "single_choice" || item.kind == "multiple_choice" {
            if item.options.len() < 2 {
                return Err(format!(
                    "item '{}' precisa de pelo menos 2 options",
                    item.external_key
                ));
            }
            if item.options.len() > MAX_OPTIONS_PER_ITEM {
                return Err(format!(
                    "item '{}' excede o limite de options",
                    item.external_key
                ));
            }
            let mut option_keys = HashSet::new();
            for (oi, option) in item.options.iter_mut().enumerate() {
                if option.external_key.is_empty()
                    || option.external_key.len() > 120
                    || option.label.is_empty()
                    || option.label.len() > 240
                    || !option_keys.insert(option.external_key.clone())
                {
                    return Err(format!(
                        "items[{ii}].options[{oi}] inválida ou externalKey duplicada"
                    ));
                }
                option.image_url = validate_optional_http_url(
                    option.image_url.take(),
                    &format!("items[{ii}].options[{oi}].imageUrl"),
                )?;
                if let Some(asset) = &mut option.image_asset {
                    validate_asset_reference(
                        asset,
                        &format!("items[{ii}].options[{oi}].imageAsset"),
                    )?;
                }
                if option.links.len() > MAX_LINKS_PER_OPTION {
                    return Err(format!(
                        "items[{ii}].options[{oi}] excede o limite de links"
                    ));
                }
                for (li, link) in option.links.iter_mut().enumerate() {
                    if !matches!(link.kind.as_str(), "video" | "audio" | "official" | "other")
                        || link.label.is_empty()
                        || link.label.len() > 80
                    {
                        return Err(format!("items[{ii}].options[{oi}].links[{li}] inválido"));
                    }
                    link.url = validate_optional_http_url(
                        Some(link.url.clone()),
                        &format!("items[{ii}].options[{oi}].links[{li}].url"),
                    )?
                    .expect("URL validada não pode ser vazia");
                }
            }
            if item.kind == "multiple_choice" {
                let min = item.min_selections.unwrap_or(1);
                let max = item.max_selections.unwrap_or(item.options.len() as i64);
                if min < 1 || max < min || max > item.options.len() as i64 {
                    return Err(format!("item '{}' min/max inválido", item.external_key));
                }
                if item.decimal_places.is_some()
                    || item.unit_label.is_some()
                    || item.min_value.is_some()
                    || item.max_value.is_some()
                {
                    return Err(format!(
                        "item '{}' multiple_choice não aceita campos numeric",
                        item.external_key
                    ));
                }
            }
        } else {
            if !item.options.is_empty() {
                return Err(format!(
                    "item '{}' numeric não aceita options",
                    item.external_key
                ));
            }
            let places =
                crate::numeric::validate_question(item.decimal_places.unwrap_or(-1), None, None)?;
            let min = item
                .min_value
                .as_ref()
                .map(|v| crate::numeric::parse_scaled(v, places))
                .transpose()?;
            let max = item
                .max_value
                .as_ref()
                .map(|v| crate::numeric::parse_scaled(v, places))
                .transpose()?;
            crate::numeric::validate_question(places as i64, min, max)?;
            item.min_value = min.map(|v| crate::numeric::display_scaled(v, places));
            item.max_value = max.map(|v| crate::numeric::display_scaled(v, places));
            if item.unit_label.as_ref().is_some_and(|v| v.len() > 60) {
                return Err(format!(
                    "item '{}' unitLabel muito longo",
                    item.external_key
                ));
            }
        }
    }
    validate_event_window(&m.starts_at, &m.ends_at)?;
    Ok(m)
}

fn validate_asset_reference(asset: &AssetReference, field: &str) -> Result<(), String> {
    if asset.kind != "asset"
        || asset.media_type != "image/webp"
        || asset.sha256.len() != 64
        || !asset.sha256.chars().all(|value| value.is_ascii_hexdigit())
    {
        return Err(format!("{field} inválido"));
    }
    Ok(())
}

pub fn canonical_json(m: &CustomEventManifest) -> Result<String, String> {
    let mut normalized = parse_and_validate(
        &serde_json::to_string(m).map_err(|e| format!("falha ao canonicalizar: {e}"))?,
    )?;
    normalized.schema_version = CURRENT_SCHEMA_VERSION;
    let mut json = serde_json::to_string_pretty(&normalized)
        .map_err(|e| format!("falha ao serializar manifesto: {e}"))?;
    json.push('\n');
    Ok(json)
}

pub fn fingerprint(m: &CustomEventManifest) -> Result<String, String> {
    Ok(hex::encode(Sha256::digest(canonical_json(m)?.as_bytes())))
}

fn counts(m: &CustomEventManifest) -> (usize, usize, usize) {
    let options = m.items.iter().map(|i| i.options.len()).sum();
    let links = m
        .items
        .iter()
        .flat_map(|i| i.options.iter())
        .map(|o| o.links.len())
        .sum();
    (m.items.len(), options, links)
}
fn absent_fingerprint(slug: &str) -> String {
    hex::encode(Sha256::digest(format!("absent:{slug}").as_bytes()))
}
fn projection(m: &CustomEventManifest) -> CustomEventManifest {
    let mut v = m.clone();
    v.name.clear();
    v.description = None;
    v.cover_url = None;
    v.external_url = None;
    v.cover_asset = None;
    for i in &mut v.items {
        for o in &mut i.options {
            o.image_url = None;
            o.image_asset = None;
            o.links.clear();
        }
    }
    v
}
fn change_kind<T>(old: &Option<T>, new: &Option<T>) -> String {
    match (old.is_some(), new.is_some()) {
        (false, true) => "adicionado".into(),
        (true, false) => "removido".into(),
        _ => "alterado".into(),
    }
}
fn push(v: &mut Vec<ManifestDiffEntry>, category: &str, path: String, change: impl Into<String>) {
    v.push(ManifestDiffEntry {
        category: category.into(),
        path,
        change: change.into(),
    });
}

fn safe_diff(a: &CustomEventManifest, b: &CustomEventManifest) -> Vec<ManifestDiffEntry> {
    let mut v = Vec::new();
    if a.name != b.name {
        push(&mut v, "safe", "Event.name".into(), "alterado");
    }
    if a.description != b.description {
        push(
            &mut v,
            "safe",
            "Event.description".into(),
            change_kind(&a.description, &b.description),
        );
    }
    if a.cover_url != b.cover_url {
        push(
            &mut v,
            "safe",
            "Event.coverUrl".into(),
            change_kind(&a.cover_url, &b.cover_url),
        );
    }
    if a.cover_asset != b.cover_asset {
        push(
            &mut v,
            "safe",
            "Event.coverAsset".into(),
            change_kind(&a.cover_asset, &b.cover_asset),
        );
    }
    if a.external_url != b.external_url {
        push(
            &mut v,
            "safe",
            "Event.externalUrl".into(),
            change_kind(&a.external_url, &b.external_url),
        );
    }
    for item in &b.items {
        let Some(old_item) = a.items.iter().find(|i| i.external_key == item.external_key) else {
            continue;
        };
        for option in &item.options {
            let Some(old) = old_item
                .options
                .iter()
                .find(|o| o.external_key == option.external_key)
            else {
                continue;
            };
            let prefix = format!("Option '{}'", option.external_key);
            if old.image_url != option.image_url {
                push(
                    &mut v,
                    "safe",
                    format!("{prefix}.imageUrl"),
                    change_kind(&old.image_url, &option.image_url),
                );
            }
            if old.image_asset != option.image_asset {
                push(
                    &mut v,
                    "safe",
                    format!("{prefix}.imageAsset"),
                    change_kind(&old.image_asset, &option.image_asset),
                );
            }
            if old.links != option.links {
                let delta = option.links.len() as i64 - old.links.len() as i64;
                let text = if delta > 0 {
                    format!("+{delta}")
                } else if delta < 0 {
                    delta.to_string()
                } else {
                    "alterado".into()
                };
                push(&mut v, "safe", format!("{prefix}.links"), text);
            }
        }
    }
    v
}

fn structural_diff(a: &CustomEventManifest, b: &CustomEventManifest) -> Vec<ManifestDiffEntry> {
    let mut v = Vec::new();
    if a.slug != b.slug {
        push(&mut v, "blocked", "Event.slug".into(), "alterado");
    }
    if a.starts_at != b.starts_at {
        push(&mut v, "blocked", "Event.startsAt".into(), "alterado");
    }
    if a.ends_at != b.ends_at {
        push(&mut v, "blocked", "Event.endsAt".into(), "alterado");
    }
    if a.items.len() != b.items.len() {
        push(
            &mut v,
            "blocked",
            "Event.items".into(),
            "quantidade alterada",
        );
    }
    for (index, item) in b.items.iter().enumerate() {
        let Some(old) = a.items.iter().find(|i| i.external_key == item.external_key) else {
            push(
                &mut v,
                "blocked",
                format!("Item '{}'", item.external_key),
                "adicionado",
            );
            continue;
        };
        let path = format!("Item '{}'", item.external_key);
        if old.kind != item.kind {
            push(&mut v, "blocked", format!("{path}.kind"), "alterado");
        }
        if old.title != item.title {
            push(&mut v, "blocked", format!("{path}.title"), "alterado");
        }
        if old.description != item.description {
            push(&mut v, "blocked", format!("{path}.description"), "alterado");
        }
        if old.lock_at != item.lock_at {
            push(&mut v, "blocked", format!("{path}.lockAt"), "alterado");
        }
        if old.reveal_at != item.reveal_at {
            push(&mut v, "blocked", format!("{path}.revealAt"), "alterado");
        }
        if old.decimal_places != item.decimal_places
            || old.unit_label != item.unit_label
            || old.min_value != item.min_value
            || old.max_value != item.max_value
        {
            push(
                &mut v,
                "blocked",
                format!("{path}.numericConfig"),
                "alterado",
            );
        }
        if old.min_selections != item.min_selections || old.max_selections != item.max_selections {
            push(
                &mut v,
                "blocked",
                format!("{path}.selectionRules"),
                "alterado",
            );
        }
        if old.options.len() != item.options.len() {
            push(
                &mut v,
                "blocked",
                format!("{path}.options"),
                "quantidade alterada",
            );
        }
        for option in &item.options {
            if let Some(old_option) = old
                .options
                .iter()
                .find(|o| o.external_key == option.external_key)
            {
                if old_option.label != option.label {
                    push(
                        &mut v,
                        "blocked",
                        format!("{path}.Option '{}'.label", option.external_key),
                        "alterado",
                    );
                }
            } else {
                push(
                    &mut v,
                    "blocked",
                    format!("{path}.Option '{}'", option.external_key),
                    "adicionada",
                );
            }
        }
        if a.items.get(index).map(|i| &i.external_key) != Some(&item.external_key) {
            push(&mut v, "blocked", format!("{path}.order"), "alterada");
        }
    }
    for old_item in &a.items {
        if !b
            .items
            .iter()
            .any(|item| item.external_key == old_item.external_key)
        {
            push(
                &mut v,
                "blocked",
                format!("Item '{}'", old_item.external_key),
                "removido",
            );
        }
    }
    for old_item in &a.items {
        let Some(new_item) = b
            .items
            .iter()
            .find(|item| item.external_key == old_item.external_key)
        else {
            continue;
        };
        for old_option in &old_item.options {
            if !new_item
                .options
                .iter()
                .any(|option| option.external_key == old_option.external_key)
            {
                push(
                    &mut v,
                    "blocked",
                    format!(
                        "Item '{}'.Option '{}'",
                        old_item.external_key, old_option.external_key
                    ),
                    "removida",
                );
            }
        }
    }
    v
}

#[cfg(feature = "server")]
use sqlx::{sqlite::SqliteConnection, Sqlite, SqlitePool};

#[cfg(feature = "server")]
async fn load_manifest_conn(
    conn: &mut SqliteConnection,
    event_id: &str,
) -> Result<CustomEventManifest, ServerFnError> {
    let event: Option<(String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>)> = sqlx::query_as("SELECT e.name,e.slug,e.kind,e.description,e.starts_at,e.ends_at,e.cover_url,e.external_url,e.cover_asset_id,a.sha256,a.media_type FROM events e LEFT JOIN assets a ON a.id=e.cover_asset_id WHERE e.id=?1").bind(event_id).fetch_optional(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_event", e))?;
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
    let rows: Vec<(String,String,String,Option<String>,String,String,i64,Option<i64>,Option<String>,Option<i64>,Option<i64>,Option<i64>,Option<i64>)> = sqlx::query_as("SELECT pi.external_key,pi.kind,pi.title,pi.description,pi.lock_at,pi.reveal_at,pi.sort_order,n.decimal_places,n.unit_label,n.min_value_scaled,n.max_value_scaled,mq.min_selections,mq.max_selections FROM prediction_items pi LEFT JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_id=?1 ORDER BY pi.sort_order,pi.id").bind(event_id).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_items", e))?;
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
        let options_rows: Vec<(String,String,i64,Option<String>,Option<String>,Option<String>)> = sqlx::query_as("SELECT o.external_key,o.label,o.sort_order,o.image_url,a.sha256,a.media_type FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id LEFT JOIN assets a ON a.id=o.image_asset_id WHERE pi.event_id=?1 AND pi.external_key=?2 ORDER BY o.sort_order,o.id").bind(event_id).bind(&external_key).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_options", e))?;
        let mut options = Vec::new();
        for (option_key, label, _sort, image_url, image_sha256, image_media_type) in options_rows {
            let links: Vec<(String,String,String,i64)> = sqlx::query_as("SELECT l.kind,l.label,l.url,l.sort_order FROM option_links l JOIN custom_question_options o ON o.id=l.option_id JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?1 AND pi.external_key=?2 AND o.external_key=?3 ORDER BY l.sort_order,l.id").bind(event_id).bind(&external_key).bind(&option_key).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_links", e))?;
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
    load_manifest_conn(&mut conn, event_id).await
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
    let Some((id, kind, status)) = row else {
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
    let used: (i64,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pools WHERE event_id=?1) OR EXISTS(SELECT 1 FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)",
    )
    .bind(&id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("manifest_plan_usage", e))?;
    let base = fingerprint(&current).map_err(crate::security::public_error)?;
    let safe = safe_diff(&current, m);
    let blocked = structural_diff(&current, m);
    let action = if current == *m {
        ImportAction::NoChange
    } else if status == "draft" && used.0 == 0 {
        ImportAction::SafeUpdate
    } else if status == "draft" && projection(&current) == projection(m) {
        ImportAction::SafeUpdate
    } else if projection(&current) != projection(m) {
        ImportAction::Conflict
    } else {
        ImportAction::SafeUpdate
    };
    let mut blocked_changes = if action == ImportAction::Conflict {
        blocked
    } else {
        Vec::new()
    };
    if status == "draft" && used.0 != 0 && current != *m {
        push(
            &mut blocked_changes,
            "blocked",
            "Event.usage".into(),
            "draft já possui Pools ou Predictions",
        );
    }
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
            safe_changes: safe,
            blocked_changes,
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
    m: &CustomEventManifest,
) -> Result<(), ServerFnError> {
    for (sort, item) in m.items.iter().enumerate() {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO prediction_items(id,event_id,external_key,kind,title,description,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'open')").bind(&id).bind(event_id).bind(&item.external_key).bind(&item.kind).bind(&item.title).bind(&item.description).bind(&item.lock_at).bind(&item.reveal_at).bind(sort as i64).execute(&mut **tx).await.map_err(|e| crate::security::internal_error("manifest_item",e))?;
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
    actor: &str,
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
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT id,kind,status FROM events WHERE slug=?1")
            .bind(&m.slug)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("manifest_apply_lookup", e))?;
    let current = if let Some((id, kind, _)) = &row {
        if kind != "custom" {
            return Err(crate::security::public_error(
                "slug já pertence a um evento não customizado",
            ));
        }
        Some(load_manifest_conn(&mut *tx, id).await?)
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
    let action = match (&row, &current) {
        (None, None) => ImportAction::Create,
        (Some((_id, _kind, _)), Some(old)) if *old == *m => ImportAction::NoChange,
        (Some((id, _kind, status)), Some(_)) if status == "draft" => {
            let used: (i64,) = sqlx::query_as(
                "SELECT EXISTS(SELECT 1 FROM pools WHERE event_id=?1) OR EXISTS(SELECT 1 FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("manifest_apply_usage", e))?;
            if used.0 == 0
                || current
                    .as_ref()
                    .is_some_and(|old| projection(old) == projection(m))
            {
                ImportAction::SafeUpdate
            } else {
                ImportAction::Conflict
            }
        }
        (Some((_id, _kind, _)), Some(old)) if projection(old) != projection(m) => {
            ImportAction::Conflict
        }
        (Some((_id, _kind, _)), Some(_)) => ImportAction::SafeUpdate,
        _ => ImportAction::Rejected,
    };
    if matches!(action, ImportAction::Conflict | ImportAction::Rejected) {
        return Err(crate::security::public_error(
            "O manifesto contém alterações estruturais bloqueadas.",
        ));
    }
    let event_id = if let Some((id, _kind, status)) = row {
        if action == ImportAction::NoChange {
            Some(id)
        } else {
            if status == "draft"
                && current
                    .as_ref()
                    .is_some_and(|old| projection(old) != projection(m))
            {
                sqlx::query("UPDATE events SET name=?2,slug=?3,starts_at=?4,ends_at=?5,description=?6,cover_url=?7,external_url=?8,cover_asset_id=?9,updated_at=datetime('now') WHERE id=?1").bind(&id).bind(&m.name).bind(&m.slug).bind(&m.starts_at).bind(&m.ends_at).bind(&m.description).bind(&m.cover_url).bind(&m.external_url).bind(&cover_asset_id).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_draft_metadata",e))?;
                sqlx::query("DELETE FROM prediction_items WHERE event_id=?1")
                    .bind(&id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| crate::security::internal_error("manifest_draft_items", e))?;
                insert_items(&mut tx, &id, m).await?;
            } else {
                sqlx::query("UPDATE events SET name=?2,description=?3,cover_url=?4,external_url=?5,cover_asset_id=?6,updated_at=datetime('now') WHERE id=?1").bind(&id).bind(&m.name).bind(&m.description).bind(&m.cover_url).bind(&m.external_url).bind(&cover_asset_id).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_editorial_metadata",e))?;
                for item in &m.items {
                    for option in &item.options {
                        let found:Option<(String,)>=sqlx::query_as("SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?1 AND pi.external_key=?2 AND o.external_key=?3").bind(&id).bind(&item.external_key).bind(&option.external_key).fetch_optional(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_editorial_option",e))?;
                        let Some((oid,)) = found else {
                            return Err(crate::security::public_error(
                                "option não encontrada para atualização editorial",
                            ));
                        };
                        let image_asset_id: Option<String> = if let Some(asset) =
                            &option.image_asset
                        {
                            Some(
                                sqlx::query_as::<_, (String,)>(
                                    "SELECT id FROM assets WHERE sha256=?1",
                                )
                                .bind(&asset.sha256)
                                .fetch_one(&mut *tx)
                                .await
                                .map_err(|e| {
                                    crate::security::internal_error("manifest_editorial_asset", e)
                                })?
                                .0,
                            )
                        } else {
                            None
                        };
                        sqlx::query("UPDATE custom_question_options SET image_url=?2,image_asset_id=?3,updated_at=datetime('now') WHERE id=?1").bind(&oid).bind(&option.image_url).bind(&image_asset_id).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_editorial_image",e))?;
                        sqlx::query("DELETE FROM option_links WHERE option_id=?1")
                            .bind(&oid)
                            .execute(&mut *tx)
                            .await
                            .map_err(|e| {
                                crate::security::internal_error("manifest_editorial_links", e)
                            })?;
                        for (s, link) in option.links.iter().enumerate() {
                            sqlx::query("INSERT INTO option_links(id,option_id,kind,label,url,sort_order) VALUES(?1,?2,?3,?4,?5,?6)").bind(uuid::Uuid::new_v4().to_string()).bind(&oid).bind(&link.kind).bind(&link.label).bind(&link.url).bind(s as i64).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_editorial_link",e))?;
                        }
                    }
                }
            }
            Some(id)
        }
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by,starts_at,ends_at,description,cover_url,external_url,cover_asset_id) VALUES(?1,?2,?3,'custom','draft',?4,?5,?6,?7,?8,?9,?10)").bind(&id).bind(&m.name).bind(&m.slug).bind(actor).bind(&m.starts_at).bind(&m.ends_at).bind(&m.description).bind(&m.cover_url).bind(&m.external_url).bind(&cover_asset_id).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_create_event",e))?;
        insert_items(&mut tx, &id, m).await?;
        Some(id)
    };
    let (i, o, l) = counts(m);
    sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,ip_address,details_json) VALUES(?1,?2,?3,?4,?5,?6,?7)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(actor)
        .bind("event_manifest_imported")
        .bind("event")
        .bind(event_id.as_deref())
        .bind(Option::<&str>::None)
        .bind(serde_json::json!({"schemaVersion":m.schema_version,"action":format!("{:?}",action),"manifestFingerprint":fingerprint(m).unwrap_or_default(),"itemCount":i,"optionCount":o,"linkCount":l}).to_string())
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
    apply_manifest(&m, expected, actor).await
}

#[cfg(feature = "server")]
pub(crate) async fn apply_normalized(
    m: &CustomEventManifest,
    expected: &str,
    actor: &str,
) -> Result<ManifestApplyResult, ServerFnError> {
    apply_manifest(m, expected, actor).await
}

/// Compatibility wrapper used by the legacy CLI. It intentionally preserves active creation.
#[cfg(feature = "server")]
pub async fn import(m: CustomEventManifest, apply: bool) -> Result<(usize, usize), ServerFnError> {
    let mut m = m;
    m.schema_version = CURRENT_SCHEMA_VERSION;
    let (i, o, _) = counts(&m);
    if !apply {
        return Ok((i, o));
    }
    let db = crate::db::pool();
    if let Some((id, _)) =
        sqlx::query_as::<_, (String, String)>("SELECT id,status FROM events WHERE slug=?1")
            .bind(&m.slug)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_lookup", e))?
    {
        let current = load_manifest(db, &id).await?;
        if current == m {
            return Ok((i, o));
        }
        if projection(&current) != projection(&m) {
            return Err(crate::security::public_error(
                "Evento existente diverge do manifesto; importação não altera estrutura usada.",
            ));
        }
        let mut tx = db
            .begin()
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_update_begin", e))?;
        sqlx::query("UPDATE events SET name=?2,description=?3,cover_url=?4,external_url=?5,updated_at=datetime('now') WHERE id=?1")
            .bind(&id)
            .bind(&m.name)
            .bind(&m.description)
            .bind(&m.cover_url)
            .bind(&m.external_url)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_update_event", e))?;
        for item in &m.items {
            for option in &item.options {
                let Some((option_id,)) = sqlx::query_as::<_, (String,)>(
                    "SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?1 AND pi.external_key=?2 AND o.external_key=?3",
                )
                .bind(&id)
                .bind(&item.external_key)
                .bind(&option.external_key)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| crate::security::internal_error("legacy_manifest_option", e))?
                else {
                    return Err(crate::security::public_error("option do manifesto não encontrada"));
                };
                sqlx::query("UPDATE custom_question_options SET image_url=?2,updated_at=datetime('now') WHERE id=?1")
                    .bind(&option_id)
                    .bind(&option.image_url)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| crate::security::internal_error("legacy_manifest_image", e))?;
                sqlx::query("DELETE FROM option_links WHERE option_id=?1")
                    .bind(&option_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| crate::security::internal_error("legacy_manifest_links", e))?;
                for (sort, link) in option.links.iter().enumerate() {
                    sqlx::query("INSERT INTO option_links(id,option_id,kind,label,url,sort_order) VALUES(?1,?2,?3,?4,?5,?6)")
                        .bind(uuid::Uuid::new_v4().to_string())
                        .bind(&option_id)
                        .bind(&link.kind)
                        .bind(&link.label)
                        .bind(&link.url)
                        .bind(sort as i64)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| crate::security::internal_error("legacy_manifest_link", e))?;
                }
            }
        }
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_update_commit", e))?;
        return Ok((i, o));
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("legacy_manifest_begin", e))?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,starts_at,ends_at,description,cover_url,external_url) VALUES(?1,?2,?3,'custom','active',?4,?5,?6,?7,?8)").bind(&id).bind(&m.name).bind(&m.slug).bind(&m.starts_at).bind(&m.ends_at).bind(&m.description).bind(&m.cover_url).bind(&m.external_url).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("legacy_manifest_event",e))?;
    insert_items(&mut tx, &id, &m).await?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("legacy_manifest_commit", e))?;
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
