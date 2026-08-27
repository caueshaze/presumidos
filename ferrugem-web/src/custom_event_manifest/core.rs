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
    pub version_id: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedPlan {
    pub(crate) preview: ManifestPreview,
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

pub(crate) fn counts(m: &CustomEventManifest) -> (usize, usize, usize) {
    let options = m.items.iter().map(|i| i.options.len()).sum();
    let links = m
        .items
        .iter()
        .flat_map(|i| i.options.iter())
        .map(|o| o.links.len())
        .sum();
    (m.items.len(), options, links)
}
pub(crate) fn absent_fingerprint(slug: &str) -> String {
    hex::encode(Sha256::digest(format!("absent:{slug}").as_bytes()))
}

pub fn draft_fingerprint(slug: &str) -> String {
    hex::encode(Sha256::digest(format!("draft:{slug}").as_bytes()))
}
pub(crate) fn projection(m: &CustomEventManifest) -> CustomEventManifest {
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

pub(crate) fn safe_diff(
    a: &CustomEventManifest,
    b: &CustomEventManifest,
) -> Vec<ManifestDiffEntry> {
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

pub(crate) fn structural_diff(
    a: &CustomEventManifest,
    b: &CustomEventManifest,
) -> Vec<ManifestDiffEntry> {
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
