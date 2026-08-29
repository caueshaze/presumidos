use super::*;

pub(super) fn package_smoke_manifest(name: &str, slug: &str, asset_sha: &str) -> String {
    serde_json::json!({
        "schemaVersion": 2,
        "name": name,
        "slug": slug,
        "kind": "custom",
        "description": "Evento de promoção com imagem interna.",
        "externalUrl": "https://example.test/evento",
        "startsAt": "2099-01-01T00:00:00Z",
        "endsAt": "2099-01-02T00:00:00Z",
        "coverAsset": {"kind": "asset", "sha256": asset_sha, "mediaType": "image/webp"},
        "items": [
            {
                "externalKey": "choice",
                "kind": "single_choice",
                "title": "Escolha",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "options": [
                    {"externalKey": "a", "label": "A", "imageAsset": {"kind": "asset", "sha256": asset_sha, "mediaType": "image/webp"}, "links": [{"kind": "official", "label": "Site oficial", "url": "https://example.test/a"}]},
                    {"externalKey": "b", "label": "B"}
                ]
            },
            {
                "externalKey": "number",
                "kind": "numeric",
                "title": "Número",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "decimalPlaces": 1,
                "unitLabel": "pontos",
                "minValue": "0.0",
                "maxValue": "10.0"
            },
            {
                "externalKey": "multiple",
                "kind": "multiple_choice",
                "title": "Múltipla",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "minSelections": 1,
                "maxSelections": 2,
                "options": [
                    {"externalKey": "x", "label": "X"},
                    {"externalKey": "y", "label": "Y"}
                ]
            }
        ]
    })
    .to_string()
}

pub(super) fn vma_smoke_manifest(
    asset_sha: &str,
) -> crate::custom_event_manifest::CustomEventManifest {
    let mut manifest = crate::custom_event_manifest::parse_and_validate(include_str!(
        "../../../resources/events/vma-2026.json"
    ))
    .expect("manifesto VMA do smoke");
    manifest.schema_version = crate::custom_event_manifest::CURRENT_SCHEMA_VERSION;
    let asset = crate::custom_event_manifest::AssetReference {
        kind: "asset".into(),
        sha256: asset_sha.into(),
        media_type: "image/webp".into(),
    };
    manifest.cover_asset = Some(asset.clone());
    for item in &mut manifest.items {
        if matches!(
            item.external_key.as_str(),
            "video-of-the-year" | "best-pop" | "best-k-pop"
        ) {
            if let Some(option) = item.options.first_mut() {
                option.image_asset = Some(asset.clone());
            }
        }
    }
    manifest
}

pub(super) fn package_smoke_master(rgb: [u8; 3]) -> Vec<u8> {
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;

    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(24, 12, Rgb(rgb)));
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageOutputFormat::WebP)
        .expect("master de smoke");
    output.into_inner()
}

pub(super) fn package_smoke_root() -> PathBuf {
    PathBuf::from(
        std::env::var("PACKAGE_SMOKE_ROOT")
            .expect("PACKAGE_SMOKE_ROOT obrigatório no smoke de pacote"),
    )
}

pub(super) fn package_smoke_env(db: &Path, assets: &Path) {
    seed_http_test_env();
    std::env::set_var("DATABASE_PATH", db.to_string_lossy().to_string());
    std::env::set_var("PRESUMIDOS_ASSET_DIR", assets.to_string_lossy().to_string());
}

pub(super) fn package_smoke_package(root: &Path, name: &str) -> Vec<u8> {
    fs::read(root.join(name)).expect("pacote de smoke")
}

pub(super) fn package_smoke_structural_view(
    mut manifest: crate::custom_event_manifest::CustomEventManifest,
) -> serde_json::Value {
    manifest.name.clear();
    manifest.description = None;
    manifest.cover_url = None;
    manifest.cover_asset = None;
    manifest.external_url = None;
    for item in &mut manifest.items {
        for option in &mut item.options {
            option.image_url = None;
            option.image_asset = None;
            option.links.clear();
        }
    }
    serde_json::to_value(manifest).expect("estrutura de manifesto")
}

pub(super) fn package_smoke_entry_names(bytes: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip smoke");
    let mut names = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .expect("entrada zip")
                .name()
                .to_string()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

pub(super) fn package_smoke_without_assets(slug: &str) -> Vec<u8> {
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "name": "Pacote HTTP",
        "slug": slug,
        "kind": "custom",
        "items": [{
            "externalKey": "choice",
            "kind": "single_choice",
            "title": "Escolha",
            "lockAt": "2099-01-01T00:00:00Z",
            "revealAt": "2099-01-02T00:00:00Z",
            "options": [{"externalKey": "a", "label": "A"}, {"externalKey": "b", "label": "B"}]
        }]
    })
    .to_string();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file(
            "manifest.json",
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .expect("manifest HTTP");
    std::io::Write::write_all(&mut writer, manifest.as_bytes()).expect("manifest bytes HTTP");
    writer.finish().expect("zip HTTP").into_inner()
}
