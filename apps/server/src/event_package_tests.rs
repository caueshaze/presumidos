use super::*;
use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
use std::io::Cursor;

fn master() -> Vec<u8> {
    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(8, 4, Rgb([20, 40, 80])));
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageOutputFormat::WebP)
        .expect("master webp");
    output.into_inner()
}

fn package_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn parses_v2_package_and_rejects_missing_or_extra_assets() {
    let bytes = master();
    let hash = hex::encode(Sha256::digest(&bytes));
    let manifest = format!(
        r#"{{"schemaVersion":2,"name":"Pacote","slug":"pacote","kind":"custom","items":[{{"externalKey":"choice","kind":"single_choice","title":"Escolha","lockAt":"2099-01-01T00:00:00Z","revealAt":"2099-01-02T00:00:00Z","options":[{{"externalKey":"a","label":"A","imageAsset":{{"kind":"asset","sha256":"{hash}","mediaType":"image/webp"}}}},{{"externalKey":"b","label":"B","imageAsset":{{"kind":"asset","sha256":"{hash}","mediaType":"image/webp"}}}}]}}]}}"#
    );
    let good = package_with(&[
        ("manifest.json", manifest.as_bytes()),
        (&format!("assets/{hash}.webp"), &bytes),
    ]);
    let parsed = parse(&good).expect("pacote válido");
    assert_eq!(parsed.assets.len(), 1, "asset deduplicado por hash");
    assert_eq!(expected_hashes(&parsed.manifest).len(), 1);

    let missing = package_with(&[("manifest.json", manifest.as_bytes())]);
    assert!(parse(&missing).is_err());
    let missing_manifest = package_with(&[("assets/unused.webp", &bytes)]);
    assert!(parse(&missing_manifest).is_err());
    let extra_hash = "a".repeat(64);
    let extra = package_with(&[
        ("manifest.json", manifest.as_bytes()),
        (&format!("assets/{hash}.webp"), &bytes),
        (&format!("assets/{extra_hash}.webp"), &bytes),
    ]);
    assert!(parse(&extra).is_err());

    let mut wrong_bytes = bytes.clone();
    wrong_bytes[0] ^= 1;
    let wrong = package_with(&[
        ("manifest.json", manifest.as_bytes()),
        (&format!("assets/{hash}.webp"), &wrong_bytes),
    ]);
    assert!(parse(&wrong).is_err());
}

#[test]
fn canonicalizes_legacy_v1_manifest_inside_a_package() {
    let manifest = br#"{"schemaVersion":1,"name":"Evento","slug":"evento","kind":"custom","items":[{"externalKey":"categoria","kind":"single_choice","title":"Categoria","lockAt":"2026-09-27T19:30:00Z","revealAt":"2026-09-27T19:30:00Z","options":[{"externalKey":"a","label":"A"},{"externalKey":"b","label":"B"}]}]}"#;
    let parsed = parse(&package_with(&[("manifest.json", manifest)])).unwrap();
    assert_eq!(
        parsed.manifest.schema_version,
        crate::custom_event_manifest::CURRENT_SCHEMA_VERSION
    );
}

#[test]
fn rejects_zip_slip_entry() {
    let package = package_with(&[("../manifest.json", b"{}")]);
    assert!(parse(&package).is_err());
}

#[test]
fn rejects_limited_reader_overflow() {
    let mut reader = Cursor::new(vec![0u8; 11]);
    assert!(crate::assets::read_limited(&mut reader, 10).is_err());
}

#[test]
fn rejects_oversized_package_before_zip_decode() {
    assert!(parse(&vec![0u8; MAX_PACKAGE_BYTES + 1]).is_err());
}
