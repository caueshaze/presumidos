use super::*;

#[tokio::test]
async fn event_builder_draft_lifecycle_enforces_owner_publish_and_pool_scoring() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = seed_user(
        &format!("builder-owner-{suffix}"),
        &format!("builder-owner-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let other_id = seed_user(
        &format!("builder-other-{suffix}"),
        &format!("builder-other-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (owner_token, owner_csrf) = seed_session(&owner_id).await;
    let (other_token, other_csrf) = seed_session(&other_id).await;
    let owner = client_with_session(base, &owner_token);
    let other = client_with_session(base, &other_token);
    let invalid = owner
        .post(format!("{base}/api/custom/events"))
        .json(&json!({"name":"Sem CSRF"}))
        .send()
        .await
        .expect("criar sem csrf");
    assert_eq!(invalid.status(), reqwest::StatusCode::FORBIDDEN);
    let event: serde_json::Value = owner.post(format!("{base}/api/custom/events")).header("X-CSRF-Token", &owner_csrf).json(&json!({"name":"Premiação Builder","startsAt":"2026-10-01T18:00:00Z","endsAt":"2026-10-01T22:00:00Z"})).send().await.expect("criar draft").json().await.expect("json do draft");
    let event_id = event["id"].as_str().expect("id").to_string();
    assert_eq!(event["status"], "draft");
    let forbidden = other.post(format!("{base}/api/custom/events/{event_id}/items")).header("X-CSRF-Token", &other_csrf).json(&json!({"title":"Invasão","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z"})).send().await.expect("tentativa de outro usuario");
    assert_eq!(forbidden.status(), reqwest::StatusCode::BAD_REQUEST);
    let item: serde_json::Value = owner.post(format!("{base}/api/custom/events/{event_id}/items")).header("X-CSRF-Token", &owner_csrf).json(&json!({"title":"Melhor café","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z"})).send().await.expect("criar pergunta").json().await.expect("json da pergunta");
    let item_id = item["id"].as_str().expect("item id");
    let mut option_ids = Vec::new();
    for label in ["Coado", "Máquina", "Solúvel"] {
        let response = owner
            .post(format!(
                "{base}/api/custom/events/{event_id}/items/{item_id}/options"
            ))
            .header("X-CSRF-Token", &owner_csrf)
            .json(&json!({"label":label}))
            .send()
            .await
            .expect("criar opcao");
        assert!(response.status().is_success());
        let value: serde_json::Value = response.json().await.expect("json da opcao");
        option_ids.push(value["id"].as_str().expect("id da opcao").to_string());
    }
    let moved = owner
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options/{}/move",
            option_ids[2]
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"direction":-1}))
        .send()
        .await
        .expect("mover opcao");
    assert!(moved.status().is_success());
    let removed = owner
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options/{}/delete",
            option_ids[1]
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("remover opcao");
    assert!(removed.status().is_success());
    let replacement = owner
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options"
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"label":"Prensa"}))
        .send()
        .await
        .expect("adicionar apos remover");
    assert!(
        replacement.status().is_success(),
        "a ordem não pode colidir após remover opção"
    );
    let published = owner
        .post(format!("{base}/api/custom/events/{event_id}/publish"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("publicar");
    assert!(published.status().is_success());
    let immutable = owner.post(format!("{base}/api/custom/events/{event_id}/items")).header("X-CSRF-Token", &owner_csrf).json(&json!({"title":"Tarde","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z"})).send().await.expect("editar publicado");
    assert_eq!(immutable.status(), reqwest::StatusCode::BAD_REQUEST);
    let pool: serde_json::Value = owner
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"name":"Bolão do café","eventId":event_id}))
        .send()
        .await
        .expect("criar pool custom")
        .json()
        .await
        .expect("pool json");
    let scoring: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM custom_pool_item_scoring WHERE pool_id=?1")
            .bind(pool["id"].as_str().expect("pool id"))
            .fetch_one(crate::db::pool())
            .await
            .expect("scoring custom");
    assert_eq!(scoring.0, 1, "cada item publicado recebe scoring do Pool");
    let archived = owner
        .post(format!("{base}/api/custom/events/{event_id}/delete"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("arquivar evento publicado");
    assert_eq!(archived.status(), reqwest::StatusCode::NO_CONTENT);
    let archived_state: (Option<String>, i64) =
        sqlx::query_as("SELECT archived_at, pool_creation_enabled FROM events WHERE id=?1")
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("ler evento arquivado");
    assert!(archived_state.0.is_some());
    assert_eq!(archived_state.1, 0);
    let blocked_new_pool = owner
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"name":"Bolão bloqueado","eventId":event_id}))
        .send()
        .await
        .expect("tentar criar pool arquivado");
    assert!(!blocked_new_pool.status().is_success());
    let denied_result = other
        .post(format!(
            "{base}/api/admin/custom/questions/{item_id}/result"
        ))
        .header("X-CSRF-Token", &other_csrf)
        .json(&json!({"optionId":option_ids[0]}))
        .send()
        .await
        .expect("resultado por não-owner");
    assert_eq!(denied_result.status(), reqwest::StatusCode::BAD_REQUEST);
    let denied_pool_result = other
        .post(format!(
            "{base}/api/admin/custom/questions/{item_id}/result"
        ))
        .header("X-CSRF-Token", &other_csrf)
        .json(&json!({"optionId":option_ids[0],"poolId":pool["id"]}))
        .send()
        .await
        .expect("resultado no pool de outro usuario");
    assert_eq!(
        denied_pool_result.status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    let owner_result = owner
        .post(format!(
            "{base}/api/admin/custom/questions/{item_id}/result"
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"optionId":option_ids[0],"poolId":pool["id"]}))
        .send()
        .await
        .expect("resultado pelo owner");
    assert!(owner_result.status().is_success());
    let audit: (String,) = sqlx::query_as("SELECT actor_user_id FROM audit_logs WHERE action='event_official_result_changed' AND target_id=?1 ORDER BY created_at DESC LIMIT 1")
        .bind(item_id).fetch_one(crate::db::pool()).await.expect("auditoria de resultado");
    assert_eq!(audit.0, owner_id);
    let disposable: serde_json::Value = owner
        .post(format!("{base}/api/custom/events"))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"name":"Rascunho descartável"}))
        .send()
        .await
        .expect("criar descartável")
        .json()
        .await
        .expect("json descartável");
    let deleted = owner
        .post(format!(
            "{base}/api/custom/events/{}/delete",
            disposable["id"].as_str().expect("id descartável")
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("excluir draft");
    assert!(deleted.status().is_success());
}
