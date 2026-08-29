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
            .expect("actor PROD C");
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
        .bind(dev_slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("event PROD C");
    let package_b = package_smoke_package(&root, "package-b.zip");
    let stale_preview = crate::event_package::preview(&package_b)
        .await
        .expect("preview stale");
    sqlx::query("UPDATE events SET description='alteração concorrente' WHERE id=?1")
        .bind(&event.0)
        .execute(crate::db::pool())
        .await
        .expect("concorrência");
    assert!(crate::event_package::apply(
        &package_b,
        &stale_preview.manifest.base_fingerprint,
        &actor.0
    )
    .await
    .is_err());
    let package = package_smoke_package(&root, "package-c.zip");
    let preview = crate::event_package::preview(&package)
        .await
        .expect("preview Conflict");
    assert_eq!(
        preview.manifest.action,
        crate::custom_event_manifest::ImportAction::Conflict
    );
    assert!(preview
        .manifest
        .blocked_changes
        .iter()
        .any(|change| change.path.contains("title")));
    let before = crate::custom_event_manifest::export_for_event(&event.0)
        .await
        .expect("manifest antes C")
        .0;
    assert!(
        crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
            .await
            .is_err()
    );
    let after = crate::custom_event_manifest::export_for_event(&event.0)
        .await
        .expect("manifest depois C")
        .0;
    assert_eq!(before, after);
}
