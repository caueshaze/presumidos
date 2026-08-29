use super::*;

/// Troca de nome de usuário: aplica o novo nome, mas rejeita um nome já em uso
/// por outra conta (case-insensitive).
#[tokio::test]
async fn change_username_updates_and_rejects_duplicates() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    // Sufixo curto: o endpoint limita o nome a 32 caracteres (um UUID já tem 36).
    let short = suffix.simple().to_string();
    let short = &short[..8];
    let email = format!("rename-{suffix}@teste.com");
    let other_email = format!("other-{suffix}@teste.com");
    let user_id = seed_user(
        &format!("rename-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;
    let taken_name = format!("taken{short}");
    seed_user(&taken_name, &other_email, "senha-correta-123", false).await;

    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);
    let url = format!("{base}/api/auth/username");

    // Nome novo e livre: sucesso, e a sessão passa a refletir o novo nome.
    let new_name = format!("novo{short}");
    let ok = client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "username": new_name }))
        .send()
        .await
        .expect("trocar nome");
    assert!(
        ok.status().is_success(),
        "troca de nome deveria ter sucesso"
    );
    let updated: crate::models::UserPublic = ok.json().await.expect("usuario atualizado");
    assert_eq!(updated.username, new_name);

    let stored: (String,) = sqlx::query_as("SELECT username FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("nome no banco");
    assert_eq!(stored.0, new_name);

    // Nome já usado por outra conta (variando maiúsc./minúsc.): rejeitado.
    let dup = client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "username": taken_name.to_uppercase() }))
        .send()
        .await
        .expect("trocar para nome ocupado");
    assert!(
        !dup.status().is_success(),
        "nome em uso deveria ser rejeitado"
    );
    let err: ErrorPayload = dup.json().await.expect("corpo de erro");
    assert!(err.error.to_lowercase().contains("ja esta em uso"));

    // O nome no banco não mudou após a tentativa rejeitada.
    let unchanged: (String,) = sqlx::query_as("SELECT username FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("nome no banco apos rejeicao");
    assert_eq!(unchanged.0, new_name);
}
