use super::*;

#[tokio::test]
async fn custom_scoring_owner_can_edit_before_lock_and_is_frozen_after() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner = seed_user(
        &format!("custom-score-{suffix}"),
        &format!("custom-score-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (event, pool) = insert_custom_event_pool(&owner, "Custom score").await;
    let (item, options) = insert_custom_question(
        &event,
        "Questão",
        "2999-01-01T00:00:00Z",
        "2999-01-01T00:00:00Z",
        &["A", "B"],
    )
    .await;
    let (token, csrf) = seed_session(&owner).await;
    let client = client_with_session(base, &token);
    let url = format!("{base}/api/pools/{pool}/scoring/items/{item}");
    assert_eq!(
        client
            .post(&url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"correctPoints":5,"incorrectPoints":2}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        client
            .post(format!("{base}/api/custom/predictions"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item,"optionId":options[0]}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    crate::custom_questions::set_correct_option(&item, &options[0])
        .await
        .unwrap();
    let points:(i64,)=sqlx::query_as("SELECT total_points FROM custom_prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND item_id=?3").bind(&pool).bind(&owner).bind(&item).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!(points.0, 5);
    sqlx::query("UPDATE prediction_items SET lock_at='2020-01-01T00:00:00Z' WHERE id=?1")
        .bind(&item)
        .execute(crate::db::pool())
        .await
        .unwrap();
    assert!(!client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"correctPoints":9,"incorrectPoints":0}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
}
