use super::*;

#[tokio::test]
async fn public_invite_preview_is_minimal_and_join_is_idempotent() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = seed_user(
        &format!("invite-owner-{suffix}"),
        &format!("invite-owner-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let member_id = seed_user(
        &format!("invite-member-{suffix}"),
        &format!("invite-member-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let event_id = uuid::Uuid::new_v4().to_string();
    let pool_id = uuid::Uuid::new_v4().to_string();
    let invite_code = uuid::Uuid::new_v4().simple().to_string()[..8].to_uppercase();
    sqlx::query(
        "INSERT INTO events(id,name,slug,kind,status,ends_at,pool_creation_enabled)
         VALUES(?1,'Versão congelada',?2,'custom','active','2099-01-01T00:00:00Z',0)",
    )
    .bind(&event_id)
    .bind(format!("invite-{suffix}"))
    .execute(crate::db::pool())
    .await
    .expect("inserir evento do convite");
    let version_id = ensure_published_version(&event_id, "Versão congelada", &owner_id).await;
    sqlx::query(
        "INSERT INTO pools(id,event_id,event_version_id,name,invite_code,created_by)
         VALUES(?1,?2,?3,'Bolão do convite',?4,?5)",
    )
    .bind(&pool_id)
    .bind(&event_id)
    .bind(&version_id)
    .bind(&invite_code)
    .bind(&owner_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir pool do convite");
    add_membership(&pool_id, &owner_id).await;

    let anonymous: serde_json::Value = client()
        .get(format!("{base}/api/public/pools/invite/{invite_code}"))
        .send()
        .await
        .expect("preview público")
        .json()
        .await
        .expect("json do preview");
    assert_eq!(anonymous["joinStatus"], "joinable");
    assert_eq!(anonymous["poolName"], "Bolão do convite");
    assert!(anonymous.get("poolId").is_none() || anonymous["poolId"].is_null());
    assert!(anonymous.get("predictions").is_none());
    assert!(anonymous.get("visibleRules").is_none());
    assert!(!version_id.is_empty());
    let members_after_preview: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id=?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("membership após preview");
    assert_eq!(members_after_preview.0, 1);

    let anonymous_join = client()
        .post(format!("{base}/api/pools/join"))
        .json(&json!({ "inviteCode": invite_code }))
        .send()
        .await
        .expect("tentativa anônima de entrada");
    assert_eq!(anonymous_join.status(), reqwest::StatusCode::UNAUTHORIZED);

    let invalid: serde_json::Value = client()
        .get(format!("{base}/api/public/pools/invite/NO-SUCH"))
        .send()
        .await
        .expect("preview inválido")
        .json()
        .await
        .expect("json do preview inválido");
    assert_eq!(invalid["joinStatus"], "invalid");

    sqlx::query("UPDATE pools SET name='Pool <& \"convite\"' WHERE id=?1")
        .bind(&pool_id)
        .execute(crate::db::pool())
        .await
        .expect("nome especial do convite");
    sqlx::query(
        "UPDATE event_versions SET description='Descrição <& \"pública\"', cover_url='https://cdn.example/cover?x=\"&y=1' WHERE id=?1",
    )
    .bind(&version_id)
    .execute(crate::db::pool())
    .await
    .expect("metadata pública do convite");
    let html = crate::render_invite_page(
        invite_code.clone(),
        std::sync::Arc::new("<html><head><title>Presumidos</title></head></html>".to_string()),
    )
    .await;
    assert_eq!(
        html.headers()[axum::http::header::CACHE_CONTROL],
        "private, no-store"
    );
    let html_body = axum::body::to_bytes(html.into_body(), 100_000)
        .await
        .expect("corpo HTML do convite");
    let html_body = String::from_utf8(html_body.to_vec()).expect("HTML UTF-8");
    assert!(html_body.contains("Pool &lt;&amp; &quot;convite&quot;"));
    assert!(
        html_body.contains("og:description\" content=\"Descrição &lt;&amp; &quot;pública&quot;\"")
    );
    assert!(html_body.contains("og:image\" content=\"https://cdn.example/cover?x=&quot;&amp;y=1\""));
    assert!(!html_body.contains("Pool <& \"convite\""));

    let (member_token, member_csrf) = seed_session(&member_id).await;
    let member = client_with_session(base, &member_token);
    let authenticated_preview: serde_json::Value = member
        .get(format!("{base}/api/public/pools/invite/{invite_code}"))
        .send()
        .await
        .expect("preview autenticado")
        .json()
        .await
        .expect("json do preview autenticado");
    assert_eq!(authenticated_preview["joinStatus"], "joinable");

    let member_a = client_with_session(base, &member_token);
    let member_b = client_with_session(base, &member_token);
    let request_a = member_a
        .post(format!("{base}/api/pools/join"))
        .header("X-CSRF-Token", &member_csrf)
        .json(&json!({ "inviteCode": invite_code }));
    let request_b = member_b
        .post(format!("{base}/api/pools/join"))
        .header("X-CSRF-Token", &member_csrf)
        .json(&json!({ "inviteCode": invite_code }));
    let (joined_a, joined_b) = tokio::join!(request_a.send(), request_b.send());
    assert!(joined_a
        .expect("primeira entrada concorrente")
        .status()
        .is_success());
    assert!(joined_b
        .expect("segunda entrada concorrente")
        .status()
        .is_success());

    for _ in 0..2 {
        assert!(member
            .post(format!("{base}/api/pools/join"))
            .header("X-CSRF-Token", &member_csrf)
            .json(&json!({ "inviteCode": invite_code }))
            .send()
            .await
            .expect("aceitar convite")
            .status()
            .is_success());
    }
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar membership");
    assert_eq!(count.0, 1);
    let member_preview: serde_json::Value = member
        .get(format!("{base}/api/public/pools/invite/{invite_code}"))
        .send()
        .await
        .expect("preview após entrada")
        .json()
        .await
        .expect("json do preview após entrada");
    assert_eq!(member_preview["joinStatus"], "already_member");
    assert_eq!(member_preview["poolId"], pool_id);
}
