use super::*;

#[tokio::test]
async fn manual_match_lifecycle_creates_syncs_and_deletes_prediction_item() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin_id = seed_user(
        &format!("item-admin-{suffix}"),
        &format!("item-admin-{suffix}@teste.com"),
        "senha-correta-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE token = ?1")
        .bind(&token)
        .execute(crate::db::pool())
        .await
        .expect("reauth admin");
    let client = client_with_session(base, &token);
    let mut create_body = String::new();
    let mut create_status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
    for attempt in 0..8 {
        let response = client
            .post(format!("{base}/api/admin/matches"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({
                "homeTeam": "Brasil", "awayTeam": "Japao", "phase": "Final",
                "kickoff": "2030-07-01T20:00:00Z"
            }))
            .send()
            .await
            .expect("criar match manual");
        create_status = response.status();
        create_body = response.text().await.expect("corpo create");
        if create_status.is_success() {
            break;
        }
        if attempt < 7 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    assert!(create_status.is_success(), "criar match: {create_body}");
    let created: crate::models::MatchRecord =
        serde_json::from_str(&create_body).expect("match criado");
    let item_id: (String,) = sqlx::query_as("SELECT prediction_item_id FROM matches WHERE id = ?1")
        .bind(&created.id)
        .fetch_one(crate::db::pool())
        .await
        .expect("item do match");
    let initial: (String, String, String) =
        sqlx::query_as("SELECT title, lock_at, reveal_at FROM prediction_items WHERE id = ?1")
            .bind(&item_id.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("item criado");
    assert_eq!(
        initial,
        (
            "Brasil x Japao".into(),
            "2030-07-01T20:00:00+00:00".into(),
            "2030-07-01T20:00:00+00:00".into()
        )
    );

    let updated_kickoff = "2030-07-02T20:00:00Z";
    let updated: crate::models::MatchRecord = client
        .post(format!("{base}/api/admin/matches/{}/schedule", created.id))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "homeTeam": "Brasil", "awayTeam": "Coreia", "phase": "Final", "kickoff": updated_kickoff }))
        .send().await.expect("editar schedule").error_for_status().expect("status schedule")
        .json().await.expect("match atualizado");
    assert_eq!(updated.kickoff, "2030-07-02T20:00:00+00:00");
    let synced: (String, String, String) =
        sqlx::query_as("SELECT title, lock_at, reveal_at FROM prediction_items WHERE id = ?1")
            .bind(&item_id.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("item sincronizado");
    assert_eq!(
        synced,
        (
            "Brasil x Coreia".into(),
            updated.kickoff.clone(),
            updated.kickoff
        )
    );

    let deleted = client
        .post(format!("{base}/api/admin/matches/{}/delete", created.id))
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .expect("deletar match");
    assert!(deleted.status().is_success());
    let item_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE id = ?1")
        .bind(&item_id.0)
        .fetch_one(crate::db::pool())
        .await
        .expect("verificar item removido");
    assert_eq!(item_exists.0, 0);
}
