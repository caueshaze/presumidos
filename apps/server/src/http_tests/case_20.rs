use super::*;

#[tokio::test]
async fn prediction_reuse_copies_football_into_only_the_selected_pool() {
    test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = seed_user(
        &format!("reuse-football-{suffix}"),
        &format!("reuse-football-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let source = insert_pool(&format!("Fonte futebol {suffix}"), &user).await;
    let target = insert_pool(&format!("Destino futebol {suffix}"), &user).await;
    add_membership(&source, &user).await;
    add_membership(&target, &user).await;
    let match_id = insert_match("Brasil", "Alemanha", "2999-01-01T00:00:00Z").await;
    let (token, csrf) = seed_session(&user).await;
    crate::matches::submit_prediction(
        token.clone(),
        source.clone(),
        match_id.clone(),
        2,
        1,
        crate::models::KnockoutEntry::default(),
        csrf.clone(),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM predictions WHERE pool_id=?1")
            .bind(&target)
            .fetch_one(crate::db::pool())
            .await
            .unwrap()
            .0,
        0,
        "um palpite normal de futebol não vaza para outro Pool",
    );
    let reused = crate::prediction_reuse::copy(token.clone(), target.clone(), csrf.clone())
        .await
        .unwrap();
    assert_eq!(reused.copied_count, 1);
    crate::matches::submit_prediction(
        token,
        target.clone(),
        match_id.clone(),
        4,
        0,
        crate::models::KnockoutEntry::default(),
        csrf,
    )
    .await
    .unwrap();
    let source_score: (i64, i64) = sqlx::query_as(
        "SELECT home_score,away_score FROM predictions WHERE pool_id=?1 AND match_id=?2",
    )
    .bind(&source)
    .bind(&match_id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    let target_score: (i64, i64) = sqlx::query_as(
        "SELECT home_score,away_score FROM predictions WHERE pool_id=?1 AND match_id=?2",
    )
    .bind(&target)
    .bind(&match_id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(source_score, (2, 1));
    assert_eq!(target_score, (4, 0));
}
