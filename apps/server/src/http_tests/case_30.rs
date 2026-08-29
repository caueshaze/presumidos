use super::*;

/// Ajuste manual de pontos: criador e admin podem lançar/remover, o total reflete
/// no ranking, membro comum é barrado para lançar mas vê os ajustes (transparência).
#[tokio::test]
async fn pool_creator_and_admin_can_adjust_points() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let creator_email = format!("creator-{suffix}@teste.com");
    let target_email = format!("target-adj-{suffix}@teste.com");
    let admin_email = format!("admin-adj-{suffix}@teste.com");
    let outsider_email = format!("outsider-adj-{suffix}@teste.com");
    let creator_id = seed_user(
        &format!("creator-{suffix}"),
        &creator_email,
        "senha-correta-123",
        false,
    )
    .await;
    let target_id = seed_user(
        &format!("targetadj-{suffix}"),
        &target_email,
        "senha-correta-123",
        false,
    )
    .await;
    let admin_id = seed_user(
        &format!("adminadj-{suffix}"),
        &admin_email,
        "senha-correta-123",
        true,
    )
    .await;
    let outsider_id = seed_user(
        &format!("outadj-{suffix}"),
        &outsider_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &creator_id).await;
    add_membership(&pool_id, &creator_id).await;
    add_membership(&pool_id, &target_id).await;

    let adj_url = format!("{base}/api/pools/{pool_id}/adjustments");

    // Criador lança +5 para o alvo (sessão semeada, sem usar o endpoint de login).
    let (creator_token, creator_csrf) = seed_session(&creator_id).await;
    let creator_c = client_with_session(base, &creator_token);

    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        0
    );

    let added = creator_c
        .post(&adj_url)
        .header("X-CSRF-Token", &creator_csrf)
        .json(&json!({ "userId": target_id, "delta": 5, "reason": "erro de placar" }))
        .send()
        .await
        .expect("lancar ajuste");
    assert!(added.status().is_success(), "criador deveria poder ajustar");
    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        5
    );

    // Lista de ajustes (criador, membro) tem 1 item.
    let list: Vec<crate::models::PointAdjustment> = creator_c
        .get(&adj_url)
        .send()
        .await
        .expect("listar ajustes")
        .json()
        .await
        .expect("corpo ajustes");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].delta, 5);
    let adjustment_id = list[0].id.clone();

    // Membro comum não pode lançar, mas enxerga os ajustes (transparência).
    let (target_token, target_csrf) = seed_session(&target_id).await;
    let target_c = client_with_session(base, &target_token);
    let denied = target_c
        .post(&adj_url)
        .header("X-CSRF-Token", &target_csrf)
        .json(&json!({ "userId": target_id, "delta": 99, "reason": "trapaca" }))
        .send()
        .await
        .expect("ajuste por membro comum");
    assert!(
        !denied.status().is_success(),
        "membro comum nao deveria ajustar"
    );
    let seen: Vec<crate::models::PointAdjustment> = target_c
        .get(&adj_url)
        .send()
        .await
        .expect("membro lista ajustes")
        .json()
        .await
        .expect("corpo ajustes membro");
    assert_eq!(seen.len(), 1, "membro deveria ver o ajuste (transparencia)");

    // Admin global ajusta um bolão que não criou: +2.
    let (admin_token, admin_csrf) = seed_session(&admin_id).await;
    let admin_c = client_with_session(base, &admin_token);
    let admin_added = admin_c
        .post(&adj_url)
        .header("X-CSRF-Token", &admin_csrf)
        .json(&json!({ "userId": target_id, "delta": 2, "reason": "bonus admin" }))
        .send()
        .await
        .expect("ajuste do admin");
    assert!(
        admin_added.status().is_success(),
        "admin deveria poder ajustar"
    );
    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        7
    );

    // Criador remove o ajuste de +5: total volta a 2.
    let removed = creator_c
        .post(format!("{base}/api/pools/{pool_id}/adjustments/remove"))
        .header("X-CSRF-Token", &creator_csrf)
        .json(&json!({ "adjustmentId": adjustment_id }))
        .send()
        .await
        .expect("remover ajuste");
    assert!(
        removed.status().is_success(),
        "criador deveria poder remover"
    );
    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        2
    );

    // Não-membro é barrado ao listar ajustes.
    let (outsider_token, _) = seed_session(&outsider_id).await;
    let outsider_c = client_with_session(base, &outsider_token);
    let outsider_list = outsider_c
        .get(&adj_url)
        .send()
        .await
        .expect("nao-membro lista ajustes");
    assert!(
        !outsider_list.status().is_success(),
        "nao-membro nao deveria listar"
    );
}
