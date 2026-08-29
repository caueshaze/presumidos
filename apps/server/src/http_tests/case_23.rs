use super::*;

#[tokio::test]
async fn logout_requires_valid_csrf_token() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("logout-{suffix}@teste.com");
    seed_user(
        &format!("logout-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;

    let client = client();

    let login_response = login(&client, base, &email, "senha-correta-123").await;
    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");

    let bad_logout = client
        .post(format!("{base}/api/auth/logout"))
        .header("X-CSRF-Token", "token-errado")
        .send()
        .await
        .expect("requisicao de logout com csrf invalido");
    assert!(!bad_logout.status().is_success());
    let error: ErrorPayload = bad_logout.json().await.expect("corpo de erro");
    assert!(error.error.to_lowercase().contains("seguranca"));

    let still_logged_in = client
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("requisicao current_user");
    let session: SessionState = still_logged_in.json().await.expect("corpo de current_user");
    assert!(session.user.is_some(), "csrf invalido nao deveria deslogar");

    let good_logout = client
        .post(format!("{base}/api/auth/logout"))
        .header("X-CSRF-Token", auth_result.csrf_token)
        .send()
        .await
        .expect("requisicao de logout com csrf valido");
    assert!(good_logout.status().is_success());

    let logged_out = client
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("requisicao current_user apos logout");
    let session: SessionState = logged_out.json().await.expect("corpo de current_user");
    assert!(session.user.is_none(), "sessao deveria estar encerrada");
}
