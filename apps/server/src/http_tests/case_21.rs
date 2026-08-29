use super::*;

#[tokio::test]
async fn login_sets_session_cookie_and_current_user_works() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("login-{suffix}@teste.com");
    seed_user(
        &format!("login-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;

    let client = client();

    let login_response = login(&client, base, &email, "senha-correta-123").await;
    assert!(
        login_response.status().is_success(),
        "login deveria ter sucesso"
    );

    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");
    assert_eq!(auth_result.user.email, email);
    assert!(!auth_result.csrf_token.is_empty());

    let current_response = client
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("requisicao current_user");
    assert!(current_response.status().is_success());

    let session: SessionState = current_response
        .json()
        .await
        .expect("corpo de current_user");
    let user = session.user.expect("sessao deveria ter usuario");
    assert_eq!(user.email, email);
    assert_eq!(session.csrf_token, auth_result.csrf_token);
}
