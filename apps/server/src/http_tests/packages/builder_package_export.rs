use super::*;

#[tokio::test]
async fn builder_package_export_includes_uploaded_assets_and_reports_external_urls() {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use std::net::SocketAddr;
    use tower::ServiceExt;

    test_server().await;
    let app = axum::Router::new()
        .nest("/api", crate::api::router())
        .layer(axum::middleware::from_fn(crate::api::context_middleware));
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = seed_user(
        &format!("builder-package-{suffix}"),
        &format!("builder-package-{suffix}@example.test"),
        "Senha-forte-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&user).await;
    let (event_id, item_id, option_id, _) = create_builder_event(app.clone(), &token, &csrf).await;
    let image = package_smoke_master([20, 60, 140]);
    let boundary = "builder-package-export";
    let upload = |path: String, bytes: &[u8]| {
        let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"image.webp\"\r\nContent-Type: image/webp\r\n\r\n").into_bytes();
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
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
            .expect("upload request")
    };
    for path in [
        format!("/api/custom/events/{event_id}/cover"),
        format!("/api/custom/events/{event_id}/items/{item_id}/options/{option_id}/image"),
    ] {
        let response = app
            .clone()
            .oneshot(upload(path, &image))
            .await
            .expect("upload asset");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let external = app
        .clone()
        .oneshot(json_request(
            &token,
            &csrf,
            format!("/api/custom/events/{event_id}/items/{item_id}/options/{option_id}/media"),
            json!({"imageUrl":"https://example.test/editorial.webp","links":[]}),
        ))
        .await
        .expect("save external URL");
    assert_eq!(external.status(), StatusCode::NO_CONTENT);
    let published = app
        .clone()
        .oneshot(json_request(
            &token,
            &csrf,
            format!("/api/custom/events/{event_id}/publish"),
            json!({}),
        ))
        .await
        .expect("publish event");
    assert_eq!(published.status(), StatusCode::NO_CONTENT);
    let working_cover = package_smoke_master([180, 40, 30]);
    let uploaded_working_cover = app
        .clone()
        .oneshot(upload(
            format!("/api/custom/events/{event_id}/cover"),
            &working_cover,
        ))
        .await
        .expect("upload working cover");
    assert_eq!(uploaded_working_cover.status(), StatusCode::OK);
    let get = |path: String| {
        Request::builder()
            .uri(path)
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::empty())
            .expect("download request")
    };
    let preview = response_json(
        app.clone()
            .oneshot(get(format!(
                "/api/custom/events/{event_id}/package/preview"
            )))
            .await
            .expect("package preview"),
    )
    .await;
    assert_eq!(
        preview["assetCount"], 2,
        "the working cover is exported beside the deduplicated option image"
    );
    assert_eq!(preview["externalImageCount"], 1);
    assert_eq!(preview["externalImages"][0]["question"], "Escolha");
    assert_eq!(preview["externalImages"][0]["optionLabel"], "A");
    let response = app
        .oneshot(get(format!("/api/custom/events/{event_id}/package")))
        .await
        .expect("package export");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 2_000_000)
        .await
        .expect("package bytes");
    let package = crate::event_package::parse(&bytes).expect("parse exported package");
    assert_eq!(package.assets.len(), 2);
    let cover = package.manifest.cover_asset.as_ref().unwrap();
    let option = &package.manifest.items[0].options[0];
    assert!(option.image_asset.is_some());
    assert_eq!(
        option.image_url.as_deref(),
        Some("https://example.test/editorial.webp")
    );
    let mut expected_entries = vec![
        format!("assets/{}.webp", cover.sha256),
        format!(
            "assets/{}.webp",
            option.image_asset.as_ref().unwrap().sha256
        ),
        "manifest.json".into(),
    ];
    expected_entries.sort();
    assert_eq!(package_smoke_entry_names(&bytes), expected_entries);
}
