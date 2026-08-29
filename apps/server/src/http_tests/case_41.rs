use super::*;

#[tokio::test]
async fn legacy_manifest_v1_import_works_without_tcp() {
    test_server().await;
    let slug = format!("legacy-v1-smoke-{}", uuid::Uuid::new_v4().simple());
    let content = serde_json::json!({
        "schemaVersion": 1,
        "name": "Legacy sem imagens",
        "slug": slug,
        "kind": "custom",
        "items": [{
            "externalKey": "choice",
            "kind": "single_choice",
            "title": "Escolha",
            "lockAt": "2099-01-01T00:00:00Z",
            "revealAt": "2099-01-02T00:00:00Z",
            "options": [
                {"externalKey": "a", "label": "A"},
                {"externalKey": "b", "label": "B"}
            ]
        }]
    })
    .to_string();
    let manifest =
        crate::custom_event_manifest::parse_and_validate(&content).expect("manifest v1 legado");
    assert_eq!(
        crate::custom_event_manifest::import(manifest, false)
            .await
            .unwrap(),
        (1, 2)
    );
    assert_eq!(
        crate::custom_event_manifest::import(
            crate::custom_event_manifest::parse_and_validate(&content).unwrap(),
            true,
        )
        .await
        .unwrap(),
        (1, 2)
    );
    let state: (String, i64, i64) = sqlx::query_as("SELECT e.status,(SELECT COUNT(*) FROM prediction_items pi JOIN event_versions v ON v.id=pi.event_version_id WHERE v.event_id=e.id AND v.state='working'),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id JOIN event_versions v ON v.id=pi.event_version_id WHERE v.event_id=e.id AND v.state='working') FROM events e WHERE e.slug=?1")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("estado do import legado");
    assert_eq!(state, ("draft".into(), 1, 2));
    assert_eq!(
        crate::custom_event_manifest::import(
            crate::custom_event_manifest::parse_and_validate(&content).unwrap(),
            true,
        )
        .await
        .unwrap(),
        (1, 2)
    );
    let event_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE slug=?1")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("idempotência do import legado");
    assert_eq!(event_count.0, 1);
}
