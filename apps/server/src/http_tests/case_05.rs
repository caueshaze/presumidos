use super::*;

#[tokio::test]
async fn prediction_items_backfill_matches_with_world_cup_lock_and_reveal() {
    test_server().await;
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM matches m
             JOIN prediction_items pi ON pi.id = m.prediction_item_id
             JOIN events e ON e.id = pi.event_id
             WHERE e.slug = 'world-cup-2026' AND m.id LIKE 'jogo-%'),
            (SELECT COUNT(*) FROM matches m
             JOIN prediction_items pi ON pi.id = m.prediction_item_id
             JOIN events e ON e.id = pi.event_id
             WHERE m.id LIKE 'jogo-%'
               AND pi.kind = 'football_match'
               AND e.slug = 'world-cup-2026'
               AND pi.lock_at = m.kickoff
               AND pi.reveal_at = m.kickoff)",
    )
    .fetch_one(crate::db::pool())
    .await
    .expect("validar backfill de prediction items");
    assert!(counts.0 > 0, "seed atual deve conter partidas");
    assert_eq!(
        counts.0, counts.1,
        "cada match deve ter um item football da Copa"
    );
}
