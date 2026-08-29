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
    let actor = seed_user(
        "package-smoke-prod",
        "package-smoke-prod@example.test",
        "Senha-forte-123",
        true,
    )
    .await;
    let package = package_smoke_package(&root, "package-a.zip");
    let preview = crate::event_package::preview(&package)
        .await
        .expect("preview Create");
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
        (3, 4, 1)
    );
    assert_eq!(preview.added_asset_count, 1);
    let applied = crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor)
        .await
        .expect("apply Create");
    assert_eq!(
        applied.result.action,
        crate::custom_event_manifest::ImportAction::Create
    );
    let event_id = applied.result.event_id.clone().expect("event PROD");
    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .expect("publicar PROD");
    let pool_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO pools(id,event_id,name,invite_code,created_by) VALUES(?1,?2,'Pool de promoção','package-smoke-invite',?3)")
                .bind(&pool_id)
                .bind(&event_id)
                .bind(&actor)
                .execute(crate::db::pool())
                .await
                .expect("pool de promoção");
    sqlx::query("INSERT INTO pool_members(pool_id,user_id) VALUES(?1,?2)")
        .bind(&pool_id)
        .bind(&actor)
        .execute(crate::db::pool())
        .await
        .expect("membro do pool de promoção");
    let item_id: (String,) = sqlx::query_as(
        "SELECT id FROM prediction_items WHERE event_id=?1 AND external_key='choice'",
    )
    .bind(&event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("item do pool de promoção");
    let option_id: (String,) = sqlx::query_as(
        "SELECT id FROM custom_question_options WHERE item_id=?1 AND external_key='a'",
    )
    .bind(&item_id.0)
    .fetch_one(crate::db::pool())
    .await
    .expect("opção do pool de promoção");
    let prediction_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO predictions(id,pool_id,user_id,item_id,match_id,home_score,away_score) VALUES(?1,?2,?3,?4,NULL,NULL,NULL)")
                .bind(&prediction_id)
                .bind(&pool_id)
                .bind(&actor)
                .bind(&item_id.0)
                .execute(crate::db::pool())
                .await
                .expect("prediction de promoção");
    sqlx::query("INSERT INTO custom_prediction_values(prediction_id,option_id) VALUES(?1,?2)")
        .bind(&prediction_id)
        .bind(&option_id.0)
        .execute(crate::db::pool())
        .await
        .expect("valor do prediction de promoção");
    let repeat = crate::event_package::preview(&package)
        .await
        .expect("preview NoChange");
    assert_eq!(
        repeat.manifest.action,
        crate::custom_event_manifest::ImportAction::NoChange
    );
    assert_eq!(repeat.existing_asset_count, 1);
    assert_eq!(repeat.added_asset_count, 0);
    let repeated = crate::event_package::apply(&package, &repeat.manifest.base_fingerprint, &actor)
        .await
        .expect("apply NoChange");
    assert_eq!(
        repeated.result.action,
        crate::custom_event_manifest::ImportAction::NoChange
    );
    let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM events WHERE slug=?1),(SELECT COUNT(*) FROM prediction_items WHERE event_id=?2),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?2),(SELECT COUNT(*) FROM assets)")
                .bind(dev_slug).bind(&event_id).fetch_one(crate::db::pool()).await.expect("contagens PROD");
    assert_eq!(counts, (1, 3, 4, 1));
    let prod_package = crate::event_package::export(&event_id)
        .await
        .expect("export PROD A");
    assert_eq!(
        package_smoke_entry_names(&package),
        package_smoke_entry_names(&prod_package)
    );
    let dev_parsed = crate::event_package::parse(&package).expect("manifest DEV A");
    let prod_parsed = crate::event_package::parse(&prod_package).expect("manifest PROD A");
    assert_eq!(dev_parsed.manifest, prod_parsed.manifest);
    assert_eq!(
        dev_parsed
            .assets
            .keys()
            .collect::<std::collections::HashSet<_>>(),
        prod_parsed
            .assets
            .keys()
            .collect::<std::collections::HashSet<_>>()
    );
    fs::write(root.join("package-prod-a.zip"), prod_package).expect("gravar round-trip");
}
