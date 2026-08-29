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
            .expect("actor VMA PROD B");
    let package = package_smoke_package(&root, "vma-package-b.zip");
    let preview = crate::event_package::preview(&package)
        .await
        .expect("preview VMA SafeUpdate");
    assert_eq!(
        preview.manifest.action,
        crate::custom_event_manifest::ImportAction::SafeUpdate
    );
    assert_eq!(
        (
            preview.manifest.item_count,
            preview.manifest.option_count,
            preview.manifest.link_count
        ),
        (19, 121, 4)
    );
    assert!(preview.manifest.blocked_changes.is_empty());
    assert_eq!(preview.added_asset_count, 1);
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug='vma-2026'")
        .fetch_one(crate::db::pool())
        .await
        .expect("event VMA PROD B");
    crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
        .await
        .expect("apply VMA SafeUpdate");
    let after = crate::custom_event_manifest::export_for_event(&event.0)
        .await
        .expect("manifest VMA depois B")
        .0;
    assert_eq!(
        (
            after.items.len(),
            after
                .items
                .iter()
                .map(|item| item.options.len())
                .sum::<usize>()
        ),
        (19, 121)
    );
    assert_eq!(
        after
            .items
            .iter()
            .map(|item| item
                .options
                .iter()
                .map(|option| option.links.len())
                .sum::<usize>())
            .sum::<usize>(),
        4
    );
    let asset_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets")
        .fetch_one(crate::db::pool())
        .await
        .expect("assets VMA PROD B");
    assert_eq!(
        asset_count.0, 4,
        "2 assets do smoke genérico + 2 assets do VMA"
    );
}
