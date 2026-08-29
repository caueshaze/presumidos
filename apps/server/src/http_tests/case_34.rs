use super::*;

#[tokio::test]
async fn load_active_subscriptions_includes_admin_accounts() {
    let _base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin_email = format!("admin-push-{suffix}@teste.com");
    let user_email = format!("user-push-{suffix}@teste.com");
    let admin_id = seed_user(
        &format!("admin-push-{suffix}"),
        &admin_email,
        "senha-correta-123",
        true,
    )
    .await;
    let user_id = seed_user(
        &format!("user-push-{suffix}"),
        &user_email,
        "senha-correta-123",
        false,
    )
    .await;

    for (user_id, endpoint) in [
        (&admin_id, format!("https://push.example/admin-{suffix}")),
        (&user_id, format!("https://push.example/user-{suffix}")),
    ] {
        sqlx::query(
            "INSERT INTO notification_preferences (user_id, enabled, lead_time_minutes)
             VALUES (?1, 1, 20)",
        )
        .bind(user_id)
        .execute(crate::db::pool())
        .await
        .expect("preferencia ativa");

        sqlx::query(
            "INSERT INTO push_subscriptions
                (id, user_id, endpoint, p256dh, auth, active)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(endpoint)
        .bind("p256dh-teste")
        .bind("auth-teste")
        .execute(crate::db::pool())
        .await
        .expect("subscription ativa");
    }

    let grouped = crate::push::test_active_subscription_user_ids(crate::db::pool())
        .await
        .expect("subscriptions ativas");

    if cfg!(feature = "web-push") {
        assert!(
            grouped.contains(&admin_id),
            "admin deveria receber notificacoes se ativar push"
        );
        assert!(
            grouped.contains(&user_id),
            "usuario comum deveria seguir recebendo notificacoes"
        );
    } else {
        assert!(
            grouped.is_empty(),
            "sem a feature web-push, o stub nao deve registrar subscriptions"
        );
    }
}
