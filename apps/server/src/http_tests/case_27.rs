use super::*;

#[tokio::test]
async fn event_deletion_distinguishes_origin_and_preserves_existing_pools() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner_id = seed_user(
        &format!("event-delete-owner-{suffix}"),
        &format!("event-delete-owner-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let admin_id = seed_user(
        &format!("event-delete-admin-{suffix}"),
        &format!("event-delete-admin-{suffix}@teste.com"),
        "senha-correta-123",
        true,
    )
    .await;
    let (owner_token, owner_csrf) = seed_session(&owner_id).await;
    let owner_client = client_with_session(base, &owner_token);

    let (user_event_id, pool_id) =
        insert_custom_event_pool(&owner_id, &format!("Evento usuario {suffix}")).await;
    let owner_delete = owner_client
        .post(format!("{base}/api/custom/events/{user_event_id}/delete"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("arquivar evento do dono");
    assert!(
        owner_delete.status().is_success(),
        "erro ao arquivar evento do dono: {}",
        owner_delete.text().await.unwrap_or_default()
    );
    let archived: (Option<String>, i64) =
        sqlx::query_as("SELECT archived_at, pool_creation_enabled FROM events WHERE id=?1")
            .bind(&user_event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("ler evento arquivado");
    assert!(archived.0.is_some());
    assert_eq!(archived.1, 0);
    let preserved_pool: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE id=?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("verificar pool preservado");
    assert_eq!(preserved_pool.0, 1);
    let dashboard: Vec<serde_json::Value> = owner_client
        .get(format!("{base}/api/pools/dashboard"))
        .send()
        .await
        .expect("acessar pool de evento arquivado")
        .json()
        .await
        .expect("corpo do dashboard preservado");
    assert!(dashboard
        .iter()
        .any(|summary| summary["pool"]["id"] == pool_id));

    let available: Vec<serde_json::Value> = owner_client
        .get(format!("{base}/api/custom/events/available"))
        .send()
        .await
        .expect("listar eventos disponiveis")
        .json()
        .await
        .expect("corpo de eventos disponiveis");
    assert!(!available.iter().any(|event| event["id"] == user_event_id));

    let system_event_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO events(id,name,slug,kind,status,created_by,pool_creation_enabled)
         VALUES(?1,?2,?3,'custom','active',NULL,1)",
    )
    .bind(&system_event_id)
    .bind(format!("Evento padrao {suffix}"))
    .bind(format!("system-event-{suffix}"))
    .execute(crate::db::pool())
    .await
    .expect("inserir evento padrao de teste");

    let denied_system = owner_client
        .post(format!("{base}/api/custom/events/{system_event_id}/delete"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("tentar apagar evento padrao como dono");
    assert!(!denied_system.status().is_success());

    let (admin_token, admin_csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at=datetime('now') WHERE token=?1")
        .bind(&admin_token)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth do admin");
    let admin_client = client_with_session(base, &admin_token);
    let admin_delete_system = admin_client
        .post(format!("{base}/api/admin/events/{system_event_id}/delete"))
        .header("X-CSRF-Token", &admin_csrf)
        .send()
        .await
        .expect("arquivar evento padrao como admin");
    assert!(admin_delete_system.status().is_success());

    let admin_events: Vec<AdminEventRecord> = admin_client
        .get(format!("{base}/api/admin/events"))
        .send()
        .await
        .expect("listar eventos no admin")
        .json()
        .await
        .expect("corpo de eventos no admin");
    let system_record = admin_events
        .iter()
        .find(|event| event.id == system_event_id)
        .expect("evento padrao deve permanecer no admin");
    assert_eq!(system_record.origin, crate::models::EventOrigin::System);
    assert!(system_record.archived_at.is_some());
    let user_record = admin_events
        .iter()
        .find(|event| event.id == user_event_id)
        .expect("evento do usuario deve permanecer no admin");
    assert_eq!(user_record.origin, crate::models::EventOrigin::User);
    assert!(user_record.archived_at.is_some());

    let draft_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO events(id,name,slug,kind,status,created_by,pool_creation_enabled)
         VALUES(?1,?2,?3,'custom','draft',?4,1)",
    )
    .bind(&draft_id)
    .bind(format!("Rascunho apagavel {suffix}"))
    .bind(format!("draft-event-{suffix}"))
    .bind(&owner_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir rascunho para exclusao");
    let admin_delete_draft = admin_client
        .post(format!("{base}/api/admin/events/{draft_id}/delete"))
        .header("X-CSRF-Token", &admin_csrf)
        .send()
        .await
        .expect("excluir rascunho como admin");
    assert!(admin_delete_draft.status().is_success());
    let draft_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE id=?1")
        .bind(&draft_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("verificar rascunho excluido");
    assert_eq!(draft_count.0, 0);
}
