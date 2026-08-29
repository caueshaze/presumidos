use super::*;

#[tokio::test]
async fn delete_account_blocks_pool_owner() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("pool-owner-{suffix}@teste.com");
    let user_id = seed_user(
        &format!("pool-owner-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;
    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_id).await;
    add_membership(&pool_id, &user_id).await;

    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    let blocked = client
        .post(format!("{base}/api/auth/delete"))
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .expect("exclusao bloqueada");
    assert!(
        !blocked.status().is_success(),
        "criador de bolao nao deveria excluir a conta"
    );
    let err: ErrorPayload = blocked.json().await.expect("erro da exclusao bloqueada");
    assert!(err.error.to_lowercase().contains("criou bol"));

    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("usuario ainda existe");
    assert_eq!(user_count.0, 1);
}
