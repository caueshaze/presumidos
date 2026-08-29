use crate::config::settings;
use crate::error::ServerFnError;
use image::{imageops, DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const VARIANTS: [&str; 3] = ["thumb", "card", "cover"];
const PACKAGE_MASTER_VARIANT: &str = "master";

#[derive(Debug, Clone)]
pub struct NormalizedAsset {
    pub master: Vec<u8>,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub variants: BTreeMap<String, VariantBytes>,
}

#[derive(Debug, Clone)]
pub struct VariantBytes {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetResponse {
    pub asset_id: String,
    pub sha256: String,
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub byte_size: usize,
    pub url: String,
    pub variants: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StagedAsset {
    pub directory: PathBuf,
    pub final_directory: PathBuf,
    pub sha256: String,
}

pub trait AssetStore {
    fn store(&self, normalized: &NormalizedAsset) -> Result<StagedAsset, ServerFnError>;
    fn promote(&self, staged: &StagedAsset) -> Result<(), ServerFnError>;
    fn read(&self, storage_key: &str) -> Result<Vec<u8>, ServerFnError>;
    fn exists(&self, storage_key: &str) -> bool;
    #[allow(dead_code)]
    fn delete(&self, storage_key: &str) -> Result<(), ServerFnError>;
    fn delete_staged(&self, staged: &StagedAsset);
}

#[derive(Debug, Clone)]
pub struct FilesystemAssetStore {
    root: PathBuf,
}

impl FilesystemAssetStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn safe_join(&self, key: &str) -> Result<PathBuf, ServerFnError> {
        let path = Path::new(key);
        if path.is_absolute()
            || path
                .components()
                .any(|part| part == std::path::Component::ParentDir)
        {
            return Err(crate::security::public_error("storage key inválida"));
        }
        Ok(self.root.join(path))
    }

    fn ensure_root(&self) -> Result<(), ServerFnError> {
        fs::create_dir_all(self.root.join(".staging"))
            .map_err(|e| crate::security::internal_error("asset_store_root", e))
    }

    fn final_files_complete(&self, directory: &Path) -> bool {
        directory.join("master.webp").is_file()
            && VARIANTS
                .iter()
                .all(|variant| directory.join(format!("{variant}.webp")).is_file())
    }

    fn delete_hash_directory(&self, sha256: &str) -> Result<(), ServerFnError> {
        let directory = self.safe_join(sha256)?;
        match fs::remove_dir_all(directory) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(crate::security::internal_error(
                "asset_delete_directory",
                error,
            )),
        }
    }
}

impl AssetStore for FilesystemAssetStore {
    fn store(&self, normalized: &NormalizedAsset) -> Result<StagedAsset, ServerFnError> {
        self.ensure_root()?;
        let directory = self.root.join(".staging").join(Uuid::new_v4().to_string());
        let final_directory = self.root.join(&normalized.sha256);
        fs::create_dir_all(&directory)
            .map_err(|e| crate::security::internal_error("asset_stage_directory", e))?;
        let write_result = (|| {
            fs::write(directory.join("master.webp"), &normalized.master)
                .map_err(|e| crate::security::internal_error("asset_stage_master", e))?;
            for (name, variant) in &normalized.variants {
                fs::write(directory.join(format!("{name}.webp")), &variant.bytes)
                    .map_err(|e| crate::security::internal_error("asset_stage_variant", e))?;
            }
            Ok::<(), ServerFnError>(())
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_dir_all(&directory);
            return Err(error);
        }
        Ok(StagedAsset {
            directory,
            final_directory,
            sha256: normalized.sha256.clone(),
        })
    }

    fn promote(&self, staged: &StagedAsset) -> Result<(), ServerFnError> {
        self.ensure_root()?;
        if staged.final_directory.exists() {
            let master = staged.final_directory.join("master.webp");
            let bytes = fs::read(&master)
                .map_err(|e| crate::security::internal_error("asset_promote_existing", e))?;
            if hex::encode(Sha256::digest(bytes)) != staged.sha256 {
                self.delete_staged(staged);
                return Err(crate::security::internal_error(
                    "asset_promote_hash_collision",
                    "storage directory exists with a different master hash",
                ));
            }
            if !self.final_files_complete(&staged.final_directory) {
                self.delete_staged(staged);
                return Err(crate::security::internal_error(
                    "asset_promote_missing_variant",
                    "storage directory exists without all generated variants",
                ));
            }
            self.delete_staged(staged);
            return Ok(());
        }
        match fs::rename(&staged.directory, &staged.final_directory) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.delete_staged(staged);
                Err(crate::security::internal_error("asset_promote", error))
            }
        }
    }

    fn read(&self, storage_key: &str) -> Result<Vec<u8>, ServerFnError> {
        let path = self.safe_join(storage_key)?;
        fs::read(path).map_err(|e| crate::security::internal_error("asset_read", e))
    }

    fn exists(&self, storage_key: &str) -> bool {
        self.safe_join(storage_key)
            .map(|path| path.is_file())
            .unwrap_or(false)
    }

    fn delete(&self, storage_key: &str) -> Result<(), ServerFnError> {
        let path = self.safe_join(storage_key)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(crate::security::internal_error("asset_delete", error)),
        }
    }

    fn delete_staged(&self, staged: &StagedAsset) {
        let _ = fs::remove_dir_all(&staged.directory);
    }
}

