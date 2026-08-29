use super::*;

pub(super) async fn run(
    root: &Path,
    dev_db: &Path,
    prod_db: &Path,
    dev_assets: &Path,
    prod_assets: &Path,
    dev_slug: &str,
) {
    package_smoke_env(&dev_db, &dev_assets);
    crate::db::init().await;
    let actor: (String,) =
        sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-dev'")
            .fetch_one(crate::db::pool())
            .await
            .expect("actor VMA DEV B");
    let master = package_smoke_master([180, 80, 30]);
    let hash = hex::encode(sha2::Sha256::digest(&master));
    crate::assets::ingest_package_master(&master, &hash, &actor.0)
        .await
        .expect("asset VMA B");
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug='vma-2026'")
        .fetch_one(crate::db::pool())
        .await
        .expect("event VMA DEV B");
    let mut manifest = crate::custom_event_manifest::export_for_event(&event.0)
        .await
        .expect("manifest VMA DEV B")
        .0;
    let base_fingerprint =
        crate::custom_event_manifest::fingerprint(&manifest).expect("base fingerprint VMA B");
    let asset = crate::custom_event_manifest::AssetReference {
        kind: "asset".into(),
        sha256: hash,
        media_type: "image/webp".into(),
    };
    manifest.cover_asset = Some(asset.clone());
    let option = manifest
        .items
        .iter_mut()
        .find(|item| item.external_key == "best-pop")
        .and_then(|item| item.options.first_mut())
        .expect("opção VMA B");
    option.image_asset = Some(asset);
    let content =
        crate::custom_event_manifest::canonical_json(&manifest).expect("serializar VMA B");
    crate::custom_event_manifest::apply_admin(&content, &base_fingerprint, &actor.0)
        .await
        .expect("aplicar alteração VMA DEV B");
    let package = crate::event_package::export(&event.0)
        .await
        .expect("export VMA B");
    fs::write(root.join("vma-package-b.zip"), package).expect("gravar VMA B");
}
