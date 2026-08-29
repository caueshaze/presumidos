#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;

    fn seed_asset_test_env() {
        std::env::set_var("APP_ENV", "test");
        std::env::set_var(
            "ADMIN_BOOTSTRAP_SECRET",
            "bootstrap-secret-super-seguro-0123456789abcdef",
        );
    }

    fn fixture(format: ImageOutputFormat, width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgb8(ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 251) as u8, (y % 251) as u8, 127])
        }));
        let mut output = Cursor::new(Vec::new());
        image
            .write_to(&mut output, format)
            .expect("codificar fixture");
        output.into_inner()
    }

    fn jpeg_with_orientation(width: u32, height: u32, orientation: u16) -> Vec<u8> {
        let jpeg = fixture(ImageOutputFormat::Jpeg(90), width, height);
        let mut tiff = Vec::from(&b"II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0"[..]);
        tiff.extend_from_slice(&u32::from(orientation).to_le_bytes());
        tiff.extend_from_slice(&[0; 4]);
        let mut exif = Vec::from(&b"Exif\0\0"[..]);
        exif.extend_from_slice(&tiff);
        let segment_length = u16::try_from(exif.len() + 2).expect("EXIF pequeno");
        let mut output = vec![0xff, 0xd8, 0xff, 0xe1];
        output.extend_from_slice(&segment_length.to_be_bytes());
        output.extend_from_slice(&exif);
        output.extend_from_slice(&jpeg[2..]);
        output
    }

    #[test]
    fn normalizes_supported_formats_and_generates_bounded_variants() {
        seed_asset_test_env();
        for format in [
            ImageOutputFormat::Jpeg(90),
            ImageOutputFormat::Png,
            ImageOutputFormat::WebP,
        ] {
            let normalized = normalize_image(&fixture(format, 2000, 1000)).expect("normalizar");
            assert_eq!((normalized.width, normalized.height), (2000, 1000));
            assert!(!normalized.master.is_empty());
            assert_eq!(
                normalized.sha256,
                hex::encode(Sha256::digest(&normalized.master))
            );
            assert_eq!(normalized.variants["thumb"].width, 320);
            assert_eq!(normalized.variants["thumb"].height, 160);
            assert_eq!(normalized.variants["card"].width, 640);
            assert_eq!(normalized.variants["card"].height, 320);
            assert_eq!(normalized.variants["cover"].width, 1280);
            assert_eq!(normalized.variants["cover"].height, 640);
        }
    }

    #[test]
    fn rejects_svg_and_oversized_upload_before_decode() {
        seed_asset_test_env();
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#;
        assert!(normalize_image(svg).is_err());
        assert!(normalize_image(b"GIF89a\x01\0\x01\0\0\0\0\0\0\0\0\0\0\0\0").is_err());
        assert!(normalize_image(b"BM\x00\x00\x00\x00\x00\x00\x00\x00").is_err());
        assert!(normalize_image(b"II*\0\x08\0\0\0\x00\0\0\0").is_err());
        let oversized = vec![0u8; settings().asset_max_upload_bytes + 1];
        assert!(normalize_image(&oversized).is_err());
    }

    #[test]
    fn rejects_pixel_bomb_from_header_before_full_decode() {
        seed_asset_test_env();
        let mut png = fixture(ImageOutputFormat::Png, 1, 1);
        png[16..20].copy_from_slice(&10_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&10_000u32.to_be_bytes());
        assert!(normalize_image(&png).is_err());
    }

    #[test]
    fn filesystem_store_promotes_hash_addressed_master_and_variants() {
        seed_asset_test_env();
        let root = std::env::temp_dir().join(format!("presumidos-assets-{}", Uuid::new_v4()));
        let normalized = normalize_image(&fixture(ImageOutputFormat::Png, 32, 16)).unwrap();
        let store = FilesystemAssetStore::new(&root);
        let staged = store.store(&normalized).unwrap();
        assert!(!store.exists(&storage_key(&normalized.sha256, "master")));
        store.promote(&staged).unwrap();
        assert!(store.exists(&storage_key(&normalized.sha256, "master")));
        assert_eq!(
            store
                .read(&storage_key(&normalized.sha256, "master"))
                .unwrap(),
            normalized.master
        );
        store
            .delete(&storage_key(&normalized.sha256, "master"))
            .unwrap();
        assert!(!store.exists(&storage_key(&normalized.sha256, "master")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_to_reuse_a_hash_directory_with_missing_variant() {
        seed_asset_test_env();
        let root = std::env::temp_dir().join(format!("presumidos-assets-{}", Uuid::new_v4()));
        let normalized = normalize_image(&fixture(ImageOutputFormat::Png, 32, 16)).unwrap();
        let store = FilesystemAssetStore::new(&root);
        store.promote(&store.store(&normalized).unwrap()).unwrap();
        fs::remove_file(root.join(format!("{}/card.webp", normalized.sha256))).unwrap();
        let staged = store.store(&normalized).unwrap();
        assert!(store.promote(&staged).is_err());
        assert!(!staged.directory.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_master_does_not_retain_exif_container() {
        seed_asset_test_env();
        let normalized = normalize_image(&fixture(ImageOutputFormat::Jpeg(90), 16, 8)).unwrap();
        assert!(exif_orientation(&normalized.master).is_none());
    }

    #[test]
    fn corrects_exif_orientation_before_normalizing() {
        seed_asset_test_env();
        let normalized = normalize_image(&jpeg_with_orientation(16, 8, 6)).unwrap();
        assert_eq!((normalized.width, normalized.height), (8, 16));
        assert!(exif_orientation(&normalized.master).is_none());
    }
}
