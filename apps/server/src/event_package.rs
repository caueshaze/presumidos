#![cfg(feature = "server")]

use crate::custom_event_manifest::{
    CustomEventManifest, ImportAction, ManifestApplyResult, ManifestPreview,
};
use crate::error::ServerFnError;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Write};
use zip::read::ZipArchive;
use zip::write::FileOptions;

const MAX_PACKAGE_BYTES: usize = 128 * 1024 * 1024;
const MAX_PACKAGE_ENTRIES: usize = 128;
const MAX_PACKAGE_UNCOMPRESSED: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ParsedPackage {
    pub manifest: CustomEventManifest,
    pub assets: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagePreview {
    pub manifest: ManifestPreview,
    pub asset_count: usize,
    pub existing_asset_count: usize,
    pub added_asset_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageApplyResult {
    pub result: ManifestApplyResult,
    pub asset_count: usize,
    pub added_asset_count: usize,
}

fn expected_hashes(manifest: &CustomEventManifest) -> HashSet<String> {
    let mut hashes = HashSet::new();
    if let Some(asset) = &manifest.cover_asset {
        hashes.insert(asset.sha256.clone());
    }
    for item in &manifest.items {
        for option in &item.options {
            if let Some(asset) = &option.image_asset {
                hashes.insert(asset.sha256.clone());
            }
        }
    }
    hashes
}

fn validate_asset_name(name: &str) -> Option<String> {
    let hash = name.strip_prefix("assets/")?.strip_suffix(".webp")?;
    if hash.len() != 64
        || !hash.chars().all(|value| value.is_ascii_hexdigit())
        || hash.contains('/')
    {
        return None;
    }
    Some(hash.to_lowercase())
}

pub fn parse(bytes: &[u8]) -> Result<ParsedPackage, ServerFnError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(crate::security::public_error(
            "O pacote excede o limite permitido.",
        ));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| crate::security::public_error("ZIP inválido."))?;
    if archive.len() > MAX_PACKAGE_ENTRIES {
        return Err(crate::security::public_error(
            "O pacote contém arquivos demais.",
        ));
    }
    let mut manifest_json = None;
    let mut assets = HashMap::new();
    let mut uncompressed = 0usize;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| crate::security::public_error("Não foi possível ler o pacote."))?;
        if file.is_dir() || file.name().ends_with('/') {
            return Err(crate::security::public_error(
                "O pacote não pode conter diretórios.",
            ));
        }
        if file.name().starts_with('/')
            || file.name().contains("..")
            || file.enclosed_name().is_none()
        {
            return Err(crate::security::public_error("Entrada insegura no pacote."));
        }
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(crate::security::public_error(
                "Links simbólicos não são aceitos no pacote.",
            ));
        }
        let declared_size = usize::try_from(file.size()).unwrap_or(usize::MAX);
        uncompressed = uncompressed.saturating_add(declared_size);
        if uncompressed > MAX_PACKAGE_UNCOMPRESSED {
            return Err(crate::security::public_error(
                "O conteúdo descompactado excede o limite permitido.",
            ));
        }
        if file.name() == "manifest.json" {
            if manifest_json.is_some() {
                return Err(crate::security::public_error(
                    "O pacote possui mais de um manifest.json.",
                ));
            }
            manifest_json = Some(crate::assets::read_limited(
                &mut file,
                crate::custom_event_manifest::MAX_MANIFEST_BYTES,
            )?);
        } else if let Some(hash) = validate_asset_name(file.name()) {
            if declared_size > crate::config::settings().asset_max_upload_bytes {
                return Err(crate::security::public_error(
                    "Asset do pacote excede o limite permitido.",
                ));
            }
            if assets.contains_key(&hash) {
                return Err(crate::security::public_error("Asset duplicado no pacote."));
            }
            assets.insert(
                hash,
                crate::assets::read_limited(
                    &mut file,
                    crate::config::settings().asset_max_upload_bytes,
                )?,
            );
        } else {
            return Err(crate::security::public_error(
                "O pacote só pode conter manifest.json e assets/<sha256>.webp.",
            ));
        }
    }
    let manifest_bytes = manifest_json
        .ok_or_else(|| crate::security::public_error("O pacote não contém manifest.json."))?;
    let manifest_json = String::from_utf8(manifest_bytes)
        .map_err(|_| crate::security::public_error("manifest.json não está em UTF-8."))?;
    let mut manifest = crate::custom_event_manifest::parse_and_validate(&manifest_json)
        .map_err(crate::security::public_error)?;
    // O pacote usa sempre a forma canônica do Manifest Service. Isso mantém
    // import v1 compatível com o estado exportado v2 e preserva NoChange na
    // segunda importação.
    manifest.schema_version = crate::custom_event_manifest::CURRENT_SCHEMA_VERSION;
    let expected = expected_hashes(&manifest);
    if expected.len() != assets.len() || expected.iter().any(|hash| !assets.contains_key(hash)) {
        return Err(crate::security::public_error(
            "O pacote não contém todos os assets referenciados pelo manifesto.",
        ));
    }
    for (hash, bytes) in &assets {
        crate::assets::validate_package_master(bytes, hash)?;
    }
    Ok(ParsedPackage { manifest, assets })
}

