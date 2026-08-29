use super::*;

#[tokio::test]
async fn admin_package_http_flow_works_without_tcp() {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use std::net::SocketAddr;
    use tower::ServiceExt;

    test_server().await;
    let app = axum::Router::new()
        .nest("/api", crate::api::router())
        .route(
            "/media/assets/{asset_id}/{variant}",
            axum::routing::get(crate::api::media_asset),
        )
        .layer(axum::middleware::from_fn(crate::api::context_middleware));
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_user(
        &format!("package-http-admin-{suffix}"),
        &format!("package-http-admin-{suffix}@example.test"),
        "Senha-forte-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at=datetime('now') WHERE token=?1")
        .bind(&token)
        .execute(crate::db::pool())
        .await
        .expect("reauth package HTTP");
    let slug = format!("pacote-http-{}", suffix);
    let package = package_smoke_without_assets(&slug);
    let boundary = "package-http-boundary";
    let multipart = |base_fingerprint: Option<&str>| {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"pacote-http.zip\"\r\nContent-Type: application/zip\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(&package);
        if let Some(fingerprint) = base_fingerprint {
            body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"baseFingerprint\"\r\n\r\n{fingerprint}\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        } else {
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        }
        body
    };
    let send = |path: &str, body: Vec<u8>| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", &csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(body))
            .expect("request package HTTP")
    };
    let verified = app
        .clone()
        .oneshot(send("/api/admin/reauth/verify", Vec::new()))
        .await
        .expect("verificar reauth antes do upload");
    assert_eq!(verified.status(), StatusCode::NO_CONTENT);
    let preview = app
        .clone()
        .oneshot(send(
            "/api/admin/events/import/package/preview",
            multipart(None),
        ))
        .await
        .expect("preview package HTTP");
    assert_eq!(preview.status(), StatusCode::OK);
    let preview_body = to_bytes(preview.into_body(), 1_000_000)
        .await
        .expect("preview body HTTP");
    let preview_json: serde_json::Value = serde_json::from_slice(&preview_body).unwrap();
    assert_eq!(preview_json["manifest"]["action"], "create");
    assert_eq!(preview_json["manifest"]["itemCount"], 1);
    let base_fingerprint = preview_json["manifest"]["baseFingerprint"]
        .as_str()
        .expect("base fingerprint HTTP")
        .to_string();
    let before_apply: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM events WHERE slug=?1),(SELECT COUNT(*) FROM prediction_items pi JOIN events e ON e.id=pi.event_id WHERE e.slug=?1),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id JOIN events e ON e.id=pi.event_id WHERE e.slug=?1)")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("contagens antes do apply HTTP");
    assert_eq!(before_apply, (0, 0, 0));
    let applied = app
        .clone()
        .oneshot(send(
            "/api/admin/events/import/package/apply",
            multipart(Some(&base_fingerprint)),
        ))
        .await
        .expect("apply package HTTP");
    assert_eq!(applied.status(), StatusCode::OK);
    let applied_body = to_bytes(applied.into_body(), 1_000_000)
        .await
        .expect("apply body HTTP");
    let applied_json: serde_json::Value = serde_json::from_slice(&applied_body).unwrap();
    assert_eq!(applied_json["result"]["action"], "create");
    let created_status: (String,) = sqlx::query_as("SELECT status FROM events WHERE slug=?1")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("status do evento importado HTTP");
    assert_eq!(created_status.0, "draft");
    let repeat = app
        .clone()
        .oneshot(send(
            "/api/admin/events/import/package/preview",
            multipart(None),
        ))
        .await
        .expect("repeat preview package HTTP");
    let repeat_body = to_bytes(repeat.into_body(), 1_000_000)
        .await
        .expect("repeat body HTTP");
    let repeat_json: serde_json::Value = serde_json::from_slice(&repeat_body).unwrap();
    assert_eq!(repeat_json["manifest"]["action"], "noChange");
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM events WHERE slug=?1),(SELECT COUNT(*) FROM prediction_items WHERE event_id=(SELECT id FROM events WHERE slug=?1)),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=(SELECT id FROM events WHERE slug=?1))")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("contagens package HTTP");
    assert_eq!(counts, (1, 1, 2));
    let events = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/events")
                .header(
                    "Cookie",
                    format!("{}={token}", crate::security::session_cookie_name()),
                )
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("listar eventos no Admin"),
        )
        .await
        .expect("resposta de eventos no Admin");
    assert_eq!(events.status(), StatusCode::OK);
    let events_body = to_bytes(events.into_body(), 1_000_000)
        .await
        .expect("corpo de eventos no Admin");
    let events_json: serde_json::Value = serde_json::from_slice(&events_body).unwrap();
    let admin_event = events_json
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["slug"] == slug)
        .expect("evento de pacote na lista Admin");
    let expected_username = format!("package-http-admin-{suffix}");
    assert_eq!(
        admin_event["createdByUsername"].as_str(),
        Some(expected_username.as_str())
    );
    assert_eq!(admin_event["itemCount"], 1);
    assert_eq!(admin_event["optionCount"], 2);
    assert_eq!(admin_event["poolCount"], 0);
    assert!(admin_event["updatedAt"].as_str().is_some());
}
