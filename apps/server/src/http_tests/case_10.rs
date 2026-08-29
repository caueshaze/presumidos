use super::*;

#[tokio::test]
async fn published_event_versions_preserve_old_pools_and_switch_new_ones() {
    test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin = seed_user(
        &format!("version-admin-{suffix}"),
        &format!("version-admin-{suffix}@test"),
        "senha-correta-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin).await;
    let event = crate::custom_events::create(
        token.clone(),
        "Versão 1".into(),
        Some("2099-01-01T00:00:00Z".into()),
        Some("2099-12-31T00:00:00Z".into()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let item = crate::custom_events::add_item(
        token.clone(),
        event.id.clone(),
        "Pergunta V1".into(),
        "2099-01-01T00:00:00Z".into(),
        "2099-01-02T00:00:00Z".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        item.clone(),
        "A".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        item,
        "B".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    crate::custom_events::publish(token.clone(), event.id.clone(), csrf.clone())
        .await
        .unwrap();
    let pool_one = crate::pools::create_pool_for_event(
        token.clone(),
        "Pool V1".into(),
        Some(event.id.clone()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let v1: (String,) = sqlx::query_as("SELECT event_version_id FROM pools WHERE id=?1")
        .bind(&pool_one.id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();

    crate::custom_events::update_metadata(
        token.clone(),
        event.id.clone(),
        "Versão 2".into(),
        Some("2099-01-01T00:00:00Z".into()),
        Some("2099-12-31T00:00:00Z".into()),
        None,
        None,
        None,
        csrf.clone(),
    )
    .await
    .unwrap();
    let working: (String, String) =
        sqlx::query_as("SELECT id,name FROM event_versions WHERE event_id=?1 AND state='working'")
            .bind(&event.id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_ne!(working.0, v1.0);
    assert_eq!(working.1, "Versão 2");
    let old_name: (String,) = sqlx::query_as(
        "SELECT v.name FROM pools p JOIN event_versions v ON v.id=p.event_version_id WHERE p.id=?1",
    )
    .bind(&pool_one.id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(old_name.0, "Versão 1");
    crate::custom_events::publish(token.clone(), event.id.clone(), csrf.clone())
        .await
        .unwrap();
    let pool_two =
        crate::pools::create_pool_for_event(token, "Pool V2".into(), Some(event.id), csrf)
            .await
            .unwrap();
    let v2: (String,) = sqlx::query_as("SELECT event_version_id FROM pools WHERE id=?1")
        .bind(&pool_two.id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert_eq!(v2.0, working.0);
    assert_ne!(v1.0, v2.0);
    let old_cover_asset = format!("old-cover-{}", uuid::Uuid::new_v4());
    let new_cover_asset = format!("new-cover-{}", uuid::Uuid::new_v4());
    let old_sha256 = format!("{}{}", suffix.simple(), "a".repeat(32));
    let new_sha256 = format!("{}{}", suffix.simple(), "b".repeat(32));
    for (asset_id, storage_key, sha256) in [
        (
            &old_cover_asset,
            &format!("old-cover-{}/master.webp", suffix.simple()),
            old_sha256,
        ),
        (
            &new_cover_asset,
            &format!("new-cover-{}/master.webp", suffix.simple()),
            new_sha256,
        ),
    ] {
        sqlx::query(
            "INSERT INTO assets(id,storage_key,sha256,media_type,width,height,byte_size,uploaded_by)
             VALUES(?1,?2,?3,'image/webp',1,1,1,?4)",
        )
        .bind(asset_id)
        .bind(storage_key)
        .bind(sha256)
        .bind(&admin)
        .execute(crate::db::pool())
        .await
        .unwrap();
    }
    sqlx::query("UPDATE event_versions SET cover_asset_id=?2 WHERE id=?1")
        .bind(&v1.0)
        .bind(&old_cover_asset)
        .execute(crate::db::pool())
        .await
        .unwrap();
    sqlx::query("UPDATE event_versions SET cover_asset_id=?2 WHERE id=?1")
        .bind(&v2.0)
        .bind(&new_cover_asset)
        .execute(crate::db::pool())
        .await
        .unwrap();
    assert!(crate::assets::can_read(&old_cover_asset).await.unwrap());
    assert!(crate::assets::can_read(&new_cover_asset).await.unwrap());
    let old_code: (String,) = sqlx::query_as("SELECT invite_code FROM pools WHERE id=?1")
        .bind(&pool_one.id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    let new_code: (String,) = sqlx::query_as("SELECT invite_code FROM pools WHERE id=?1")
        .bind(&pool_two.id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    let old_preview = crate::pools::public_invite_preview(old_code.0.clone())
        .await
        .unwrap();
    let new_preview = crate::pools::public_invite_preview(new_code.0)
        .await
        .unwrap();
    assert_eq!(old_preview.event_name.as_deref(), Some("Versão 1"));
    assert_eq!(new_preview.event_name.as_deref(), Some("Versão 2"));
    let expected_old_cover = format!("/media/assets/{old_cover_asset}/cover");
    assert_eq!(
        old_preview.cover_asset_url.as_deref(),
        Some(expected_old_cover.as_str())
    );

    let newcomer = seed_user(
        &format!("version-newcomer-{suffix}"),
        &format!("version-newcomer-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (newcomer_token, newcomer_csrf) = seed_session(&newcomer).await;
    let joined = crate::pools::join_pool(newcomer_token, old_code.0, newcomer_csrf)
        .await
        .unwrap();
    assert_eq!(joined.id, pool_one.id);
    assert_eq!(joined.event.name, "Versão 1");
    let old_member_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id=?1")
            .bind(&pool_one.id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(old_member_count.0, 2);
}
