#![cfg(feature = "server")]

use crate::custom_event_manifest::CustomEventManifest;
use crate::error::ServerFnError;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageExportPreview {
    pub asset_count: usize,
    pub external_image_count: usize,
    pub external_images: Vec<ExternalImageReference>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalImageReference {
    pub question: String,
    pub option_label: Option<String>,
    pub url: String,
}

pub(crate) async fn for_working_event(
    event_id: &str,
) -> Result<PackageExportPreview, ServerFnError> {
    let (manifest, _) = crate::custom_event_manifest::export_for_working_event(event_id).await?;
    let external_images = external_images(&manifest);
    Ok(PackageExportPreview {
        asset_count: asset_hashes(&manifest).len(),
        external_image_count: external_images.len(),
        external_images,
    })
}

fn external_images(manifest: &CustomEventManifest) -> Vec<ExternalImageReference> {
    manifest
        .cover_url
        .iter()
        .map(|url| ExternalImageReference {
            question: "Capa do evento".into(),
            option_label: None,
            url: url.clone(),
        })
        .chain(manifest.items.iter().flat_map(|item| {
            item.options.iter().filter_map(|option| {
                option.image_url.as_ref().map(|url| ExternalImageReference {
                    question: item.title.clone(),
                    option_label: Some(option.label.clone()),
                    url: url.clone(),
                })
            })
        }))
        .collect()
}

fn asset_hashes(manifest: &CustomEventManifest) -> HashSet<&str> {
    manifest
        .cover_asset
        .iter()
        .map(|asset| asset.sha256.as_str())
        .chain(
            manifest
                .items
                .iter()
                .flat_map(|item| &item.options)
                .flat_map(|option| option.image_asset.iter().map(|asset| asset.sha256.as_str())),
        )
        .collect()
}
