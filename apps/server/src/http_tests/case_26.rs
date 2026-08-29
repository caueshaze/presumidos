use super::*;

#[tokio::test]
async fn pool_member_can_report_once_and_admin_can_review_report() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner_id = seed_user(
        &format!("report-owner-{suffix}"),
        &format!("report-owner-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let reporter_id = seed_user(
        &format!("report-user-{suffix}"),
        &format!("report-user-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let admin_id = seed_user(
        &format!("report-admin-{suffix}"),
        &format!("report-admin-{suffix}@teste.com"),
        "senha-correta-123",
        true,
    )
    .await;
    let pool_id = insert_pool(&format!("Bolao report {suffix}"), &owner_id).await;
    add_membership(&pool_id, &owner_id).await;
    add_membership(&pool_id, &reporter_id).await;

    let (reporter_token, reporter_csrf) = seed_session(&reporter_id).await;
    let reporter = client_with_session(base, &reporter_token);
    let report_url = format!("{base}/api/pools/{pool_id}/reports");
    let created = reporter
        .post(&report_url)
        .header("X-CSRF-Token", &reporter_csrf)
        .json(&json!({ "category": "spam_or_fraud", "details": "Há links suspeitos neste bolão." }))
        .send()
        .await
        .expect("criar denuncia");
    assert!(created.status().is_success());
    let created_body: serde_json::Value = created.json().await.expect("corpo da denuncia");
    assert_eq!(created_body["status"], "open");
    assert_eq!(created_body["poolName"], format!("Bolao report {suffix}"));

    let duplicate = reporter
        .post(&report_url)
        .header("X-CSRF-Token", &reporter_csrf)
        .json(&json!({ "category": "other", "details": "Outra tentativa" }))
        .send()
        .await
        .expect("denuncia duplicada");
    assert!(!duplicate.status().is_success());

    let (admin_token, admin_csrf) = seed_session(&admin_id).await;
    let admin = client_with_session(base, &admin_token);
    let listed: Vec<crate::models::PoolReport> = admin
        .get(format!("{base}/api/admin/pool-reports?status=open"))
        .send()
        .await
        .expect("listar denuncias")
        .json()
        .await
        .expect("corpo da lista de denuncias");
    let report = listed
        .iter()
        .find(|item| item.pool_id == pool_id)
        .expect("denuncia criada deveria ser listada");
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE user_id=?1")
        .bind(&admin_id)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth do admin");
    let updated = admin
        .post(format!(
            "{base}/api/admin/pool-reports/{}/status",
            report.id
        ))
        .header("X-CSRF-Token", &admin_csrf)
        .json(&json!({ "status": "resolved" }))
        .send()
        .await
        .expect("atualizar status da denuncia");
    assert!(updated.status().is_success());
    let updated_body: serde_json::Value = updated.json().await.expect("corpo atualizado");
    assert_eq!(updated_body["status"], "resolved");
}
