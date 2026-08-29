use super::*;

#[tokio::test]
async fn prediction_reuse_copies_independent_values_only_within_the_same_event_version() {
    test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = seed_user(
        &format!("reuse-{suffix}"),
        &format!("reuse-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&user).await;
    let event = crate::custom_events::create(
        token.clone(),
        "Reuso".into(),
        None,
        Some("2099-12-31T00:00:00Z".into()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let single = crate::custom_events::add_item(
        token.clone(),
        event.id.clone(),
        "Única".into(),
        "2099-01-01T00:00:00Z".into(),
        "2099-01-02T00:00:00Z".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    let option_a = crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        single.clone(),
        "A".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    let option_b = crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        single.clone(),
        "B".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    let numeric = crate::custom_events::add_numeric_item(
        token.clone(),
        event.id.clone(),
        "Número".into(),
        "2099-01-01T00:00:00Z".into(),
        "2099-01-02T00:00:00Z".into(),
        2,
        None,
        None,
        None,
        csrf.clone(),
    )
    .await
    .unwrap();
    let multiple = crate::custom_events::add_multiple_choice_item(
        token.clone(),
        event.id.clone(),
        "Múltipla".into(),
        "2099-01-01T00:00:00Z".into(),
        "2099-01-02T00:00:00Z".into(),
        1,
        Some(3),
        csrf.clone(),
    )
    .await
    .unwrap();
    let multiple_a = crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        multiple.clone(),
        "X".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    let multiple_b = crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        multiple.clone(),
        "Y".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    let _multiple_c = crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        multiple.clone(),
        "Z".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    crate::custom_events::publish(token.clone(), event.id.clone(), csrf.clone())
        .await
        .unwrap();
    let source = crate::pools::create_pool_for_event(
        token.clone(),
        "Fonte".into(),
        Some(event.id.clone()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let target = crate::pools::create_pool_for_event(
        token.clone(),
        "Destino".into(),
        Some(event.id.clone()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let db = crate::db::pool();
    let source_single = uuid::Uuid::new_v4().to_string();
    let source_numeric = uuid::Uuid::new_v4().to_string();
    let source_multiple = uuid::Uuid::new_v4().to_string();
    for (id, item) in [
        (&source_single, &single),
        (&source_numeric, &numeric),
        (&source_multiple, &multiple),
    ] {
        sqlx::query("INSERT INTO predictions(id,pool_id,user_id,item_id) VALUES(?1,?2,?3,?4)")
            .bind(id)
            .bind(&source.id)
            .bind(&user)
            .bind(item)
            .execute(db)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO custom_prediction_values(prediction_id,option_id) VALUES(?1,?2)")
        .bind(&source_single)
        .bind(&option_a)
        .execute(db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO numeric_prediction_values(prediction_id,value_scaled) VALUES(?1,1234)",
    )
    .bind(&source_numeric)
    .execute(db)
    .await
    .unwrap();
    for option in [&multiple_a, &multiple_b] {
        sqlx::query(
            "INSERT INTO multiple_choice_prediction_options(prediction_id,option_id) VALUES(?1,?2)",
        )
        .bind(&source_multiple)
        .bind(option)
        .execute(db)
        .await
        .unwrap();
    }

    let suggestion = crate::prediction_reuse::suggestion(token.clone(), target.id.clone())
        .await
        .unwrap();
    assert!(suggestion.available);
    assert_eq!(suggestion.source_pool.unwrap().name, "Fonte");
    assert_eq!(suggestion.answered, 3);
    assert_eq!(suggestion.copyable, 3);
    let copied = crate::prediction_reuse::copy(token.clone(), target.id.clone(), csrf.clone())
        .await
        .unwrap();
    assert_eq!(copied.copied_count, 3);
    assert!(!copied.already_initialized);
    assert!(
        !crate::prediction_reuse::suggestion(token.clone(), target.id.clone())
            .await
            .unwrap()
            .available
    );

    let target_single: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND item_id=?2")
            .bind(&target.id)
            .bind(&single)
            .fetch_one(db)
            .await
            .unwrap();
    let copied_option: (String,) =
        sqlx::query_as("SELECT option_id FROM custom_prediction_values WHERE prediction_id=?1")
            .bind(&target_single.0)
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!(copied_option.0, option_a);
    let copied_numeric: (i64,) = sqlx::query_as("SELECT value_scaled FROM numeric_prediction_values v JOIN predictions p ON p.id=v.prediction_id WHERE p.pool_id=?1 AND p.item_id=?2").bind(&target.id).bind(&numeric).fetch_one(db).await.unwrap();
    assert_eq!(copied_numeric.0, 1234);
    let copied_multiple: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM multiple_choice_prediction_options v JOIN predictions p ON p.id=v.prediction_id WHERE p.pool_id=?1 AND p.item_id=?2").bind(&target.id).bind(&multiple).fetch_one(db).await.unwrap();
    assert_eq!(copied_multiple.0, 2);
    let empty_target = crate::pools::create_pool_for_event(
        token.clone(),
        "Destino vazio".into(),
        Some(event.id.clone()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let started_empty =
        crate::prediction_reuse::start_empty(token.clone(), empty_target.id.clone(), csrf.clone())
            .await
            .unwrap();
    assert!(!started_empty.already_initialized);
    let empty_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1")
        .bind(&empty_target.id)
        .fetch_one(db)
        .await
        .unwrap();
    assert_eq!(empty_count.0, 0);
    assert!(
        !crate::prediction_reuse::suggestion(token.clone(), empty_target.id)
            .await
            .unwrap()
            .available
    );
    crate::custom_questions::submit_single_choice_prediction(
        token,
        target.id.clone(),
        single.clone(),
        option_b,
        csrf,
    )
    .await
    .unwrap();
    let source_option: (String,) =
        sqlx::query_as("SELECT option_id FROM custom_prediction_values WHERE prediction_id=?1")
            .bind(&source_single)
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!(
        source_option.0, option_a,
        "editar destino não altera a fonte"
    );
    let decision: (String,) = sqlx::query_as(
        "SELECT prediction_reuse_decision FROM pool_members WHERE pool_id=?1 AND user_id=?2",
    )
    .bind(&target.id)
    .bind(&user)
    .fetch_one(db)
    .await
    .unwrap();
    assert_eq!(decision.0, "copied");
}
