use super::*;

#[tokio::test]
async fn football_scoring_is_persisted_and_isolated_per_pool() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user = seed_user(
        &format!("scoring-{suffix}"),
        &format!("scoring-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let pool_a = insert_pool(&format!("A-{suffix}"), &user).await;
    let pool_b = insert_pool(&format!("B-{suffix}"), &user).await;
    add_membership(&pool_a, &user).await;
    add_membership(&pool_b, &user).await;
    let default:(i64,i64,i64,i64,i64)=sqlx::query_as("SELECT exact_score_points,correct_result_exact_side_points,correct_result_points,incorrect_result_points,knockout_bonus_points FROM football_pool_scoring WHERE pool_id=?1").bind(&pool_a).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!(default, (7, 4, 3, 0, 3));
    sqlx::query("UPDATE football_pool_scoring SET exact_score_points=15 WHERE pool_id=?1")
        .bind(&pool_b)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let match_id = insert_finished_match("Brasil", "Japao", "2020-01-01T00:00:00Z", 2, 1).await;
    let item: (String,) = sqlx::query_as("SELECT prediction_item_id FROM matches WHERE id=?1")
        .bind(&match_id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    for pool in [&pool_a, &pool_b] {
        sqlx::query("INSERT INTO predictions (id,pool_id,user_id,item_id,match_id,home_score,away_score) VALUES (?1,?2,?3,?4,?5,2,1)").bind(uuid::Uuid::new_v4().to_string()).bind(pool).bind(&user).bind(&item.0).bind(&match_id).execute(crate::db::pool()).await.unwrap();
    }
    crate::scoring::recalculate_all_breakdowns(None)
        .await
        .unwrap();
    let a:(i64,)=sqlx::query_as("SELECT total_points FROM prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND match_id=?3").bind(&pool_a).bind(&user).bind(&match_id).fetch_one(crate::db::pool()).await.unwrap();
    let b:(i64,)=sqlx::query_as("SELECT total_points FROM prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND match_id=?3").bind(&pool_b).bind(&user).bind(&match_id).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!((a.0, b.0), (7, 15));
    let (token, _csrf) = seed_session(&user).await;
    let client = client_with_session(base, &token);
    let read: crate::models::FootballScoringConfig = client
        .get(format!("{base}/api/pools/{pool_a}/scoring/football"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(read.exact_score_points, 7);
}
