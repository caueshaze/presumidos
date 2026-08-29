use super::*;

#[tokio::test]
async fn delete_account_removes_user_data_and_logs_out() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner_email = format!("owner-{suffix}@teste.com");
    let member_email = format!("member-{suffix}@teste.com");
    let owner_id = seed_user(
        &format!("owner-{suffix}"),
        &owner_email,
        "senha-correta-123",
        false,
    )
    .await;
    let member_id = seed_user(
        &format!("member-{suffix}"),
        &member_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao do owner {suffix}"), &owner_id).await;
    add_membership(&pool_id, &owner_id).await;
    add_membership(&pool_id, &member_id).await;

    let match_id = insert_match("Brasil", "Japao", "2999-01-01T00:00:00Z").await;
    insert_prediction(&member_id, &match_id, 2, 1).await;

    sqlx::query(
        "INSERT INTO notification_preferences (user_id, enabled, lead_time_minutes) VALUES (?1, 1, 20)",
    )
    .bind(&member_id)
    .execute(crate::db::pool())
    .await
    .expect("preferencia de notificacao");

    sqlx::query(
        "INSERT INTO push_subscriptions
            (id, user_id, endpoint, p256dh, auth, active)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&member_id)
    .bind(format!("https://push.example/{suffix}"))
    .bind("p256dh-teste")
    .bind("auth-teste")
    .execute(crate::db::pool())
    .await
    .expect("subscription de push");

    let (token, csrf) = seed_session(&member_id).await;
    let client = client_with_session(base, &token);
    let delete_url = format!("{base}/api/auth/delete");

    let deleted = client
        .post(&delete_url)
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .expect("excluir conta");
    assert!(
        deleted.status().is_success(),
        "exclusao deveria ter sucesso"
    );

    let current = client
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("current_user apos exclusao");
    let session: SessionState = current.json().await.expect("sessao apos exclusao");
    assert!(session.user.is_none(), "sessao deveria estar encerrada");

    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?1")
        .bind(&member_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("contar usuario");
    assert_eq!(user_count.0, 0);

    let prediction_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar palpites");
    assert_eq!(prediction_count.0, 0);

    let membership_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar memberships");
    assert_eq!(membership_count.0, 0);

    let pref_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notification_preferences WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar preferencias");
    assert_eq!(pref_count.0, 0);

    let push_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM push_subscriptions WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar subscriptions");
    assert_eq!(push_count.0, 0);

    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'account_deleted' AND target_id = ?1",
    )
    .bind(&member_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("contar auditoria de exclusao");
    assert_eq!(audit_count.0, 1);
}
