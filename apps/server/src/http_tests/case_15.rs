use super::*;

#[tokio::test]
async fn numeric_predictions_are_exact_per_pool_and_recalculate_without_duplicates() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = seed_user(
        &format!("numeric-owner-{suffix}"),
        &format!("numeric-owner-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&owner_id).await;
    let client = client_with_session(base, &token);
    let event: serde_json::Value = client
        .post(format!("{base}/api/custom/events"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":"Evento Numeric {suffix}"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let event_id = event["id"].as_str().unwrap();
    let invalid=client.post(format!("{base}/api/custom/events/{event_id}/items/numeric")).header("X-CSRF-Token",&csrf).json(&json!({"title":"Inválida","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z","decimalPlaces":7})).send().await.unwrap();
    assert_eq!(invalid.status(), reqwest::StatusCode::BAD_REQUEST);
    let item:serde_json::Value=client.post(format!("{base}/api/custom/events/{event_id}/items/numeric")).header("X-CSRF-Token",&csrf).json(&json!({"title":"Quantos prêmios?","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z","decimalPlaces":2,"unitLabel":"prêmios","minValue":"0","maxValue":"20"})).send().await.unwrap().json().await.unwrap();
    let item_id = item["id"].as_str().unwrap();
    let option = client
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options"
        ))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"label":"indevida"}))
        .send()
        .await
        .unwrap();
    assert!(!option.status().is_success());
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
        .json(&json!({"name":"Numeric A {suffix}","eventId":event_id}))
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
        .json(&json!({"name":"Numeric B {suffix}","eventId":event_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pool_b = pool_b["id"].as_str().unwrap();
    for pool in [pool_a, pool_b] {
        let response = client
            .post(format!("{base}/api/custom/numeric-predictions"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item_id,"value":"7.00"}))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
    }
    let excessive = client
        .post(format!("{base}/api/custom/numeric-predictions"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":pool_a,"itemId":item_id,"value":"7.001"}))
        .send()
        .await
        .unwrap();
    assert_eq!(excessive.status(), reqwest::StatusCode::BAD_REQUEST);
    for (pool, exact, tolerance, within, incorrect) in
        [(pool_a, 5, "0", 0, 1), (pool_b, 12, "2.00", 3, 0)]
    {
        assert!(client.post(format!("{base}/api/pools/{pool}/scoring/numeric/{item_id}")).header("X-CSRF-Token",&csrf).json(&json!({"exactPoints":exact,"tolerance":tolerance,"withinTolerancePoints":within,"incorrectPoints":incorrect})).send().await.unwrap().status().is_success());
    }
    assert!(client
        .post(format!("{base}/api/admin/custom/numeric/{item_id}/result"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"value":"9"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let rows:Vec<(String,String,i64)>=sqlx::query_as("SELECT pool_id,outcome,total_points FROM numeric_prediction_score_breakdowns WHERE item_id=?1 ORDER BY pool_id").bind(item_id).fetch_all(crate::db::pool()).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_a && o == "incorrect" && *points == 1));
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_b && o == "within_tolerance" && *points == 3));
    assert!(client
        .post(format!("{base}/api/admin/custom/numeric/{item_id}/result"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"value":"7.00"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let rows:Vec<(String,String,i64)>=sqlx::query_as("SELECT pool_id,outcome,total_points FROM numeric_prediction_score_breakdowns WHERE item_id=?1 ORDER BY pool_id").bind(item_id).fetch_all(crate::db::pool()).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_a && o == "exact" && *points == 5));
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_b && o == "exact" && *points == 12));
    let reader_id = seed_user(
        &format!("numeric-reader-{suffix}"),
        &format!("numeric-reader-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (reader_token, reader_csrf) = seed_session(&reader_id).await;
    let reader = client_with_session(base, &reader_token);
    let invite: (String,) = sqlx::query_as("SELECT invite_code FROM pools WHERE id=?1")
        .bind(pool_a)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert!(reader
        .post(format!("{base}/api/pools/join"))
        .header("X-CSRF-Token", &reader_csrf)
        .json(&json!({"inviteCode":invite.0}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    sqlx::query("UPDATE prediction_items SET reveal_at='2000-01-01T00:00:00Z' WHERE id=?1")
        .bind(item_id)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let prediction: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3")
            .bind(pool_a)
            .bind(&owner_id)
            .bind(item_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    let reaction = reader
        .post(format!("{base}/api/pools/{pool_a}/prediction-reactions"))
        .header("X-CSRF-Token", &reader_csrf)
        .json(&json!({"targetUserId":owner_id,"predictionId":prediction.0,"emoji":"🔥"}))
        .send()
        .await
        .unwrap();
    assert!(
        reaction.status().is_success(),
        "numeric revelado aceita reação por prediction_id"
    );
    let stored: (String,) = sqlx::query_as(
        "SELECT prediction_id FROM prediction_reactions WHERE pool_id=?1 AND target_user_id=?2",
    )
    .bind(pool_a)
    .bind(&owner_id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(stored.0, prediction.0);
}
