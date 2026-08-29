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
            .expect("actor VMA DEV");
    let master = package_smoke_master([30, 100, 180]);
    let hash = hex::encode(sha2::Sha256::digest(&master));
    crate::assets::ingest_package_master(&master, &hash, &actor.0)
        .await
        .expect("asset VMA DEV");
    let manifest = vma_smoke_manifest(&hash);
    let content = serde_json::to_string(&manifest).expect("serializar VMA DEV");
    let preview = crate::custom_event_manifest::preview(&content)
        .await
        .expect("preview VMA Create");
    assert_eq!(
        preview.action,
        crate::custom_event_manifest::ImportAction::Create
    );
    assert_eq!(
        (preview.item_count, preview.option_count, preview.link_count),
        (19, 121, 4)
    );
    let result =
        crate::custom_event_manifest::apply_admin(&content, &preview.base_fingerprint, &actor.0)
            .await
            .expect("apply VMA DEV");
    let event_id = result.event_id.expect("event VMA DEV");
    let package = crate::event_package::export(&event_id)
        .await
        .expect("export VMA A");
    fs::write(root.join("vma-package-a.zip"), package).expect("gravar VMA A");
}
