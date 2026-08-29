use super::*;

#[tokio::test]
async fn ended_events_are_historical_and_admin_can_finish_legacy_edition_without_data_loss() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_user(
        &format!("legacy-admin-{suffix}"),
        &format!("legacy-admin-{suffix}@example.test"),
        "Senha-forte-123",
        true,
    )
    .await;
    let member_id = seed_user(
        &format!("legacy-member-{suffix}"),
        &format!("legacy-member-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let event_id = uuid::Uuid::new_v4().to_string();
    let pool_id = uuid::Uuid::new_v4().to_string();
    let item_id = uuid::Uuid::new_v4().to_string();
    let option_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO events (id,name,slug,kind,status,ends_at)
         VALUES (?1,'Edição legada',?2,'custom','active','2020-01-01T00:00:00Z')",
    )
    .bind(&event_id)
    .bind(format!("legacy-{suffix}"))
    .execute(crate::db::pool())
    .await
    .expect("inserir evento legado");
    sqlx::query(
        "INSERT INTO pools (id,event_id,name,invite_code,created_by) VALUES (?1,?2,'Bolão legado',?3,?4)",
    )
    .bind(&pool_id)
    .bind(&event_id)
    .bind(format!("L{suffix}")[..8].to_uppercase())
    .bind(&member_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir pool legado");
    add_membership(&pool_id, &member_id).await;
    sqlx::query(
        "INSERT INTO prediction_items (id,event_id,kind,title,lock_at,reveal_at,sort_order,status)
         VALUES (?1,?2,'single_choice','Pergunta preservada','2099-01-01T00:00:00Z','2099-01-02T00:00:00Z',0,'open')",
    )
    .bind(&item_id)
    .bind(&event_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir item legado");
    sqlx::query("INSERT INTO custom_questions (item_id,points) VALUES (?1,1)")
        .bind(&item_id)
        .execute(crate::db::pool())
        .await
        .expect("inserir pergunta legada");
    sqlx::query(
        "INSERT INTO custom_question_options (id,item_id,label,sort_order) VALUES (?1,?2,'A',0)",
    )
    .bind(&option_id)
    .bind(&item_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir opção legada");

    let (member_token, member_csrf) = seed_session(&member_id).await;
    let member = client_with_session(base, &member_token);
    let before: serde_json::Value = member
        .get(format!("{base}/api/pools/dashboard"))
        .send()
        .await
        .expect("dashboard legado")
        .json()
        .await
        .expect("json dashboard legado");
    assert_eq!(before[0]["pool"]["id"], pool_id);
    assert_eq!(before[0]["pool"]["event"]["isHistorical"], true);

    let (admin_token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE token = ?1")
        .bind(&admin_token)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth");
    let admin = client_with_session(base, &admin_token);
    let finished: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/{event_id}/finish"))
        .header("X-CSRF-Token", csrf)
        .send()
        .await
        .expect("encerrar evento legado")
        .json()
        .await
        .expect("json evento encerrado");
    assert_eq!(finished["status"], "finished");

    let stored: (String,) = sqlx::query_as("SELECT status FROM events WHERE id=?1")
        .bind(&event_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("status armazenado");
    assert_eq!(stored.0, "finished");
    let pool_still_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE id=?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("pool preservado");
    assert_eq!(pool_still_exists.0, 1);
    let audit: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='event_finished' AND target_id=?1",
    )
    .bind(&event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("auditoria de encerramento");
    assert_eq!(audit.0, 1);

    let mutation = member
        .post(format!("{base}/api/custom/predictions"))
        .header("X-CSRF-Token", member_csrf)
        .json(&json!({"poolId": pool_id, "itemId": item_id, "optionId": option_id}))
        .send()
        .await
        .expect("tentar editar edição encerrada");
    assert!(!mutation.status().is_success());
    let prediction_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("palpites preservados sem criação");
    assert_eq!(prediction_count.0, 0);
}
