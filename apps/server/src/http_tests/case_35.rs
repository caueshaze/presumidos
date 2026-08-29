use super::*;

/// Regra de privacidade: os palpites de um membro só ficam visíveis depois que
/// a partida começa (kickoff <= agora). Jogos no futuro não podem vazar.
#[tokio::test]
async fn pool_member_predictions_hides_matches_before_kickoff() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email_a = format!("memberA-{suffix}@teste.com");
    let email_b = format!("memberB-{suffix}@teste.com");
    let email_c = format!("outsider-{suffix}@teste.com");
    let email_d = format!("late-member-{suffix}@teste.com");
    let user_a = seed_user(
        &format!("memberA-{suffix}"),
        &email_a,
        "senha-correta-123",
        false,
    )
    .await;
    let user_b = seed_user(
        &format!("memberB-{suffix}"),
        &email_b,
        "senha-correta-123",
        false,
    )
    .await;
    let user_c = seed_user(
        &format!("outsider-{suffix}"),
        &email_c,
        "senha-correta-123",
        false,
    )
    .await;
    let user_d = seed_user(
        &format!("late-member-{suffix}"),
        &email_d,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_a).await;
    // Entraram no bolão antes do jogo "passado", para isolar o teste da regra de
    // elegibilidade por data de entrada (coberta em outro teste).
    add_membership_at(&pool_id, &user_a, "2019-01-01 00:00:00").await;
    add_membership_at(&pool_id, &user_b, "2019-01-01 00:00:00").await;
    // Entrou depois do jogo passado: sua participação não é retroativa, nem
    // pode vazar para os demais membros na lista de palpites.
    add_membership_at(&pool_id, &user_d, "2021-01-01 00:00:00").await;

    let past_match = insert_match("Brasil", "Argentina", "2020-01-01T00:00:00Z").await;
    let future_match = insert_match("Franca", "Espanha", "2999-01-01T00:00:00Z").await;

    // O membro B palpitou nos dois jogos (um já iniciado, um no futuro).
    insert_prediction(&user_b, &past_match, 2, 1).await;
    insert_prediction(&user_b, &future_match, 0, 0).await;
    insert_prediction(&user_d, &past_match, 1, 0).await;
    // Fixture arquitetural: reveal vem do item genérico, não do kickoff.
    sqlx::query(
        "UPDATE prediction_items SET reveal_at = '2020-01-01T00:00:00Z'
         WHERE id = (SELECT prediction_item_id FROM matches WHERE id = ?1)",
    )
    .bind(&future_match)
    .execute(crate::db::pool())
    .await
    .expect("forçar reveal arquitetural");

    // Membro A consulta os palpites do bolão (sessão semeada, sem login).
    let (token_a, _) = seed_session(&user_a).await;
    let viewer = client_with_session(base, &token_a);
    let response = viewer
        .get(format!("{base}/api/pools/{pool_id}/member-predictions"))
        .send()
        .await
        .expect("requisicao member-predictions");
    assert!(
        response.status().is_success(),
        "membro deveria poder consultar"
    );

    let members: Vec<crate::models::MemberPredictions> =
        response.json().await.expect("corpo member-predictions");
    let b = members
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("membro B presente na resposta");

    // Apenas o palpite do jogo já iniciado deve aparecer.
    assert_eq!(
        b.predictions.len(),
        2,
        "reveal_at do item deve liberar o palpite mesmo se o kickoff for futuro"
    );
    assert!(b
        .predictions
        .iter()
        .any(|prediction| prediction.match_id == past_match));
    assert!(b
        .predictions
        .iter()
        .any(|prediction| prediction.match_id == future_match));

    let late_member = members
        .iter()
        .find(|m| m.user_id == user_d)
        .expect("membro que entrou depois presente na resposta");
    assert!(
        late_member.predictions.is_empty(),
        "palpite de jogo anterior a entrada não deve ser exposto retroativamente"
    );

    // Quem não é membro do bolão é barrado.
    let (token_c, _) = seed_session(&user_c).await;
    let outsider = client_with_session(base, &token_c);
    let denied = outsider
        .get(format!("{base}/api/pools/{pool_id}/member-predictions"))
        .send()
        .await
        .expect("requisicao de nao-membro");
    assert!(
        !denied.status().is_success(),
        "nao-membro nao deveria acessar"
    );
}
