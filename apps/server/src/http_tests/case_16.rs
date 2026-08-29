use super::*;

#[tokio::test]
async fn multiple_choice_predictions_are_sets_and_score_per_pool() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner = seed_user(
        &format!("multiple-owner-{suffix}"),
        &format!("multiple-owner-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&owner).await;
    let client = client_with_session(base, &token);
    let event: serde_json::Value = client
        .post(format!("{base}/api/custom/events"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":format!("Multiple {suffix}")}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let event_id = event["id"].as_str().unwrap();
    let item:serde_json::Value=client.post(format!("{base}/api/custom/events/{event_id}/items/multiple-choice")).header("X-CSRF-Token",&csrf).json(&json!({"title":"Artistas","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z","minSelections":1,"maxSelections":3})).send().await.unwrap().json().await.unwrap();
    let item_id = item["id"].as_str().unwrap();
    let mut options = Vec::new();
    for label in ["A", "B", "C", "D"] {
        let option: serde_json::Value = client
            .post(format!(
                "{base}/api/custom/events/{event_id}/items/{item_id}/options"
            ))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"label":label}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        options.push(option["id"].as_str().unwrap().to_string());
    }
    assert!(!client
        .post(format!("{base}/api/custom/multiple-choice-predictions"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":"nope","itemId":item_id,"optionIds":[options[0],options[0]]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert!(client
        .post(format!("{base}/api/custom/events/{event_id}/publish"))
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let pool_a: serde_json::Value = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":format!("Multiple A {suffix}"),"eventId":event_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pool_a = pool_a["id"].as_str().unwrap();
    let pool_b: serde_json::Value = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":format!("Multiple B {suffix}"),"eventId":event_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pool_b = pool_b["id"].as_str().unwrap();
    for (pool, partial) in [(pool_a, 2), (pool_b, 6)] {
        assert!(client
            .post(format!(
                "{base}/api/pools/{pool}/scoring/multiple-choice/{item_id}"
            ))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"exactPoints":8,"partialPoints":partial,"incorrectPoints":1}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
    }
    for pool in [pool_a, pool_b] {
        assert!(client
            .post(format!("{base}/api/custom/multiple-choice-predictions"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item_id,"optionIds":[options[0],options[2]]}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
    }
    assert!(client
        .post(format!(
            "{base}/api/admin/custom/multiple-choice/{item_id}/result"
        ))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"optionIds":[options[0],options[2],options[3]]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let partial:Vec<(String,String,i64)>=sqlx::query_as("SELECT pool_id,outcome,total_points FROM multiple_choice_prediction_score_breakdowns WHERE item_id=?1 ORDER BY pool_id").bind(item_id).fetch_all(crate::db::pool()).await.unwrap();
    assert!(partial
        .iter()
        .any(|(pool, outcome, points)| pool == pool_a && outcome == "partial" && *points == 2));
    assert!(partial
        .iter()
        .any(|(pool, outcome, points)| pool == pool_b && outcome == "partial" && *points == 6));
    assert!(client.post(format!("{base}/api/custom/multiple-choice-predictions")).header("X-CSRF-Token",&csrf).json(&json!({"poolId":pool_a,"itemId":item_id,"optionIds":[options[3],options[0],options[2]]})).send().await.unwrap().status().is_success());
    let stored: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3")
            .bind(pool_a)
            .bind(&owner)
            .bind(item_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM multiple_choice_prediction_options WHERE prediction_id=?1",
    )
    .bind(stored.0)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(count.0, 3);
    crate::scoring::recalculate_custom_breakdowns()
        .await
        .unwrap();
    let exact:(String,i64)=sqlx::query_as("SELECT outcome,total_points FROM multiple_choice_prediction_score_breakdowns WHERE pool_id=?1 AND item_id=?2").bind(pool_a).bind(item_id).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!(exact, ("exact".into(), 8));
}
