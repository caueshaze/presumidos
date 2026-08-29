use super::*;

#[tokio::test]
async fn login_rehashes_password_with_outdated_parameters() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("rehash-{suffix}@teste.com");
    let weak_hash = weak_password_hash("senha-correta-123");
    let user_id = crate::auth::insert_user_account(
        crate::db::pool(),
        &format!("rehash-{suffix}"),
        &email,
        &weak_hash,
        false,
    )
    .await
    .expect("inserir usuario com hash fraco");

    let client = client();

    // Senha errada nao deve alterar o hash armazenado.
    let wrong_password = login(&client, base, &email, "senha-incorreta").await;
    assert!(!wrong_password.status().is_success());

    let hash_after_wrong_password: (String,) =
        sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("hash do usuario");
    assert_eq!(hash_after_wrong_password.0, weak_hash);

    // Senha correta com hash desatualizado deve disparar rehash transparente.
    let login_response = login(&client, base, &email, "senha-correta-123").await;
    assert!(login_response.status().is_success());

    let hash_after_login: (String,) =
        sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("hash do usuario");
    assert_ne!(hash_after_login.0, weak_hash);

    let parsed = argon2::password_hash::PasswordHash::new(&hash_after_login.0)
        .expect("hash novo deve ser valido");
    let cfg = crate::config::settings();
    let params = argon2::Params::try_from(&parsed).expect("params do hash novo");
    assert_eq!(params.m_cost(), cfg.argon2_memory_kib);
    assert_eq!(params.t_cost(), cfg.argon2_time_cost);
    assert_eq!(params.p_cost(), cfg.argon2_parallelism);
}
