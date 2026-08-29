use super::*;

pub(super) async fn run(
    root: &Path,
    dev_db: &Path,
    prod_db: &Path,
    dev_assets: &Path,
    prod_assets: &Path,
    dev_slug: &str,
) {
    package_smoke_env(&prod_db, &prod_assets);
    crate::db::init().await;
    let actor: (String,) =
        sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
            .fetch_one(crate::db::pool())
            .await
            .expect("actor VMA PROD");
    let package = package_smoke_package(&root, "vma-package-a.zip");
    let preview = crate::event_package::preview(&package)
        .await
        .expect("preview VMA PROD Create");
    assert_eq!(
        preview.manifest.action,
        crate::custom_event_manifest::ImportAction::Create
    );
    assert_eq!(
        (
            preview.manifest.item_count,
            preview.manifest.option_count,
            preview.manifest.link_count
        ),
        (19, 121, 4)
    );
    assert_eq!(preview.asset_count, 1);
    let applied =
        crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
            .await
            .expect("apply VMA PROD");
    let event_id = applied.result.event_id.expect("event VMA PROD");
    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .expect("publicar VMA PROD");
    let repeat = crate::event_package::preview(&package)
        .await
        .expect("preview VMA NoChange");
    assert_eq!(
        repeat.manifest.action,
        crate::custom_event_manifest::ImportAction::NoChange
    );
    let prod_package = crate::event_package::export(&event_id)
        .await
        .expect("export VMA PROD A");
    let dev_parsed = crate::event_package::parse(&package).expect("parse VMA DEV A");
    let prod_parsed = crate::event_package::parse(&prod_package).expect("parse VMA PROD A");
    assert_eq!(dev_parsed.manifest, prod_parsed.manifest);
    assert_eq!(
        dev_parsed.assets.keys().collect::<HashSet<_>>(),
        prod_parsed.assets.keys().collect::<HashSet<_>>()
    );
}
