use super::*;

#[tokio::test]
async fn contextual_asset_upload_http_smoke_without_tcp() {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;
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
    let owner = seed_user(
        &format!("in-process-owner-{suffix}"),
        &format!("in-process-owner-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let other = seed_user(
        &format!("in-process-other-{suffix}"),
        &format!("in-process-other-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let event_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by) VALUES(?1,?2,?3,'custom','draft',?4)")
        .bind(&event_id)
        .bind("In-process asset smoke")
        .bind(format!("in-process-asset-{suffix}"))
        .bind(&owner)
        .execute(crate::db::pool())
    .await
    .expect("evento in-process");
    sqlx::query("INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,created_by) VALUES(?1,?2,1,'working',0,?3,?4)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&event_id)
        .bind("In-process asset smoke")
        .bind(&owner)
        .execute(crate::db::pool())
        .await
        .expect("versão working in-process");
    let (item_id, options) = insert_custom_question(
        &event_id,
        "Imagem editorial",
        "2099-01-01T00:00:00Z",
        "2099-01-02T00:00:00Z",
        &["A", "B"],
    )
    .await;
    let (owner_token, owner_csrf) = seed_session(&owner).await;
    let (other_token, _) = seed_session(&other).await;
    let fixture_digest = sha2::Sha256::digest(suffix.as_bytes());
    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
        24,
        12,
        Rgb([fixture_digest[0], fixture_digest[1], fixture_digest[2]]),
    ));
    let mut encoded = Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, ImageOutputFormat::Png)
        .expect("png fixture");
    let png = encoded.into_inner();
    let boundary = "asset-smoke-boundary";
    let multipart = |bytes: &[u8]| {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"editorial.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        body
    };
    let request = |path: String, token: &str, csrf: &str, body: Vec<u8>| {
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
            .header("X-CSRF-Token", csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(body))
            .expect("request multipart")
    };
    let cover = app
        .clone()
        .oneshot(request(
            format!("/api/custom/events/{event_id}/cover"),
            &owner_token,
            &owner_csrf,
            multipart(&png),
        ))
        .await
        .expect("upload cover in-process");
    assert_eq!(cover.status(), StatusCode::OK);
    let cover_body = to_bytes(cover.into_body(), 1_000_000)
        .await
        .expect("cover response body");
    let cover_asset: crate::assets::AssetResponse =
        serde_json::from_slice(&cover_body).expect("cover asset json");

    let option = app
        .clone()
        .oneshot(request(
            format!(
                "/api/custom/events/{event_id}/items/{item_id}/options/{}/image",
                options[0]
            ),
            &owner_token,
            &owner_csrf,
            multipart(&png),
        ))
        .await
        .expect("upload option in-process");
    assert_eq!(option.status(), StatusCode::OK);
    let option_body = to_bytes(option.into_body(), 1_000_000)
        .await
        .expect("option response body");
    let option_asset: crate::assets::AssetResponse =
        serde_json::from_slice(&option_body).expect("option asset json");
    assert_eq!(cover_asset.asset_id, option_asset.asset_id);

    let invalid = app
        .clone()
        .oneshot(request(
            format!("/api/custom/events/{event_id}/cover"),
            &owner_token,
            &owner_csrf,
            multipart(b"not-an-image"),
        ))
        .await
        .expect("tentativa de substituir cover com arquivo inválido");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let cover_after_invalid: (Option<String>,) =
        sqlx::query_as("SELECT cover_asset_id FROM events WHERE id=?1")
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("cover preservada após upload inválido");
    assert_eq!(
        cover_after_invalid.0.as_deref(),
        Some(cover_asset.asset_id.as_str())
    );

    let private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&cover_asset.variants["cover"])
                .header(
                    "Cookie",
                    format!("{}={other_token}", crate::security::session_cookie_name()),
                )
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("request private asset"),
        )
        .await
        .expect("private asset response");
    assert_eq!(private.status(), StatusCode::BAD_REQUEST);

    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .expect("publicar evento in-process");
    let public = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&cover_asset.variants["cover"])
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("request public asset"),
        )
        .await
        .expect("public asset response");
    assert_eq!(public.status(), StatusCode::OK);
    assert_eq!(public.headers()["content-type"], "image/webp");
    assert!(public.headers()["cache-control"]
        .to_str()
        .expect("cache header")
        .contains("immutable"));

    let removed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/custom/events/{event_id}/cover/remove"))
                .header(
                    "Cookie",
                    format!("{}={owner_token}", crate::security::session_cookie_name()),
                )
                .header("X-CSRF-Token", &owner_csrf)
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("remover cover publicada pelo dono"),
        )
        .await
        .expect("remover cover publicada");
    assert_eq!(
        removed.status(),
        StatusCode::BAD_REQUEST,
        "mídia publicada só pode ser alterada por administrador"
    );
}
