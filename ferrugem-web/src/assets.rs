#![cfg(feature = "server")]

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

fn storage_key(sha256: &str, variant: &str) -> String {
    format!("{sha256}/{variant}.webp")
}

async fn authorize_editor(
    session: &crate::auth::AuthSession,
    event_id: &str,
) -> Result<(), ServerFnError> {
    let allowed: Option<(String, String)> =
        sqlx::query_as("SELECT status,created_by FROM events WHERE id=?1 AND kind='custom'")
            .bind(event_id)
            .fetch_optional(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_event_access", e))?;
    let Some((_status, owner)) = allowed else {
        return Err(crate::security::public_error("Evento não encontrado."));
    };
    let is_admin: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id=?1")
        .bind(&session.user_id)
        .fetch_one(crate::db::pool())
        .await
        .map_err(|e| crate::security::internal_error("asset_event_admin", e))?;
    if is_admin.0 || owner == session.user_id {
        let status: (String,) = sqlx::query_as("SELECT status FROM events WHERE id=?1")
            .bind(event_id)
            .fetch_one(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_event_status", e))?;
        if status.0 != "draft" && !is_admin.0 {
            return Err(crate::security::public_error(
                "A mídia de evento publicado só pode ser alterada por um administrador.",
            ));
        }
        Ok(())
    } else {
        Err(crate::security::public_error(
            "Você não pode editar a mídia deste evento.",
        ))
    }
}

async fn persist_asset(normalized: &NormalizedAsset, actor: &str) -> Result<String, ServerFnError> {
    let db = crate::db::pool();
    let asset: Option<(String, String)> =
        sqlx::query_as("SELECT id,storage_key FROM assets WHERE sha256=?1")
            .bind(&normalized.sha256)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("asset_lookup_hash", e))?;
    let file_store = store();
    if let Some((id, master_key)) = asset {
        if !file_store.exists(&master_key) {
            return Err(crate::security::internal_error(
                "asset_missing_master",
                "asset metadata points to missing file",
            ));
        }
        let variants: Vec<(String,)> =
            sqlx::query_as("SELECT storage_key FROM asset_variants WHERE asset_id=?1")
                .bind(&id)
                .fetch_all(db)
                .await
                .map_err(|e| crate::security::internal_error("asset_lookup_variants", e))?;
        if variants.len() != VARIANTS.len()
            || variants
                .iter()
                .any(|(storage_key,)| !file_store.exists(storage_key))
        {
            return Err(crate::security::internal_error(
                "asset_missing_variant",
                "asset metadata points to missing variant file",
            ));
        }
        return Ok(id);
    }
    let staged = file_store.store(normalized)?;
    file_store.promote(&staged)?;
    let asset_id = Uuid::new_v4().to_string();
    let result: Result<String, ServerFnError> = async {
        // Serialize the hash lookup and insert. This gives rollback
        // compensation a stable database lock before it removes a failed
        // filesystem promotion.
        let mut tx = db
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| crate::security::internal_error("asset_insert_begin", e))?;
        if let Some((id, _)) =
            sqlx::query_as::<_, (String, String)>("SELECT id,storage_key FROM assets WHERE sha256=?1")
                .bind(&normalized.sha256)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| crate::security::internal_error("asset_insert_race", e))?
        {
            tx.rollback().await.ok();
            return Ok(id);
        }
        sqlx::query("INSERT INTO assets(id,storage_key,sha256,media_type,width,height,byte_size,uploaded_by) VALUES(?1,?2,?3,'image/webp',?4,?5,?6,?7)")
            .bind(&asset_id).bind(storage_key(&normalized.sha256, PACKAGE_MASTER_VARIANT)).bind(&normalized.sha256)
            .bind(i64::from(normalized.width)).bind(i64::from(normalized.height)).bind(normalized.master.len() as i64).bind(actor)
            .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("asset_insert", e))?;
        for (variant, data) in &normalized.variants {
            sqlx::query("INSERT INTO asset_variants(asset_id,variant,storage_key,width,height,byte_size) VALUES(?1,?2,?3,?4,?5,?6)")
                .bind(&asset_id).bind(variant).bind(storage_key(&normalized.sha256, variant)).bind(i64::from(data.width)).bind(i64::from(data.height)).bind(data.bytes.len() as i64)
                .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("asset_variant_insert", e))?;
        }
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("asset_insert_commit", e))?;
        Ok(asset_id)
    }
    .await;
    if result.is_err() {
        // SQLite rollback and filesystem cleanup are separate operations. The
        // helper checks references while holding BEGIN IMMEDIATE so an asset
        // cannot become referenced between the check and physical deletion.
        let _ = remove_unreferenced_asset(&normalized.sha256).await;
    }
    result
}

