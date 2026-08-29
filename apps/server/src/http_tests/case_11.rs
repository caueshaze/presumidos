use super::*;

#[tokio::test]
async fn restoring_published_version_creates_new_revision_without_moving_old_pool() {
    test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin = seed_user(
        &format!("restore-admin-{suffix}"),
        &format!("restore-admin-{suffix}@test"),
        "senha-correta-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin).await;
    let event = crate::custom_events::create(
        token.clone(),
        "Restaurar evento".into(),
        Some("2099-01-01T00:00:00Z".into()),
        Some("2099-12-31T00:00:00Z".into()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let first_item = crate::custom_events::add_item(
        token.clone(),
        event.id.clone(),
        "Pergunta original".into(),
        "2099-01-01T00:00:00Z".into(),
        "2099-01-02T00:00:00Z".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    for label in ["A", "B"] {
        crate::custom_events::add_option(
            token.clone(),
            event.id.clone(),
            first_item.clone(),
            label.into(),
            csrf.clone(),
        )
        .await
        .unwrap();
    }
    crate::custom_events::publish(token.clone(), event.id.clone(), csrf.clone())
        .await
        .unwrap();
    let old_pool = crate::pools::create_pool_for_event(
        token.clone(),
        "Pool antigo".into(),
        Some(event.id.clone()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let old_version: (String,) = sqlx::query_as("SELECT event_version_id FROM pools WHERE id=?1")
        .bind(&old_pool.id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();

    let second_item = crate::custom_events::add_item(
        token.clone(),
        event.id.clone(),
        "Pergunta nova".into(),
        "2099-01-01T00:00:00Z".into(),
        "2099-01-02T00:00:00Z".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    for label in ["C", "D"] {
        crate::custom_events::add_option(
            token.clone(),
            event.id.clone(),
            second_item.clone(),
            label.into(),
            csrf.clone(),
        )
        .await
        .unwrap();
    }
    crate::custom_events::publish(token.clone(), event.id.clone(), csrf.clone())
        .await
        .unwrap();
    let current_version: (String,) =
        sqlx::query_as("SELECT current_published_version_id FROM events WHERE id=?1")
            .bind(&event.id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_ne!(old_version.0, current_version.0);

    let restored =
        crate::custom_event_manifest::restore_published_version(&event.id, &old_version.0, &admin)
            .await
            .unwrap();
    assert_ne!(restored.version_id, old_version.0);
    assert_ne!(restored.version_id, current_version.0);
    let restored_items: (i64, Option<String>) = sqlx::query_as(
        "SELECT COUNT(*),MIN(title) FROM prediction_items WHERE event_version_id=?1",
    )
    .bind(&restored.version_id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(restored_items.0, 1);
    assert_eq!(restored_items.1.as_deref(), Some("Pergunta original"));
    let old_pool_version: (String,) =
        sqlx::query_as("SELECT event_version_id FROM pools WHERE id=?1")
            .bind(&old_pool.id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(old_pool_version.0, old_version.0);
}
