use super::*;

#[tokio::test]
async fn creating_a_pool_requires_an_explicit_active_event() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user_id = seed_user(
        &format!("event-owner-{suffix}"),
        &format!("event-owner-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    let response = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "name": format!("Bolao Evento {suffix}") }))
        .send()
        .await
        .expect("tentar criar sem evento");
    assert!(!response.status().is_success());
    let payload: ErrorPayload = response.json().await.expect("erro de evento obrigatório");
    assert!(payload.error.contains("Escolha um evento publicado"));
}