/// Compensates a failed asset/database operation without becoming a general
/// garbage collector. Only assets with no Event/Option reference are removed,
/// and the reference check plus row deletion are serialized by SQLite.
pub async fn remove_unreferenced_asset(sha256: &str) -> Result<bool, ServerFnError> {
    if sha256.len() != 64 || !sha256.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err(crate::security::public_error("Hash de asset inválido."));
    }
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_begin", e))?;
    let asset: Option<(String,)> = sqlx::query_as("SELECT id FROM assets WHERE sha256=?1")
        .bind(sha256)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_lookup", e))?;
    let Some(asset_id) = asset else {
        store().delete_hash_directory(sha256)?;
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("asset_cleanup_commit", e))?;
        return Ok(true);
    };
    let referenced: (i64,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM events WHERE cover_asset_id=?1) OR EXISTS(SELECT 1 FROM event_versions WHERE cover_asset_id=?1) OR EXISTS(SELECT 1 FROM custom_question_options WHERE image_asset_id=?1)",
    )
    .bind(&asset_id.0)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("asset_cleanup_reference", e))?;
    if referenced.0 != 0 {
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("asset_cleanup_keep_commit", e))?;
        return Ok(false);
    }
    store().delete_hash_directory(sha256)?;
    sqlx::query("DELETE FROM asset_variants WHERE asset_id=?1")
        .bind(&asset_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_variants", e))?;
    sqlx::query("DELETE FROM assets WHERE id=?1")
        .bind(&asset_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_row", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("asset_cleanup_delete_commit", e))?;
    Ok(true)
}

async fn attach(
    event_id: &str,
    version_id: &str,
    option_id: Option<&str>,
    asset_id: Option<&str>,
    actor: &str,
    action: &str,
) -> Result<(), ServerFnError> {
    let db = crate::db::pool();
    if let Some(option_id) = option_id {
        sqlx::query("UPDATE custom_question_options SET image_asset_id=?2 WHERE id=?1 AND EXISTS(SELECT 1 FROM prediction_items pi WHERE pi.id=custom_question_options.item_id AND pi.event_version_id=?3)")
            .bind(option_id).bind(asset_id).bind(version_id).execute(db).await
            .map_err(|e| crate::security::internal_error("asset_option_attach", e))?;
    } else {
        sqlx::query("UPDATE event_versions SET cover_asset_id=?2,updated_at=datetime('now') WHERE id=?1 AND state='working'")
            .bind(version_id)
            .bind(asset_id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("asset_cover_attach", e))?;
    }
    crate::security::append_audit_log(
        db,
        Some(actor),
        action,
        "event",
        Some(event_id),
        None,
        serde_json::json!({"assetId": asset_id}),
    )
    .await
}

pub async fn upload_cover(
    token: String,
    event_id: String,
    bytes: Vec<u8>,
    csrf: String,
) -> Result<AssetResponse, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    let normalized = normalize_image(&bytes)?;
    let asset_id = persist_asset(&normalized, &session.user_id).await?;
    attach(
        &event_id,
        &version_id,
        None,
        Some(&asset_id),
        &session.user_id,
        "event_cover_asset_changed",
    )
    .await?;
    response_for(&asset_id).await
}

pub async fn upload_option(
    token: String,
    event_id: String,
    option_id: String,
    bytes: Vec<u8>,
    csrf: String,
) -> Result<AssetResponse, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    let owns: Option<(String,)> = sqlx::query_as("SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.id=?1 AND pi.event_version_id=?2")
        .bind(&option_id).bind(&version_id).fetch_optional(crate::db::pool()).await
        .map_err(|e| crate::security::internal_error("asset_option_access", e))?;
    if owns.is_none() {
        return Err(crate::security::public_error("Opção não encontrada."));
    }
    let normalized = normalize_image(&bytes)?;
    let asset_id = persist_asset(&normalized, &session.user_id).await?;
    attach(
        &event_id,
        &version_id,
        Some(&option_id),
        Some(&asset_id),
        &session.user_id,
        "event_option_asset_changed",
    )
    .await?;
    response_for(&asset_id).await
}

