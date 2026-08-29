use super::*;
use sha2::{Digest, Sha256};

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
#[cfg(test)]
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
pub(crate) fn change_kind<T>(old: &Option<T>, new: &Option<T>) -> String {
    match (old.is_some(), new.is_some()) {
        (false, true) => "adicionado".into(),
        (true, false) => "removido".into(),
        _ => "alterado".into(),
    }
}
