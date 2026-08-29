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
    let actor = seed_user(
        "package-smoke-dev",
        "package-smoke-dev@example.test",
        "Senha-forte-123",
        true,
    )
    .await;
    let master = package_smoke_master([20, 40, 80]);
    let hash = hex::encode(sha2::Sha256::digest(&master));
    crate::assets::ingest_package_master(&master, &hash, &actor)
        .await
        .expect("asset DEV");
    let content = package_smoke_manifest("Evento de promoção", dev_slug, &hash);
    let preview = crate::custom_event_manifest::preview(&content)
        .await
        .expect("preview DEV");
    assert_eq!(
        preview.action,
        crate::custom_event_manifest::ImportAction::Create
    );
    let result =
        crate::custom_event_manifest::apply_admin(&content, &preview.base_fingerprint, &actor)
            .await
            .expect("apply DEV");
    let event_id = result.event_id.expect("event DEV");
    let package = crate::event_package::export(&event_id)
        .await
        .expect("export A");
    fs::write(root.join("package-a.zip"), package).expect("gravar package A");
}
