use super::*;

#[tokio::test]
async fn user_event_builder_asset_pool_flow_works_without_tcp() {
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
    let user = seed_user(
        &format!("builder-asset-user-{suffix}"),
        &format!("builder-asset-user-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&user).await;
    let (event_id, single_id, single_option, numeric_id) =
        create_builder_event(app.clone(), &token, &csrf).await;

    let image = package_smoke_master([
        suffix.as_bytes()[0],
        suffix.as_bytes()[1],
        suffix.as_bytes()[2],
    ]);
    let boundary = "builder-asset-boundary";
    let multipart = |bytes: &[u8]| {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"cover.webp\"\r\nContent-Type: image/webp\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        body
    };
    let upload_request = |path: String| {
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
            .body(Body::from(multipart(&image)))
            .expect("upload builder")
    };
    let cover = app
        .clone()
        .oneshot(upload_request(format!(
            "/api/custom/events/{event_id}/cover"
        )))
        .await
        .expect("upload cover builder");
    assert_eq!(cover.status(), StatusCode::OK);
    let option_image = app
        .clone()
        .oneshot(upload_request(format!(
            "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/image"
        )))
        .await
        .expect("upload option builder");
    assert_eq!(option_image.status(), StatusCode::OK);

    let publish = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/custom/events/{event_id}/publish"))
                .header(
                    "Cookie",
                    format!("{}={token}", crate::security::session_cookie_name()),
                )
                .header("X-CSRF-Token", &csrf)
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("publish builder"),
        )
        .await
        .expect("publicar builder");
    assert_eq!(publish.status(), StatusCode::NO_CONTENT);
    let published_media_update = app
        .clone()
        .oneshot(json_request(
            &token,
            &csrf,
            format!(
                "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/media"
            ),
            json!({"imageUrl":"https://example.test/published-option.webp","links":[]}),
        ))
        .await
        .expect("editar mídia editorial publicada");
    assert_eq!(published_media_update.status(), StatusCode::BAD_REQUEST);
    let published_label_update = app
        .clone()
        .oneshot(json_request(
            &token,
            &csrf,
            format!(
                "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/update"
            ),
            json!({"label":"A renomeada"}),
        ))
        .await
        .expect("editar nome editorial publicado");
    assert_eq!(published_label_update.status(), StatusCode::BAD_REQUEST);
    let saved_label: (String,) =
        sqlx::query_as("SELECT label FROM custom_question_options WHERE id=?1")
            .bind(&single_option)
            .fetch_one(crate::db::pool())
            .await
            .expect("ler nome editorial publicado");
    assert_eq!(saved_label.0, "A");
    let replacement_image = package_smoke_master([100, 50, 20]);
    let replacement_request = {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"replacement.webp\"\r\nContent-Type: image/webp\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(&replacement_image);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/image"
            ))
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
            .expect("trocar imagem publicada")
    };
    let replacement = app
        .clone()
        .oneshot(replacement_request)
        .await
        .expect("troca de imagem publicada");
    assert_eq!(replacement.status(), StatusCode::BAD_REQUEST);
    let pool = response_json(
        app.clone()
            .oneshot(json_request(
                &token,
                &csrf,
                "/api/pools".into(),
                json!({"name":"Pool do smoke","eventId":event_id}),
            ))
            .await
            .expect("criar pool builder"),
    )
    .await;
    let pool_id = pool["id"].as_str().expect("id pool builder").to_string();
    let showcase = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/custom/event-showcase?poolId={pool_id}"))
                    .header(
                        "Cookie",
                        format!("{}={token}", crate::security::session_cookie_name()),
                    )
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                    .body(Body::empty())
                    .expect("ler showcase do pool"),
            )
            .await
            .expect("ler showcase builder"),
    )
    .await;
    assert!(showcase["coverAssetUrl"].as_str().is_some());
    let questions = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/custom/questions?poolId={pool_id}"))
                    .header(
                        "Cookie",
                        format!("{}={token}", crate::security::session_cookie_name()),
                    )
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                    .body(Body::empty())
                    .expect("ler perguntas do pool"),
            )
            .await
            .expect("ler perguntas builder"),
    )
    .await;
    assert!(questions
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| item["options"].as_array().into_iter().flatten())
        .any(|option| option["imageAssetUrl"].as_str().is_some()));
    let draft = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/custom/events/{event_id}/draft"))
                .header(
                    "Cookie",
                    format!("{}={token}", crate::security::session_cookie_name()),
                )
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("ler draft builder"),
        )
        .await
        .expect("ler draft publicado");
    let draft_body = to_bytes(draft.into_body(), 2_000_000)
        .await
        .expect("draft body builder");
    let draft_json: serde_json::Value =
        serde_json::from_slice(&draft_body).expect("draft JSON builder");
    assert!(draft_json["event"]["coverAssetUrl"].as_str().is_some());
    assert!(draft_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "numeric" && item["decimalPlaces"] == 1));
    assert!(draft_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "multiple_choice"));
    assert!(draft_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| item["options"].as_array().into_iter().flatten())
        .any(|option| option["imageAssetUrl"].as_str().is_some()));
    let mine = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri("/api/custom/events/mine")
                    .header(
                        "Cookie",
                        format!("{}={token}", crate::security::session_cookie_name()),
                    )
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                    .body(Body::empty())
                    .expect("listar eventos builder"),
            )
            .await
            .expect("listar eventos do builder"),
    )
    .await;
    assert!(mine
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["id"] == event_id && event["coverAssetUrl"].as_str().is_some()));
    let pool_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE event_id=?1")
        .bind(&event_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("pool count builder");
    assert_eq!(pool_count.0, 1);
    let _ = numeric_id;
}
