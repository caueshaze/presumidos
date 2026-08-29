use super::*;

/// Gestão de membros (admin): adicionar/remover exige admin + reautenticação
/// recente + CSRF; usuário comum é barrado.
#[tokio::test]
async fn prediction_reactions_can_be_created_changed_removed_and_seen() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email_a = format!("reactor-{suffix}@teste.com");
    let email_b = format!("target-{suffix}@teste.com");
    let user_a = seed_user(
        &format!("reactor-{suffix}"),
        &email_a,
        "senha-correta-123",
        false,
    )
    .await;
    let user_b = seed_user(
        &format!("target-{suffix}"),
        &email_b,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao React {suffix}"), &user_a).await;
    add_membership_at(&pool_id, &user_a, "2019-01-01 00:00:00").await;
    add_membership_at(&pool_id, &user_b, "2019-01-01 00:00:00").await;

    let match_id = insert_match("Brasil", "Argentina", "2020-01-01T00:00:00Z").await;
    insert_prediction(&user_b, &match_id, 7, 0).await;

    let (token_a, csrf_a) = seed_session(&user_a).await;
    let reactor = client_with_session(base, &token_a);
    let react_url = format!("{base}/api/pools/{pool_id}/prediction-reactions");

    let created = reactor
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_b, "matchId": match_id, "emoji": "😂" }))
        .send()
        .await
        .expect("criar reacao");
    assert!(created.status().is_success(), "criacao deveria ter sucesso");

    let (token_b, csrf_b) = seed_session(&user_b).await;
    let target = client_with_session(base, &token_b);
    let list_url = format!("{base}/api/pools/{pool_id}/member-predictions");
    let members: Vec<crate::models::MemberPredictions> = target
        .get(&list_url)
        .send()
        .await
        .expect("listar palpites apos reacao")
        .json()
        .await
        .expect("corpo member-predictions");
    let target_entry = members
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("membro alvo presente");
    let prediction = target_entry
        .predictions
        .iter()
        .find(|p| p.match_id == match_id)
        .expect("palpite do alvo presente");
    assert_eq!(target_entry.unread_reaction_count, 1);
    assert_eq!(prediction.unread_reaction_count, 1);
    assert_eq!(prediction.reactions.len(), 1);
    assert_eq!(prediction.reactions[0].emoji, "😂");
    assert_eq!(prediction.reactions[0].count, 1);

    let seen = target
        .post(format!(
            "{base}/api/pools/{pool_id}/prediction-reactions/mark-seen"
        ))
        .header("X-CSRF-Token", &csrf_b)
        .send()
        .await
        .expect("marcar reacoes como vistas");
    assert!(seen.status().is_success(), "mark-seen deveria ter sucesso");

    let after_seen: Vec<crate::models::MemberPredictions> = target
        .get(&list_url)
        .send()
        .await
        .expect("listar apos mark-seen")
        .json()
        .await
        .expect("corpo apos mark-seen");
    let target_after_seen = after_seen
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("alvo apos mark-seen");
    assert_eq!(target_after_seen.unread_reaction_count, 0);

    let changed = reactor
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_b, "matchId": match_id, "emoji": "🔥" }))
        .send()
        .await
        .expect("trocar reacao");
    assert!(changed.status().is_success(), "troca deveria ter sucesso");

    let removed = reactor
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_b, "matchId": match_id, "emoji": "🔥" }))
        .send()
        .await
        .expect("remover reacao");
    assert!(removed.status().is_success(), "remocao deveria ter sucesso");

    let after_remove: Vec<crate::models::MemberPredictions> = target
        .get(&list_url)
        .send()
        .await
        .expect("listar apos remover")
        .json()
        .await
        .expect("corpo apos remover");
    let target_after_remove = after_remove
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("alvo apos remover");
    let prediction_after_remove = target_after_remove
        .predictions
        .iter()
        .find(|p| p.match_id == match_id)
        .expect("palpite apos remover");
    assert!(prediction_after_remove.reactions.is_empty());
}
