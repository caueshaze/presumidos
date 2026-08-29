use super::*;

#[tokio::test]
async fn single_choice_is_a_real_prediction_without_match_and_respects_identity_lock_and_event() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user = seed_user(
        &format!("custom-{suffix}"),
        &format!("custom-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (event, pool) = insert_custom_event_pool(&user, "Premios").await;
    let (item, options) = insert_custom_question(
        &event,
        "Video of the Year",
        "2999-01-01T00:00:00Z",
        "2999-01-01T00:00:00Z",
        &["Artist A", "Artist B"],
    )
    .await;
    let (other_item, other_options) = insert_custom_question(
        &event,
        "Album",
        "2999-01-01T00:00:00Z",
        "2999-01-01T00:00:00Z",
        &["C", "D"],
    )
    .await;
    let (token, csrf) = seed_session(&user).await;
    let client = client_with_session(base, &token);
    let url = format!("{base}/api/custom/predictions");
    let submit = |option_id: &String| {
        client
            .post(&url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item,"optionId":option_id}))
    };
    assert_eq!(
        submit(&options[0])
            .send()
            .await
            .expect("responder")
            .status()
            .as_u16(),
        204
    );
    let stored: (String, Option<String>, Option<i64>,) = sqlx::query_as("SELECT id,match_id,home_score FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3").bind(&pool).bind(&user).bind(&item).fetch_one(crate::db::pool()).await.expect("prediction custom");
    assert!(
        stored.1.is_none() && stored.2.is_none(),
        "custom não possui Match nem placar"
    );
    let value: (String,) =
        sqlx::query_as("SELECT option_id FROM custom_prediction_values WHERE prediction_id=?1")
            .bind(&stored.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("valor custom");
    assert_eq!(value.0, options[0]);
    assert_eq!(
        crate::custom_questions::custom_prediction_value(&stored.0)
            .await
            .unwrap()
            .unwrap()
            .option_id,
        options[0]
    );
    assert_eq!(
        submit(&options[1])
            .send()
            .await
            .expect("editar")
            .status()
            .as_u16(),
        204
    );
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3",
    )
    .bind(&pool)
    .bind(&user)
    .bind(&item)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(count.0, 1);
    let changed: (String,) =
        sqlx::query_as("SELECT option_id FROM custom_prediction_values WHERE prediction_id=?1")
            .bind(&stored.0)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(changed.0, options[1]);
    assert!(!submit(&other_options[0])
        .send()
        .await
        .expect("opção errada")
        .status()
        .is_success());
    let questions: Vec<crate::models::CustomQuestion> = client
        .get(format!("{base}/api/custom/questions?poolId={pool}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(questions
        .iter()
        .any(|q| q.item_id == item && q.options.len() == 2));
    sqlx::query("UPDATE prediction_items SET lock_at='2020-01-01T00:00:00Z' WHERE id=?1")
        .bind(&item)
        .execute(crate::db::pool())
        .await
        .unwrap();
    assert!(!submit(&options[0])
        .send()
        .await
        .expect("lock")
        .status()
        .is_success());
    let football = insert_match("Brasil", "Japao", "2999-01-01T00:00:00Z").await;
    let football_item: (String,) =
        sqlx::query_as("SELECT prediction_item_id FROM matches WHERE id=?1")
            .bind(&football)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert!(!client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":pool,"itemId":football_item.0,"optionId":options[0]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    crate::custom_questions::set_correct_option(&item, &options[1])
        .await
        .expect("resultado correto da própria pergunta");
    assert!(
        crate::custom_questions::set_correct_option(&item, &other_options[0])
            .await
            .is_err()
    );
    let correct: (String,) =
        sqlx::query_as("SELECT correct_option_id FROM custom_questions WHERE item_id=?1")
            .bind(&item)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(correct.0, options[1]);
    let awarded: (i64,) = sqlx::query_as(
        "SELECT total_points FROM custom_prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND item_id=?3",
    )
    .bind(&pool)
    .bind(&user)
    .bind(&item)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(awarded.0, 1, "single choice resolvida participa do score");
    assert_ne!(item, other_item);
}
