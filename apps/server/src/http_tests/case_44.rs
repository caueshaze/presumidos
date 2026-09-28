use super::*;

#[tokio::test]
async fn pool_owner_can_reopen_only_the_final_pool_closure() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner = seed_user(
        &format!("reopen-owner-{suffix}"),
        &format!("reopen-owner-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let outsider = seed_user(
        &format!("reopen-outsider-{suffix}"),
        &format!("reopen-outsider-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (_, pool_id) = insert_custom_event_pool(&owner, "Bolão reaberto").await;
    sqlx::query("UPDATE pools SET predictions_closed_at=datetime('now'),closed_at=datetime('now') WHERE id=?1")
        .bind(&pool_id)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let (owner_token, owner_csrf) = seed_session(&owner).await;
    let reopened = client_with_session(base, &owner_token)
        .post(format!("{base}/api/pools/{pool_id}/reopen"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .unwrap();
    assert!(reopened.status().is_success());
    let state: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT predictions_closed_at,closed_at FROM pools WHERE id=?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert!(state.0.is_some(), "reabrir não reabre palpites");
    assert!(state.1.is_none());
    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='pool_reopened' AND target_id=?1",
    )
    .bind(&pool_id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);

    let (outsider_token, outsider_csrf) = seed_session(&outsider).await;
    let denied = client_with_session(base, &outsider_token)
        .post(format!("{base}/api/pools/{pool_id}/reopen"))
        .header("X-CSRF-Token", &outsider_csrf)
        .send()
        .await
        .unwrap();
    assert!(!denied.status().is_success());
}