pub async fn remove_cover(
    token: String,
    event_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    attach(
        &event_id,
        &version_id,
        None,
        None,
        &session.user_id,
        "event_cover_asset_removed",
    )
    .await
}

pub async fn remove_option(
    token: String,
    event_id: String,
    option_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    authorize_editor(&session, &event_id).await?;
    let version_id =
        crate::custom_event_manifest::ensure_working_revision(&event_id, &session.user_id).await?;
    let owns: Option<(String,)> = sqlx::query_as(
        "SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.id=?1 AND pi.event_version_id=?2",
    )
    .bind(&option_id)
    .bind(&version_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("asset_option_remove_access", e))?;
    if owns.is_none() {
        return Err(crate::security::public_error("Opção não encontrada."));
    }
    attach(
        &event_id,
        &version_id,
        Some(&option_id),
        None,
        &session.user_id,
        "event_option_asset_removed",
    )
    .await
}

async fn response_for(asset_id: &str) -> Result<AssetResponse, ServerFnError> {
    let row: (String, String, i64, i64, i64) =
        sqlx::query_as("SELECT sha256,media_type,width,height,byte_size FROM assets WHERE id=?1")
            .bind(asset_id)
            .fetch_one(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_response", e))?;
    let variants = VARIANTS
        .into_iter()
        .map(|variant| {
            (
                variant.to_string(),
                format!("/media/assets/{asset_id}/{variant}"),
            )
        })
        .collect();
    Ok(AssetResponse {
        asset_id: asset_id.to_string(),
        sha256: row.0,
        media_type: row.1,
        width: row.2 as u32,
        height: row.3 as u32,
        byte_size: row.4 as usize,
        url: format!("/media/assets/{asset_id}/cover"),
        variants,
    })
}

pub async fn read_variant(
    asset_id: &str,
    variant: &str,
) -> Result<(Vec<u8>, String), ServerFnError> {
    if !matches!(variant, "master" | "thumb" | "card" | "cover") {
        return Err(crate::security::public_error("Variante de asset inválida."));
    }
    let db = crate::db::pool();
    let row: Option<(String, String)> = sqlx::query_as("SELECT av.storage_key,a.sha256 FROM asset_variants av JOIN assets a ON a.id=av.asset_id WHERE av.asset_id=?1 AND av.variant=?2 UNION ALL SELECT storage_key,sha256 FROM assets WHERE id=?1 AND ?2='master'")
        .bind(asset_id).bind(variant).fetch_optional(db).await.map_err(|e| crate::security::internal_error("asset_variant_lookup", e))?;
    let Some((storage_key, sha256)) = row else {
        return Err(crate::security::public_error("Asset não encontrado."));
    };
    let bytes = store().read(&storage_key)?;
    Ok((bytes, sha256))
}

pub async fn can_read(asset_id: &str) -> Result<bool, ServerFnError> {
    let db = crate::db::pool();
    let published: (i64,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM events e JOIN event_versions v ON v.id=e.current_published_version_id WHERE e.status IN ('active','finished') AND (v.cover_asset_id=?1 OR EXISTS(SELECT 1 FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.image_asset_id=?1 AND pi.event_version_id=v.id)))")
        .bind(asset_id).fetch_one(db).await.map_err(|e| crate::security::internal_error("asset_public_access", e))?;
    if published.0 != 0 {
        return Ok(true);
    }
    let state = crate::auth::current_user(String::new()).await?;
    let Some(user) = state.user else {
        return Ok(false);
    };
    let allowed: (i64,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM events e JOIN event_versions v ON v.event_id=e.id AND v.state='working' WHERE e.status='draft' AND (v.cover_asset_id=?1 OR EXISTS(SELECT 1 FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE o.image_asset_id=?1 AND pi.event_version_id=v.id)) AND (e.created_by=?2 OR EXISTS(SELECT 1 FROM users u WHERE u.id=?2 AND u.is_admin=1)))")
        .bind(asset_id).bind(user.id).fetch_one(db).await.map_err(|e| crate::security::internal_error("asset_private_access", e))?;
    Ok(allowed.0 != 0)
}

pub fn master_bytes_for_hash(sha256: &str) -> Result<Vec<u8>, ServerFnError> {
    store().read(&storage_key(sha256, PACKAGE_MASTER_VARIANT))
}

pub async fn has_complete_asset(sha256: &str) -> Result<bool, ServerFnError> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id,storage_key FROM assets WHERE sha256=?1 AND media_type='image/webp'",
    )
    .bind(sha256)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("asset_complete_lookup", e))?;
    let Some((asset_id, master_key)) = row else {
        return Ok(false);
    };
    let file_store = store();
    if !file_store.exists(&master_key) {
        return Ok(false);
    }
    let variants: Vec<(String, String)> =
        sqlx::query_as("SELECT variant,storage_key FROM asset_variants WHERE asset_id=?1")
            .bind(asset_id)
            .fetch_all(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("asset_complete_variants", e))?;
    let variants: HashSet<_> = variants
        .into_iter()
        .filter(|(variant, storage_key)| {
            file_store.exists(storage_key) && VARIANTS.contains(&variant.as_str())
        })
        .map(|(variant, _)| variant)
        .collect();
    Ok(variants.len() == VARIANTS.len())
}

