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
            .expect("actor DEV");
    let master = package_smoke_master([120, 20, 40]);
    let hash = hex::encode(sha2::Sha256::digest(&master));
    crate::assets::ingest_package_master(&master, &hash, &actor.0)
        .await
        .expect("asset B");
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
        .bind(dev_slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("event DEV B");
    let asset: (String,) = sqlx::query_as("SELECT id FROM assets WHERE sha256=?1")
        .bind(&hash)
        .fetch_one(crate::db::pool())
        .await
        .expect("asset id B");
    sqlx::query("UPDATE events SET name='Evento de promoção atualizado',description='Descrição editorial B',cover_asset_id=?2 WHERE id=?1")
                .bind(&event.0).bind(&asset.0).execute(crate::db::pool()).await.expect("metadata B");
    let option: (String,) = sqlx::query_as("SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?1 AND o.external_key='a'").bind(&event.0).fetch_one(crate::db::pool()).await.expect("option B");
    sqlx::query("UPDATE custom_question_options SET image_asset_id=?2 WHERE id=?1")
        .bind(&option.0)
        .bind(&asset.0)
        .execute(crate::db::pool())
        .await
        .expect("option asset B");
    let package = crate::event_package::export(&event.0)
        .await
        .expect("export B");
    fs::write(root.join("package-b.zip"), package).expect("gravar package B");
}
