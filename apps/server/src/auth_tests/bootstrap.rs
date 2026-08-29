use super::super::{
    count_admins, create_bootstrap_admin_account_with_expected_secret, create_public_user_account,
    validate_registration_input,
};
use super::{seed_security_env, test_db};

#[tokio::test]
async fn public_registration_flow_never_creates_admin() {
    seed_security_env();
    let db = test_db().await;
    let (username, username_lookup, email) = validate_registration_input(
        "Caue".to_string(),
        "caue@teste.com".to_string(),
        "senha-super-segura",
    )
    .expect("input should validate");

    let user_id = create_public_user_account(
        &db,
        &username,
        &username_lookup,
        &email,
        "senha-super-segura",
    )
    .await
    .expect("public registration should work");

    let row: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(&db)
        .await
        .expect("user should exist");

    assert!(!row.0);
    assert_eq!(count_admins(&db).await.expect("count admins"), 0);
}

#[tokio::test]
async fn bootstrap_admin_creates_first_admin_and_blocks_second_one() {
    seed_security_env();
    let db = test_db().await;
    let (username, username_lookup, email) = validate_registration_input(
        "Root".to_string(),
        "root@teste.com".to_string(),
        "senha-super-segura",
    )
    .expect("input should validate");

    let user_id = create_bootstrap_admin_account_with_expected_secret(
        &db,
        &username,
        &username_lookup,
        &email,
        "senha-super-segura",
        "bootstrap-secret-super-seguro-0123456789abcdef",
        "bootstrap-secret-super-seguro-0123456789abcdef",
        "127.0.0.1",
    )
    .await
    .expect("bootstrap should create first admin");

    let row: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(&db)
        .await
        .expect("admin should exist");
    assert!(row.0);
    assert_eq!(count_admins(&db).await.expect("count admins"), 1);

    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'bootstrap_admin_created_explicit'",
    )
    .fetch_one(&db)
    .await
    .expect("audit should exist");
    assert_eq!(audit_count.0, 1);

    let second = create_bootstrap_admin_account_with_expected_secret(
        &db,
        "Outro",
        "outro",
        "outro@teste.com",
        "senha-super-segura",
        "bootstrap-secret-super-seguro-0123456789abcdef",
        "bootstrap-secret-super-seguro-0123456789abcdef",
        "127.0.0.1",
    )
    .await;
    assert!(second.is_err());

    let blocked_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'bootstrap_admin_blocked_existing_admin'",
    )
    .fetch_one(&db)
    .await
    .expect("blocked audit should exist");
    assert_eq!(blocked_count.0, 1);
}

#[tokio::test]
async fn bootstrap_admin_invalid_secret_is_audited_without_creating_admin() {
    seed_security_env();
    let db = test_db().await;

    let attempt = create_bootstrap_admin_account_with_expected_secret(
        &db,
        "Root",
        "root",
        "root@teste.com",
        "senha-super-segura",
        "segredo-incorreto",
        "bootstrap-secret-super-seguro-0123456789abcdef",
        "127.0.0.1",
    )
    .await;
    assert!(attempt.is_err());

    assert_eq!(count_admins(&db).await.expect("count admins"), 0);

    let failed_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'bootstrap_admin_failed_invalid_secret'",
    )
    .fetch_one(&db)
    .await
    .expect("failed audit should exist");
    assert_eq!(failed_count.0, 1);
}