pub fn validate_package_master(
    bytes: &[u8],
    expected_sha: &str,
) -> Result<NormalizedAsset, ServerFnError> {
    if bytes.len() > settings().asset_max_upload_bytes {
        return Err(crate::security::public_error(
            "Asset do pacote excede o limite permitido.",
        ));
    }
    let raw_sha = hex::encode(Sha256::digest(bytes));
    if raw_sha != expected_sha {
        return Err(crate::security::public_error(format!(
            "Hash do asset {expected_sha} não confere."
        )));
    }
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| crate::security::public_error("Master de asset inválido."))?;
    if reader.format() != Some(ImageFormat::WebP) {
        return Err(crate::security::public_error(
            "Masters de pacote devem ser WebP.",
        ));
    }
    let (header_width, header_height) = reader
        .into_dimensions()
        .map_err(|_| crate::security::public_error("Não foi possível ler as dimensões."))?;
    if u64::from(header_width) * u64::from(header_height) > settings().asset_max_pixels {
        return Err(crate::security::public_error(
            "Master de asset excede o limite de pixels.",
        ));
    }
    let image = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| crate::security::public_error("Master de asset inválido."))?
        .decode()
        .map_err(|_| crate::security::public_error("Não foi possível decodificar o master."))?;
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
    Ok(NormalizedAsset {
        master: bytes.to_vec(),
        sha256: expected_sha.to_string(),
        width: image.width(),
        height: image.height(),
        variants,
    })
}

pub fn read_limited<R: Read>(reader: &mut R, limit: usize) -> Result<Vec<u8>, ServerFnError> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| crate::security::public_error(format!("falha ao ler pacote: {e}")))?;
    if bytes.len() > limit {
        return Err(crate::security::public_error(
            "Pacote ou asset excede o limite permitido.",
        ));
    }
    Ok(bytes)
}

pub async fn ensure_manifest_assets(
    manifest: &crate::custom_event_manifest::CustomEventManifest,
) -> Result<(), ServerFnError> {
    ensure_manifest_assets_available(manifest, &std::collections::HashSet::new()).await
}

pub async fn ensure_manifest_assets_available(
    manifest: &crate::custom_event_manifest::CustomEventManifest,
    available: &std::collections::HashSet<String>,
) -> Result<(), ServerFnError> {
    let mut hashes = Vec::new();
    if let Some(asset) = &manifest.cover_asset {
        hashes.push(asset.sha256.as_str());
    }
    for item in &manifest.items {
        for option in &item.options {
            if let Some(asset) = &option.image_asset {
                hashes.push(asset.sha256.as_str());
            }
        }
    }
    for sha256 in hashes {
        if available.contains(sha256) {
            continue;
        }
        if !has_complete_asset(sha256).await? {
            return Err(crate::security::public_error(format!(
                "Asset {sha256} referenciado pelo manifesto não foi encontrado."
            )));
        }
    }
    Ok(())
}

pub async fn ingest_package_master(
    bytes: &[u8],
    expected_sha256: &str,
    actor: &str,
) -> Result<String, ServerFnError> {
    let normalized = validate_package_master(bytes, expected_sha256)?;
    persist_asset(&normalized, actor).await
}

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
