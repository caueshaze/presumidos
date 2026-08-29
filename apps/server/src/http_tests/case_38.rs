use super::*;

#[tokio::test]
async fn admin_can_add_and_remove_pool_members() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin_email = format!("admin-mgmt-{suffix}@teste.com");
    let target_email = format!("target-{suffix}@teste.com");
    let admin_id = seed_user(
        &format!("admin-mgmt-{suffix}"),
        &admin_email,
        "senha-correta-123",
        true,
    )
    .await;
    let target_id = seed_user(
        &format!("target-{suffix}"),
        &target_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao Admin {suffix}"), &admin_id).await;

    let (admin_token, csrf) = seed_session(&admin_id).await;
    let admin = client_with_session(base, &admin_token);
    let add_url = format!("{base}/api/admin/pools/{pool_id}/members");
    let members_url = add_url.clone();

    // Sem reautenticação recente, a ação é bloqueada.
    let needs_reauth = admin
        .post(&add_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "userId": target_id }))
        .send()
        .await
        .expect("add sem reauth");
    assert_eq!(needs_reauth.status().as_u16(), 403);
    let err: ErrorPayload = needs_reauth.json().await.expect("corpo de erro");
    assert!(
        err.error.contains("ADMIN_REAUTH_REQUIRED"),
        "esperava exigencia de reauth, recebeu: {}",
        err.error
    );

    // Marca a sessão como reautenticada recentemente (sem passar pelo endpoint
    // de reauth, para não interferir no rate limit compartilhado dos testes).
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE user_id = ?1")
        .bind(&admin_id)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth recente");

    // Adiciona o usuário ao bolão.
    let added = admin
        .post(&add_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "userId": target_id }))
        .send()
        .await
        .expect("add membro");
    assert!(added.status().is_success(), "add deveria ter sucesso");

    // A listagem de membros passa a conter o alvo.
    let listed: Vec<crate::models::UserPublic> = admin
        .get(&members_url)
        .send()
        .await
        .expect("listar membros")
        .json()
        .await
        .expect("corpo de membros");
    assert!(
        listed.iter().any(|u| u.id == target_id),
        "alvo deveria estar nos membros"
    );

    // Remove o usuário do bolão.
    let removed = admin
        .post(format!("{base}/api/admin/pools/{pool_id}/members/remove"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "userId": target_id }))
        .send()
        .await
        .expect("remover membro");
    assert!(removed.status().is_success(), "remove deveria ter sucesso");

    let after: Vec<crate::models::UserPublic> = admin
        .get(&members_url)
        .send()
        .await
        .expect("listar membros apos remocao")
        .json()
        .await
        .expect("corpo de membros 2");
    assert!(
        !after.iter().any(|u| u.id == target_id),
        "alvo deveria ter sido removido"
    );

    // Usuário comum não pode gerenciar membros.
    let (normal_token, normal_csrf) = seed_session(&target_id).await;
    let normal = client_with_session(base, &normal_token);
    let denied = normal
        .post(&add_url)
        .header("X-CSRF-Token", &normal_csrf)
        .json(&json!({ "userId": admin_id }))
        .send()
        .await
        .expect("add por nao-admin");
    assert!(
        !denied.status().is_success(),
        "usuario comum nao deveria poder gerenciar membros"
    );
}
