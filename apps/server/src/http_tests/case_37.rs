use super::*;

#[tokio::test]
async fn prediction_reactions_reject_self_reaction_and_non_member() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email_a = format!("self-react-{suffix}@teste.com");
    let email_b = format!("other-react-{suffix}@teste.com");
    let email_c = format!("outsider-react-{suffix}@teste.com");
    let user_a = seed_user(
        &format!("self-react-{suffix}"),
        &email_a,
        "senha-correta-123",
        false,
    )
    .await;
    let user_b = seed_user(
        &format!("other-react-{suffix}"),
        &email_b,
        "senha-correta-123",
        false,
    )
    .await;
    let user_c = seed_user(
        &format!("outsider-react-{suffix}"),
        &email_c,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao React Guard {suffix}"), &user_a).await;
    add_membership_at(&pool_id, &user_a, "2019-01-01 00:00:00").await;
    add_membership_at(&pool_id, &user_b, "2019-01-01 00:00:00").await;
    let match_id = insert_match("Italia", "Alemanha", "2020-01-01T00:00:00Z").await;
    insert_prediction(&user_a, &match_id, 0, 0).await;

    let react_url = format!("{base}/api/pools/{pool_id}/prediction-reactions");

    let (token_a, csrf_a) = seed_session(&user_a).await;
    let self_client = client_with_session(base, &token_a);
    let self_reaction = self_client
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_a, "matchId": match_id, "emoji": "😂" }))
        .send()
        .await
        .expect("auto-reacao");
    assert!(!self_reaction.status().is_success());

    let (token_c, csrf_c) = seed_session(&user_c).await;
    let outsider = client_with_session(base, &token_c);
    let outsider_reaction = outsider
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_c)
        .json(&json!({ "targetUserId": user_a, "matchId": match_id, "emoji": "😂" }))
        .send()
        .await
        .expect("reacao por nao-membro");
    assert!(!outsider_reaction.status().is_success());
}
