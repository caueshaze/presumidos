use super::*;

#[tokio::test]
async fn admin_reauth_flow_and_rate_limit() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("admin-{suffix}@teste.com");
    let user_id = seed_user(
        &format!("admin-{suffix}"),
        &email,
        "senha-correta-123",
        true,
    )
    .await;

    let client = client();

    let login_response = login(&client, base, &email, "senha-correta-123").await;
    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");
    let csrf = auth_result.csrf_token;
    let reauth_url = format!("{base}/api/auth/reauth");

    // Senha errada nao altera o estado da sessao.
    let wrong_password = client
        .post(&reauth_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "password": "senha-errada" }))
        .send()
        .await
        .expect("reauth com senha errada");
    assert!(!wrong_password.status().is_success());
    let error: ErrorPayload = wrong_password.json().await.expect("corpo de erro");
    assert!(error
        .error
        .to_lowercase()
        .contains("senha de administrador"));

    let admin_reauthed_after_failure: (Option<String>,) =
        sqlx::query_as("SELECT admin_reauthed_at FROM sessions WHERE user_id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("sessao do admin");
    assert!(admin_reauthed_after_failure.0.is_none());

    // Senha correta confirma a reautenticacao recente.
    let right_password = client
        .post(&reauth_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "password": "senha-correta-123" }))
        .send()
        .await
        .expect("reauth com senha correta");
    assert!(right_password.status().is_success());

    let admin_reauthed_after_success: (Option<String>,) =
        sqlx::query_as("SELECT admin_reauthed_at FROM sessions WHERE user_id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("sessao do admin");
    assert!(admin_reauthed_after_success.0.is_some());

    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'admin_reauthenticated' AND actor_user_id = ?1",
    )
    .bind(&user_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("audit log");
    assert_eq!(audit_count.0, 1);

    // Ja foram feitas 2 chamadas (errada + correta) nesta janela. O limite por
    // IP e de 8 tentativas/min, entao mais 6 chamadas com senha errada devem
    // estourar o limite na setima.
    let mut last_message = String::new();
    for _ in 0..6 {
        let response = client
            .post(&reauth_url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({ "password": "senha-errada" }))
            .send()
            .await
            .expect("reauth repetido");
        assert!(!response.status().is_success());
        let error: ErrorPayload = response.json().await.expect("corpo de erro");
        last_message = error.error;
    }
    assert!(
        last_message.to_lowercase().contains("muitas tentativas"),
        "esperava erro de rate limit, recebeu: {last_message}"
    );
}