pub async fn preview(bytes: &[u8]) -> Result<PackagePreview, ServerFnError> {
    let package = parse(bytes)?;
    let plan = crate::custom_event_manifest::preview_manifest(
        &package.manifest,
        &package.assets.keys().cloned().collect::<HashSet<_>>(),
    )
    .await?;
    let mut existing = 0usize;
    for hash in expected_hashes(&package.manifest) {
        existing += crate::assets::has_complete_asset(&hash).await? as usize;
    }
    Ok(PackagePreview {
        manifest: plan,
        asset_count: package.assets.len(),
        existing_asset_count: existing,
        added_asset_count: package.assets.len().saturating_sub(existing),
    })
}

pub async fn apply(
    bytes: &[u8],
    expected_fingerprint: &str,
    actor: &str,
) -> Result<PackageApplyResult, ServerFnError> {
    let package = parse(bytes)?;
    let available = package.assets.keys().cloned().collect::<HashSet<_>>();
    let plan =
        crate::custom_event_manifest::preview_manifest(&package.manifest, &available).await?;
    if plan.base_fingerprint != expected_fingerprint {
        return Err(crate::security::public_error(
            "O evento mudou desde o preview. Valide o pacote novamente.",
        ));
    }
    if matches!(plan.action, ImportAction::Conflict | ImportAction::Rejected) {
        return Err(crate::security::public_error(
            "O pacote contém alterações estruturais bloqueadas.",
        ));
    }
    let mut added = 0usize;
    let mut added_hashes = Vec::new();
    let ingest_result: Result<(), ServerFnError> = async {
        for (hash, data) in &package.assets {
            if !crate::assets::has_complete_asset(hash).await? {
                crate::assets::ingest_package_master(data, hash, actor).await?;
                added += 1;
                added_hashes.push(hash.clone());
            }
        }
        Ok(())
    }
    .await;
    if let Err(error) = ingest_result {
        for hash in &added_hashes {
            let _ = crate::assets::remove_unreferenced_asset(hash).await;
        }
        return Err(error);
    }
    let result = crate::custom_event_manifest::apply_normalized(
        &package.manifest,
        expected_fingerprint,
        actor,
    )
    .await;
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            for hash in &added_hashes {
                let _ = crate::assets::remove_unreferenced_asset(hash).await;
            }
            return Err(error);
        }
    };
    Ok(PackageApplyResult {
        result,
        asset_count: package.assets.len(),
        added_asset_count: added,
    })
}

pub async fn export(event_id: &str) -> Result<Vec<u8>, ServerFnError> {
    let (manifest, manifest_json) =
        crate::custom_event_manifest::export_for_event(event_id).await?;
    let mut hashes: Vec<_> = expected_hashes(&manifest).into_iter().collect();
    hashes.sort();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer
        .start_file("manifest.json", options)
        .map_err(|e| crate::security::internal_error("package_manifest_entry", e))?;
    writer
        .write_all(manifest_json.as_bytes())
        .map_err(|e| crate::security::internal_error("package_manifest_write", e))?;
    for hash in hashes {
        let bytes = crate::assets::master_bytes_for_hash(&hash)?;
        if hex::encode(Sha256::digest(&bytes)) != hash {
            return Err(crate::security::public_error(
                "Hash do asset armazenado não confere.",
            ));
        }
        writer
            .start_file(format!("assets/{hash}.webp"), options)
            .map_err(|e| crate::security::internal_error("package_asset_entry", e))?;
        writer
            .write_all(&bytes)
            .map_err(|e| crate::security::internal_error("package_asset_write", e))?;
    }
    let output = writer
        .finish()
        .map_err(|e| crate::security::internal_error("package_finish", e))?;
    Ok(output.into_inner())
}

#[cfg(test)]
#[path = "event_package_tests.rs"]
mod tests;
