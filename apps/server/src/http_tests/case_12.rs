use super::*;

#[tokio::test]
async fn admin_manifest_preview_apply_is_idempotent_and_blocks_published_structure() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_user(
        &format!("manifest-admin-{suffix}"),
        &format!("manifest-admin-{suffix}@example.test"),
        "Senha-forte-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE token = ?1")
        .bind(&token)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth de manifesto");
    let admin = client_with_session(base, &token);
    let slug = format!("manifest-smoke-{suffix}");
    let content = serde_json::json!({
        "schemaVersion": 1,
        "name": "Manifest Smoke",
        "slug": slug,
        "kind": "custom",
        "description": "descrição inicial",
        "items": [{
            "externalKey": "best-picture",
            "kind": "single_choice",
            "title": "Best Picture",
            "lockAt": "2099-01-01T00:00:00Z",
            "revealAt": "2099-01-02T00:00:00Z",
            "options": [
                {"externalKey": "artist-a", "label": "Artist A"},
                {"externalKey": "artist-b", "label": "Artist B"}
            ]
        }]
    })
    .to_string();
    let preview: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": content.clone(), "filename": "smoke.json"}))
        .send()
        .await
        .expect("preview create")
        .json()
        .await
        .expect("json preview create");
    assert_eq!(preview["action"], "create");
    let applied_response = admin
        .post(format!("{base}/api/admin/events/import/apply"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": content.clone(), "baseFingerprint": preview["baseFingerprint"], "filename": "smoke.json"}))
        .send()
        .await
        .expect("apply create");
    let applied_status = applied_response.status();
    let applied_body = applied_response.text().await.expect("body apply create");
    assert!(
        applied_status.is_success(),
        "apply create returned {applied_status}: {applied_body}"
    );
    let applied: serde_json::Value =
        serde_json::from_str(&applied_body).expect("json apply create");
    assert_eq!(applied["action"], "create");
    let event_id = applied["eventId"].as_str().expect("event id").to_string();

    let second: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": content.clone()}))
        .send()
        .await
        .expect("preview no change")
        .json()
        .await
        .expect("json preview no change");
    assert_eq!(second["action"], "noChange");

    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .expect("publicar smoke");
    let editorial = content.replace("Manifest Smoke", "Manifest Smoke Renamed");
    let editorial_preview: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": editorial}))
        .send()
        .await
        .expect("preview editorial")
        .json()
        .await
        .expect("json preview editorial");
    assert_eq!(editorial_preview["action"], "safeUpdate");
    assert!(editorial_preview["safeChanges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"] == "Event.name"));
    let editorial_applied: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/apply"))
        .header("X-CSRF-Token", &csrf)
        .json(
            &json!({"content": editorial, "baseFingerprint": editorial_preview["baseFingerprint"]}),
        )
        .send()
        .await
        .expect("apply editorial")
        .json()
        .await
        .expect("json apply editorial");
    assert_eq!(editorial_applied["action"], "safeUpdate");
    let exported = admin
        .get(format!("{base}/api/admin/events/{event_id}/manifest"))
        .send()
        .await
        .expect("exportar manifesto");
    assert!(exported.status().is_success());
    assert!(exported
        .headers()
        .get("content-disposition")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains(&format!("{slug}.json"))));
    let exported_json: serde_json::Value = exported.json().await.expect("json exportado");
    assert_eq!(exported_json["schemaVersion"], 2);
    assert!(exported_json.get("eventId").is_none());
    let export_audit: (String,) = sqlx::query_as(
        "SELECT details_json FROM audit_logs WHERE action='event_manifest_exported' AND target_id=?1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("auditoria de export manifesto");
    assert!(export_audit.0.contains("manifestFingerprint"));
    assert!(!export_audit.0.contains("Best Picture"));
    assert!(!export_audit.0.contains("Artist A"));
    let import_audit: (String,) = sqlx::query_as(
        "SELECT details_json FROM audit_logs WHERE action='event_manifest_imported' AND target_id=?1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("auditoria de import manifesto");
    assert!(import_audit.0.contains("manifestFingerprint"));
    assert!(!import_audit.0.contains("Manifest Smoke Renamed"));

    let structural = editorial.replace("Artist A", "Different Label");
    let structural_preview: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": structural}))
        .send()
        .await
        .expect("preview structural conflict")
        .json()
        .await
        .expect("json preview structural conflict");
    assert_eq!(structural_preview["action"], "safeUpdate");
    assert!(structural_preview["safeChanges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"].as_str().unwrap_or_default().contains("label")));
    let stored: (String,) =
        sqlx::query_as("SELECT name FROM event_versions WHERE event_id=?1 AND state='working'")
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("evento preservado após conflict preview");
    assert_eq!(stored.0, "Manifest Smoke Renamed");
}
