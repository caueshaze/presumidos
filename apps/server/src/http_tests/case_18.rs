use super::*;

/// O lock é aplicado pelo domínio ao gravar (e não apenas ocultado pela UI):
/// antes dele o palpite pode ser criado e atualizado; depois, nem a criação
/// nem a alteração passam sem uma reabertura administrativa explícita.
#[tokio::test]
async fn predictions_can_change_before_lock_and_are_rejected_after_lock() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user_id = seed_user(
        &format!("prediction-lock-{suffix}"),
        &format!("prediction-lock-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let pool_id = insert_pool(&format!("prediction-lock-pool-{suffix}"), &user_id).await;
    add_membership(&pool_id, &user_id).await;
    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    let future_match = insert_match("Brasil", "Japao", "2999-01-01T00:00:00Z").await;
    let prediction_url = format!("{base}/api/predictions");
    for (home_score, away_score) in [(2, 1), (3, 1)] {
        let response = client
            .post(&prediction_url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({
                "poolId": pool_id,
                "matchId": future_match,
                "homeScore": home_score,
                "awayScore": away_score,
            }))
            .send()
            .await
            .expect("enviar palpite antes do lock");
        assert_eq!(response.status().as_u16(), 204);
    }
    let future_stored: (i64, i64) = sqlx::query_as(
        "SELECT home_score, away_score FROM predictions WHERE user_id = ?1 AND match_id = ?2",
    )
    .bind(&user_id)
    .bind(&future_match)
    .fetch_one(crate::db::pool())
    .await
    .expect("ler palpite atualizado antes do lock");
    assert_eq!(future_stored, (3, 1));

    // Fixture arquitetural: football de produção mantém lock_at == kickoff,
    // mas esta divergência prova que a regra consulta o item genérico.
    sqlx::query(
        "UPDATE prediction_items SET lock_at = '2020-01-01T00:00:00Z'
         WHERE id = (SELECT prediction_item_id FROM matches WHERE id = ?1)",
    )
    .bind(&future_match)
    .execute(crate::db::pool())
    .await
    .expect("forçar lock arquitetural");
    let item_locked = client
        .post(&prediction_url)
        .header("X-CSRF-Token", &csrf)
        .json(
            &json!({ "poolId": pool_id, "matchId": future_match, "homeScore": 4, "awayScore": 1 }),
        )
        .send()
        .await
        .expect("palpite contra lock do item");
    assert!(
        !item_locked.status().is_success(),
        "lock_at passado deve bloquear mesmo com kickoff futuro"
    );

    let locked_match = insert_match("Franca", "Espanha", "2020-01-01T00:00:00Z").await;
    let rejected_new = client
        .post(&prediction_url)
        .header("X-CSRF-Token", &csrf)
        .json(
            &json!({ "poolId": pool_id, "matchId": locked_match, "homeScore": 1, "awayScore": 0 }),
        )
        .send()
        .await
        .expect("tentar criar palpite travado");
    assert!(!rejected_new.status().is_success());

    insert_prediction(&user_id, &locked_match, 0, 0).await;
    let rejected_update = client
        .post(&prediction_url)
        .header("X-CSRF-Token", &csrf)
        .json(
            &json!({ "poolId": pool_id, "matchId": locked_match, "homeScore": 4, "awayScore": 0 }),
        )
        .send()
        .await
        .expect("tentar alterar palpite travado");
    assert!(!rejected_update.status().is_success());
    let locked_stored: (i64, i64) = sqlx::query_as(
        "SELECT home_score, away_score FROM predictions WHERE user_id = ?1 AND match_id = ?2",
    )
    .bind(&user_id)
    .bind(&locked_match)
    .fetch_one(crate::db::pool())
    .await
    .expect("ler palpite preservado apos lock");
    assert_eq!(locked_stored, (0, 0));
}
