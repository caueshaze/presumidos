use super::*;

#[tokio::test]
async fn close_predictions_allows_pool_or_event_creator_only() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let event_owner = seed_user(
        &format!("close-event-{suffix}"),
        &format!("close-event-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let pool_owner = seed_user(
        &format!("close-pool-{suffix}"),
        &format!("close-pool-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let outsider = seed_user(
        &format!("close-outsider-{suffix}"),
        &format!("close-outsider-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (_, pool_id) = insert_custom_event_pool(&event_owner, "Evento de fechamento").await;
    sqlx::query("UPDATE pools SET created_by=?2 WHERE id=?1")
        .bind(&pool_id)
        .bind(&pool_owner)
        .execute(crate::db::pool())
        .await
        .unwrap();
    add_membership(&pool_id, &pool_owner).await;
    add_membership(&pool_id, &outsider).await;
    let (event_token, event_csrf) = seed_session(&event_owner).await;
    let event_client = client_with_session(base, &event_token);
    let listed: Vec<serde_json::Value> = event_client
        .get(format!("{base}/api/pools"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed[0]["canClosePredictions"], true);
    let closed = event_client
        .post(format!("{base}/api/pools/{pool_id}/close-predictions"))
        .header("X-CSRF-Token", &event_csrf)
        .send()
        .await
        .unwrap();
    assert!(
        closed.status().is_success(),
        "{}",
        closed.text().await.unwrap()
    );
    let repeated = event_client
        .post(format!("{base}/api/pools/{pool_id}/close-predictions"))
        .header("X-CSRF-Token", &event_csrf)
        .send()
        .await
        .unwrap();
    assert!(repeated.status().is_success());
    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='pool_predictions_closed' AND target_id=?1",
    )
    .bind(&pool_id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);

    let (_, other_pool) =
        insert_custom_event_pool(&event_owner, "Outro evento de fechamento").await;
    sqlx::query("UPDATE pools SET created_by=?2 WHERE id=?1")
        .bind(&other_pool)
        .bind(&pool_owner)
        .execute(crate::db::pool())
        .await
        .unwrap();
    add_membership(&other_pool, &pool_owner).await;
    let (pool_token, pool_csrf) = seed_session(&pool_owner).await;
    let pool_client = client_with_session(base, &pool_token);
    let by_pool_owner = pool_client
        .post(format!("{base}/api/pools/{other_pool}/close-predictions"))
        .header("X-CSRF-Token", &pool_csrf)
        .send()
        .await
        .unwrap();
    assert!(
        by_pool_owner.status().is_success(),
        "{}",
        by_pool_owner.text().await.unwrap()
    );

    let (outsider_token, outsider_csrf) = seed_session(&outsider).await;
    let denied = client_with_session(base, &outsider_token)
        .post(format!("{base}/api/pools/{other_pool}/close-predictions"))
        .header("X-CSRF-Token", &outsider_csrf)
        .send()
        .await
        .unwrap();
    assert!(!denied.status().is_success());
}
