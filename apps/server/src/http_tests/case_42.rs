use super::*;

#[tokio::test]
async fn pool_editorial_links_are_owner_scoped_and_freeze_with_predictions() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner = seed_user(
        &format!("editorial-owner-{suffix}"),
        &format!("editorial-owner-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let member = seed_user(
        &format!("editorial-member-{suffix}"),
        &format!("editorial-member-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (event_id, pool_id) = insert_custom_event_pool(&owner, "Evento editorial").await;
    let (_, options) = insert_custom_question(
        &event_id,
        "Escolha",
        "2099-01-01T00:00:00Z",
        "2099-01-02T00:00:00Z",
        &["A", "B"],
    )
    .await;
    sqlx::query("INSERT INTO option_links(id,option_id,kind,label,url,sort_order) VALUES(?1,?2,'official','Padrão','https://event.test/padrao',0)")
        .bind(uuid::Uuid::new_v4().to_string()).bind(&options[0]).execute(crate::db::pool()).await.unwrap();
    add_membership(&pool_id, &member).await;
    let (owner_token, owner_csrf) = seed_session(&owner).await;
    let owner_client = client_with_session(base, &owner_token);
    let (member_token, member_csrf) = seed_session(&member).await;
    let member_client = client_with_session(base, &member_token);

    let denied = member_client
        .get(format!("{base}/api/pools/{pool_id}/editorial"))
        .send()
        .await
        .unwrap();
    assert!(
        !denied.status().is_success(),
        "membro não administra editorial"
    );
    let missing_csrf = owner_client
        .post(format!("{base}/api/pools/{pool_id}/editorial/name"))
        .json(&json!({"name":"Outro"}))
        .send()
        .await
        .unwrap();
    assert!(!missing_csrf.status().is_success());
    let renamed = owner_client
        .post(format!("{base}/api/pools/{pool_id}/editorial/name"))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"name":"Bolão isolado"}))
        .send()
        .await
        .unwrap();
    assert!(renamed.status().is_success());
    let replaced = owner_client.post(format!("{base}/api/pools/{pool_id}/editorial/options/{}/links", options[0])).header("X-CSRF-Token", &owner_csrf).json(&json!({"links":[{"kind":"video","label":"Vídeo próprio","url":"https://pool.test/video"}]})).send().await.unwrap();
    assert!(replaced.status().is_success());
    let invalid_url = owner_client
        .post(format!(
            "{base}/api/pools/{pool_id}/editorial/options/{}/links",
            options[0]
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(
            &json!({"links":[{"kind":"official","label":"Inválido","url":"javascript:alert(1)"}]}),
        )
        .send()
        .await
        .unwrap();
    assert!(!invalid_url.status().is_success());
    let foreign_option = owner_client
        .post(format!(
            "{base}/api/pools/{pool_id}/editorial/options/{}/links",
            uuid::Uuid::new_v4()
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"links":[]}))
        .send()
        .await
        .unwrap();
    assert!(!foreign_option.status().is_success());
    let own_questions: Vec<crate::models::CustomQuestion> = owner_client
        .get(format!("{base}/api/custom/questions?poolId={pool_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        own_questions[0].options[0].links[0].url,
        "https://pool.test/video"
    );

    let version_id: (String,) = sqlx::query_as("SELECT event_version_id FROM pools WHERE id=?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    let other_pool = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO pools(id,event_id,event_version_id,name,invite_code,created_by) VALUES(?1,?2,?3,'Outro pool',?4,?5)").bind(&other_pool).bind(&event_id).bind(&version_id.0).bind(uuid::Uuid::new_v4().simple().to_string()).bind(&owner).execute(crate::db::pool()).await.unwrap();
    add_membership(&other_pool, &owner).await;
    let other_questions: Vec<crate::models::CustomQuestion> = owner_client
        .get(format!("{base}/api/custom/questions?poolId={other_pool}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        other_questions[0].options[0].links[0].url, "https://event.test/padrao",
        "outro Pool herda o evento"
    );
    let restored = owner_client
        .post(format!(
            "{base}/api/pools/{pool_id}/editorial/options/{}/links/reset",
            options[0]
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .unwrap();
    assert!(restored.status().is_success());
    let original: (String,) = sqlx::query_as("SELECT url FROM option_links WHERE option_id=?1")
        .bind(&options[0])
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert_eq!(
        original.0, "https://event.test/padrao",
        "EventVersion permanece intacta"
    );
    sqlx::query("UPDATE pools SET predictions_closed_at=datetime('now') WHERE id=?1")
        .bind(&pool_id)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let frozen = owner_client
        .post(format!("{base}/api/pools/{pool_id}/editorial/name"))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"name":"Proibido"}))
        .send()
        .await
        .unwrap();
    assert!(!frozen.status().is_success());
    let member_mutation = member_client
        .post(format!("{base}/api/pools/{other_pool}/editorial/name"))
        .header("X-CSRF-Token", &member_csrf)
        .json(&json!({"name":"Proibido"}))
        .send()
        .await
        .unwrap();
    assert!(!member_mutation.status().is_success());
}
