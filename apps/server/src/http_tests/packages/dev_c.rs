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
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
        .bind(dev_slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("event DEV C");
    let item: (String,) = sqlx::query_as(
        "SELECT id FROM prediction_items WHERE event_id=?1 AND external_key='choice'",
    )
    .bind(&event.0)
    .fetch_one(crate::db::pool())
    .await
    .expect("item C");
    sqlx::query("UPDATE prediction_items SET title='Título estrutural incompatível' WHERE id=?1")
        .bind(&item.0)
        .execute(crate::db::pool())
        .await
        .expect("alteração estrutural C");
    let package = crate::event_package::export(&event.0)
        .await
        .expect("export C");
    fs::write(root.join("package-c.zip"), package).expect("gravar package C");
}
