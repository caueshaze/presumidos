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
            .expect("actor PROD B");
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
        .bind(dev_slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("event PROD B");
    let before = crate::custom_event_manifest::export_for_event(&event.0)
        .await
        .expect("manifest antes B")
        .0;
    let used_before: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM pools WHERE event_id=?1),(SELECT COUNT(*) FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)")
                .bind(&event.0)
                .fetch_one(crate::db::pool())
                .await
                .expect("uso antes B");
    assert_eq!(used_before, (1, 1));
    let package = package_smoke_package(&root, "package-b.zip");
    let preview = crate::event_package::preview(&package)
        .await
        .expect("preview SafeUpdate");
    assert_eq!(
        preview.manifest.action,
        crate::custom_event_manifest::ImportAction::SafeUpdate
    );
    assert!(!preview.manifest.safe_changes.is_empty());
    assert!(preview.manifest.blocked_changes.is_empty());
    assert_eq!(preview.added_asset_count, 1);
    let result =
        crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
            .await
            .expect("apply SafeUpdate");
    assert_eq!(
        result.result.action,
        crate::custom_event_manifest::ImportAction::SafeUpdate
    );
    let after = crate::custom_event_manifest::export_for_event(&event.0)
        .await
        .expect("manifest depois B")
        .0;
    assert_eq!(
        package_smoke_structural_view(before),
        package_smoke_structural_view(after.clone())
    );
    assert_eq!(after.name, "Evento de promoção atualizado");
    assert_eq!(after.description.as_deref(), Some("Descrição editorial B"));
    let asset_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets")
        .fetch_one(crate::db::pool())
        .await
        .expect("assets B");
    assert_eq!(asset_count.0, 2);
    let used_after: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM pools WHERE event_id=?1),(SELECT COUNT(*) FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)")
                .bind(&event.0)
                .fetch_one(crate::db::pool())
                .await
                .expect("uso depois B");
    assert_eq!(used_after, used_before);
}