pub fn store() -> FilesystemAssetStore {
    FilesystemAssetStore::new(settings().asset_dir.clone())
}

fn exif_orientation(bytes: &[u8]) -> Option<u32> {
    let mut cursor = Cursor::new(bytes);
    let exif = exif::Reader::new().read_from_container(&mut cursor).ok()?;
    let orientation = exif
        .fields()
        .find(|field| field.tag == exif::Tag::Orientation)
        .and_then(|field| field.value.get_uint(0));
    orientation
}

fn apply_orientation(image: DynamicImage, orientation: Option<u32>) -> DynamicImage {
    let rgba = image.to_rgba8();
    match orientation.unwrap_or(1) {
        2 => DynamicImage::ImageRgba8(imageops::flip_horizontal(&rgba)),
        3 => DynamicImage::ImageRgba8(imageops::rotate180(&rgba)),
        4 => DynamicImage::ImageRgba8(imageops::flip_vertical(&rgba)),
        5 => DynamicImage::ImageRgba8(imageops::rotate90(&imageops::flip_horizontal(&rgba))),
        6 => DynamicImage::ImageRgba8(imageops::rotate90(&rgba)),
        7 => DynamicImage::ImageRgba8(imageops::rotate270(&imageops::flip_horizontal(&rgba))),
        8 => DynamicImage::ImageRgba8(imageops::rotate270(&rgba)),
        _ => DynamicImage::ImageRgba8(rgba),
    }
}

fn encode_webp(image: &DynamicImage) -> Result<Vec<u8>, ServerFnError> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageOutputFormat::WebP)
        .map_err(|e| {
            crate::security::public_error(format!("não foi possível normalizar a imagem: {e}"))
        })?;
    Ok(output.into_inner())
}

pub fn normalize_image(bytes: &[u8]) -> Result<NormalizedAsset, ServerFnError> {
    if bytes.len() > settings().asset_max_upload_bytes {
        return Err(crate::security::public_error(format!(
            "A imagem excede {} MB.",
            settings().asset_max_upload_bytes / (1024 * 1024)
        )));
    }
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| crate::security::public_error("Formato de imagem inválido."))?;
    let format = reader
        .format()
        .ok_or_else(|| crate::security::public_error("Formato de imagem inválido."))?;
    if !matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP
    ) {
        return Err(crate::security::public_error(
            "Formato não suportado. Use JPEG, PNG ou WebP.",
        ));
    }
    let (header_width, header_height) = reader
        .into_dimensions()
        .map_err(|_| crate::security::public_error("Não foi possível ler as dimensões."))?;
    if u64::from(header_width) * u64::from(header_height) > settings().asset_max_pixels {
        return Err(crate::security::public_error(
            "A imagem excede o limite de pixels.",
        ));
    }
    let image = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| crate::security::public_error("Formato de imagem inválido."))?
        .decode()
        .map_err(|_| crate::security::public_error("Não foi possível decodificar a imagem."))?;
    let pixels = u64::from(image.width()) * u64::from(image.height());
    if pixels > settings().asset_max_pixels {
        return Err(crate::security::public_error(
            "A imagem excede o limite de pixels.",
        ));
    }
    let image = apply_orientation(image, exif_orientation(bytes));
    let master = encode_webp(&image)?;
    let mut variants = BTreeMap::new();
    for (name, width) in [("thumb", 320u32), ("card", 640u32), ("cover", 1280u32)] {
        let resized = image.thumbnail(width, width);
        variants.insert(
            name.to_string(),
            VariantBytes {
                bytes: encode_webp(&resized)?,
                width: resized.width(),
                height: resized.height(),
            },
        );
    }
    let sha256 = hex::encode(Sha256::digest(&master));
    Ok(NormalizedAsset {
        master,
        sha256,
        width: image.width(),
        height: image.height(),
        variants,
    })
}

