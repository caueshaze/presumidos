use super::*;
use std::collections::HashSet;

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
