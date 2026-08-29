use super::*;

/// Elegibilidade por data de entrada: palpites de jogos que começaram ANTES de
/// o usuário entrar no bolão não pontuam (sem retroatividade). Palpites de jogos
/// que começaram depois da entrada contam normalmente.
#[tokio::test]
async fn leaderboard_ignores_predictions_from_before_join() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("joiner-{suffix}@teste.com");
    let user_id = seed_user(
        &format!("joiner-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_id).await;
    // Entrou no bolão em 2022.
    add_membership_at(&pool_id, &user_id, "2022-01-01 00:00:00").await;

    // Jogo anterior à entrada (2020): palpite exato valeria 7, mas NÃO deve contar.
    let old_match =
        insert_finished_match("Brasil", "Argentina", "2020-01-01T00:00:00Z", 2, 1).await;
    insert_prediction(&user_id, &old_match, 2, 1).await;

    // Jogo posterior à entrada (2023): palpite exato vale 7 e DEVE contar.
    let new_match = insert_finished_match("Franca", "Espanha", "2023-01-01T00:00:00Z", 1, 0).await;
    insert_prediction(&user_id, &new_match, 1, 0).await;

    // A suíte inteira pode deixar breakdowns materializados de testes anteriores.
    // Recalcula explicitamente para garantir isolamento deste caso.
    crate::scoring::recalculate_all_breakdowns(None)
        .await
        .expect("recalcular breakdowns");

    let (token, _csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    // Só o jogo posterior à entrada pontua: 7 (e não 14).
    assert_eq!(
        leaderboard_points(&client, base, &pool_id, &user_id).await,
        7,
        "apenas o palpite do jogo posterior a entrada deve pontuar"
    );
}
