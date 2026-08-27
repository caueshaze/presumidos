use super::*;

fn package_smoke_manifest(name: &str, slug: &str, asset_sha: &str) -> String {
    serde_json::json!({
        "schemaVersion": 2,
        "name": name,
        "slug": slug,
        "kind": "custom",
        "description": "Evento de promoção com imagem interna.",
        "externalUrl": "https://example.test/evento",
        "startsAt": "2099-01-01T00:00:00Z",
        "endsAt": "2099-01-02T00:00:00Z",
        "coverAsset": {"kind": "asset", "sha256": asset_sha, "mediaType": "image/webp"},
        "items": [
            {
                "externalKey": "choice",
                "kind": "single_choice",
                "title": "Escolha",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "options": [
                    {"externalKey": "a", "label": "A", "imageAsset": {"kind": "asset", "sha256": asset_sha, "mediaType": "image/webp"}, "links": [{"kind": "official", "label": "Site oficial", "url": "https://example.test/a"}]},
                    {"externalKey": "b", "label": "B"}
                ]
            },
            {
                "externalKey": "number",
                "kind": "numeric",
                "title": "Número",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "decimalPlaces": 1,
                "unitLabel": "pontos",
                "minValue": "0.0",
                "maxValue": "10.0"
            },
            {
                "externalKey": "multiple",
                "kind": "multiple_choice",
                "title": "Múltipla",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "minSelections": 1,
                "maxSelections": 2,
                "options": [
                    {"externalKey": "x", "label": "X"},
                    {"externalKey": "y", "label": "Y"}
                ]
            }
        ]
    })
    .to_string()
}

fn vma_smoke_manifest(asset_sha: &str) -> crate::custom_event_manifest::CustomEventManifest {
    let mut manifest = crate::custom_event_manifest::parse_and_validate(include_str!(
        "../../data/events/vma-2026.json"
    ))
    .expect("manifesto VMA do smoke");
    manifest.schema_version = crate::custom_event_manifest::CURRENT_SCHEMA_VERSION;
    let asset = crate::custom_event_manifest::AssetReference {
        kind: "asset".into(),
        sha256: asset_sha.into(),
        media_type: "image/webp".into(),
    };
    manifest.cover_asset = Some(asset.clone());
    for item in &mut manifest.items {
        if matches!(
            item.external_key.as_str(),
            "video-of-the-year" | "best-pop" | "best-k-pop"
        ) {
            if let Some(option) = item.options.first_mut() {
                option.image_asset = Some(asset.clone());
            }
        }
    }
    manifest
}

fn package_smoke_master(rgb: [u8; 3]) -> Vec<u8> {
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;

    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(24, 12, Rgb(rgb)));
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageOutputFormat::WebP)
        .expect("master de smoke");
    output.into_inner()
}

fn package_smoke_root() -> PathBuf {
    PathBuf::from(
        std::env::var("PACKAGE_SMOKE_ROOT")
            .expect("PACKAGE_SMOKE_ROOT obrigatório no smoke de pacote"),
    )
}

fn package_smoke_env(db: &Path, assets: &Path) {
    seed_http_test_env();
    std::env::set_var("DATABASE_PATH", db.to_string_lossy().to_string());
    std::env::set_var("PRESUMIDOS_ASSET_DIR", assets.to_string_lossy().to_string());
}

fn package_smoke_package(root: &Path, name: &str) -> Vec<u8> {
    fs::read(root.join(name)).expect("pacote de smoke")
}

fn package_smoke_structural_view(
    mut manifest: crate::custom_event_manifest::CustomEventManifest,
) -> serde_json::Value {
    manifest.name.clear();
    manifest.description = None;
    manifest.cover_url = None;
    manifest.cover_asset = None;
    manifest.external_url = None;
    for item in &mut manifest.items {
        for option in &mut item.options {
            option.image_url = None;
            option.image_asset = None;
            option.links.clear();
        }
    }
    serde_json::to_value(manifest).expect("estrutura de manifesto")
}

fn package_smoke_entry_names(bytes: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip smoke");
    let mut names = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .expect("entrada zip")
                .name()
                .to_string()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn package_smoke_without_assets(slug: &str) -> Vec<u8> {
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "name": "Pacote HTTP",
        "slug": slug,
        "kind": "custom",
        "items": [{
            "externalKey": "choice",
            "kind": "single_choice",
            "title": "Escolha",
            "lockAt": "2099-01-01T00:00:00Z",
            "revealAt": "2099-01-02T00:00:00Z",
            "options": [{"externalKey": "a", "label": "A"}, {"externalKey": "b", "label": "B"}]
        }]
    })
    .to_string();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file(
            "manifest.json",
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .expect("manifest HTTP");
    std::io::Write::write_all(&mut writer, manifest.as_bytes()).expect("manifest bytes HTTP");
    writer.finish().expect("zip HTTP").into_inner()
}

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
    let json_request = |path: String, payload: serde_json::Value| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/json")
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", &csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(payload.to_string()))
            .expect("request JSON builder")
    };
    let response_json = |response: axum::response::Response| async move {
        let status = response.status();
        let body = to_bytes(response.into_body(), 2_000_000)
            .await
            .expect("body JSON builder");
        assert!(
            status.is_success(),
            "resposta builder: {status} {}",
            String::from_utf8_lossy(&body)
        );
        serde_json::from_slice::<serde_json::Value>(&body).expect("JSON builder")
    };
    let created = response_json(
        app.clone()
            .oneshot(json_request(
                "/api/custom/events".into(),
                json!({"name":"Evento com imagens","startsAt":null,"endsAt":null}),
            ))
            .await
            .expect("criar event builder"),
    )
    .await;
    let event_id = created["id"]
        .as_str()
        .expect("id event builder")
        .to_string();
    let lock = "2099-01-01T00:00:00Z";
    let reveal = "2099-01-02T00:00:00Z";
    let add_item = |path: String, payload: serde_json::Value| async {
        response_json(
            app.clone()
                .oneshot(json_request(path, payload))
                .await
                .expect("add item builder"),
        )
        .await["id"]
            .as_str()
            .expect("id item builder")
            .to_string()
    };
    let single_id = add_item(
        format!("/api/custom/events/{event_id}/items"),
        json!({"title":"Escolha","lockAt":lock,"revealAt":reveal}),
    )
    .await;
    let numeric_id = add_item(
        format!("/api/custom/events/{event_id}/items/numeric"),
        json!({"title":"Número","lockAt":lock,"revealAt":reveal,"decimalPlaces":1,"unitLabel":"pontos","minValue":"0.0","maxValue":"10.0"}),
    )
    .await;
    let multiple_id = add_item(
        format!("/api/custom/events/{event_id}/items/multiple-choice"),
        json!({"title":"Múltipla","lockAt":lock,"revealAt":reveal,"minSelections":1,"maxSelections":2}),
    )
    .await;
    let add_option = |item_id: String, label: String| {
        let app = app.clone();
        let request = json_request(
            format!("/api/custom/events/{event_id}/items/{item_id}/options"),
            json!({"label": label}),
        );
        async move {
            let response = app.oneshot(request).await.expect("add option builder");
            let status = response.status();
            let body = to_bytes(response.into_body(), 2_000_000)
                .await
                .expect("option body builder");
            assert!(
                status.is_success(),
                "resposta option builder: {status} {}",
                String::from_utf8_lossy(&body)
            );
            let value: serde_json::Value =
                serde_json::from_slice(&body).expect("option JSON builder");
            value["id"].as_str().expect("id option builder").to_string()
        }
    };
    let single_option = add_option(single_id.clone(), "A".into()).await;
    add_option(single_id.clone(), "B".into()).await;
    add_option(multiple_id.clone(), "X".into()).await;
    add_option(multiple_id, "Y".into()).await;

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

async fn run_package_smoke_stage(stage: &str) {
    let root = package_smoke_root();
    let dev_db = root.join("dev.db");
    let prod_db = root.join("prod.db");
    let dev_assets = root.join("dev-assets");
    let prod_assets = root.join("prod-assets");
    let dev_slug = "package-promotion-smoke";
    match stage {
        "dev-a" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor = seed_user(
                "package-smoke-dev",
                "package-smoke-dev@example.test",
                "Senha-forte-123",
                true,
            )
            .await;
            let master = package_smoke_master([20, 40, 80]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor)
                .await
                .expect("asset DEV");
            let content = package_smoke_manifest("Evento de promoção", dev_slug, &hash);
            let preview = crate::custom_event_manifest::preview(&content)
                .await
                .expect("preview DEV");
            assert_eq!(
                preview.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            let result = crate::custom_event_manifest::apply_admin(
                &content,
                &preview.base_fingerprint,
                &actor,
            )
            .await
            .expect("apply DEV");
            let event_id = result.event_id.expect("event DEV");
            let package = crate::event_package::export(&event_id)
                .await
                .expect("export A");
            fs::write(root.join("package-a.zip"), package).expect("gravar package A");
        }
        "prod-a" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor = seed_user(
                "package-smoke-prod",
                "package-smoke-prod@example.test",
                "Senha-forte-123",
                true,
            )
            .await;
            let package = package_smoke_package(&root, "package-a.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview Create");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            assert_eq!(
                (
                    preview.manifest.item_count,
                    preview.manifest.option_count,
                    preview.manifest.link_count
                ),
                (3, 4, 1)
            );
            assert_eq!(preview.added_asset_count, 1);
            let applied =
                crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor)
                    .await
                    .expect("apply Create");
            assert_eq!(
                applied.result.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            let event_id = applied.result.event_id.clone().expect("event PROD");
            sqlx::query("UPDATE events SET status='active' WHERE id=?1")
                .bind(&event_id)
                .execute(crate::db::pool())
                .await
                .expect("publicar PROD");
            let pool_id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO pools(id,event_id,name,invite_code,created_by) VALUES(?1,?2,'Pool de promoção','package-smoke-invite',?3)")
                .bind(&pool_id)
                .bind(&event_id)
                .bind(&actor)
                .execute(crate::db::pool())
                .await
                .expect("pool de promoção");
            sqlx::query("INSERT INTO pool_members(pool_id,user_id) VALUES(?1,?2)")
                .bind(&pool_id)
                .bind(&actor)
                .execute(crate::db::pool())
                .await
                .expect("membro do pool de promoção");
            let item_id: (String,) = sqlx::query_as(
                "SELECT id FROM prediction_items WHERE event_id=?1 AND external_key='choice'",
            )
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("item do pool de promoção");
            let option_id: (String,) = sqlx::query_as(
                "SELECT id FROM custom_question_options WHERE item_id=?1 AND external_key='a'",
            )
            .bind(&item_id.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("opção do pool de promoção");
            let prediction_id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO predictions(id,pool_id,user_id,item_id,match_id,home_score,away_score) VALUES(?1,?2,?3,?4,NULL,NULL,NULL)")
                .bind(&prediction_id)
                .bind(&pool_id)
                .bind(&actor)
                .bind(&item_id.0)
                .execute(crate::db::pool())
                .await
                .expect("prediction de promoção");
            sqlx::query(
                "INSERT INTO custom_prediction_values(prediction_id,option_id) VALUES(?1,?2)",
            )
            .bind(&prediction_id)
            .bind(&option_id.0)
            .execute(crate::db::pool())
            .await
            .expect("valor do prediction de promoção");
            let repeat = crate::event_package::preview(&package)
                .await
                .expect("preview NoChange");
            assert_eq!(
                repeat.manifest.action,
                crate::custom_event_manifest::ImportAction::NoChange
            );
            assert_eq!(repeat.existing_asset_count, 1);
            assert_eq!(repeat.added_asset_count, 0);
            let repeated =
                crate::event_package::apply(&package, &repeat.manifest.base_fingerprint, &actor)
                    .await
                    .expect("apply NoChange");
            assert_eq!(
                repeated.result.action,
                crate::custom_event_manifest::ImportAction::NoChange
            );
            let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM events WHERE slug=?1),(SELECT COUNT(*) FROM prediction_items WHERE event_id=?2),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?2),(SELECT COUNT(*) FROM assets)")
                .bind(dev_slug).bind(&event_id).fetch_one(crate::db::pool()).await.expect("contagens PROD");
            assert_eq!(counts, (1, 3, 4, 1));
            let prod_package = crate::event_package::export(&event_id)
                .await
                .expect("export PROD A");
            assert_eq!(
                package_smoke_entry_names(&package),
                package_smoke_entry_names(&prod_package)
            );
            let dev_parsed = crate::event_package::parse(&package).expect("manifest DEV A");
            let prod_parsed = crate::event_package::parse(&prod_package).expect("manifest PROD A");
            assert_eq!(dev_parsed.manifest, prod_parsed.manifest);
            assert_eq!(
                dev_parsed
                    .assets
                    .keys()
                    .collect::<std::collections::HashSet<_>>(),
                prod_parsed
                    .assets
                    .keys()
                    .collect::<std::collections::HashSet<_>>()
            );
            fs::write(root.join("package-prod-a.zip"), prod_package).expect("gravar round-trip");
        }
        "dev-b" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-dev'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor DEV");
            let master = package_smoke_master([120, 20, 40]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor.0)
                .await
                .expect("asset B");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event DEV B");
            let asset: (String,) = sqlx::query_as("SELECT id FROM assets WHERE sha256=?1")
                .bind(&hash)
                .fetch_one(crate::db::pool())
                .await
                .expect("asset id B");
            sqlx::query("UPDATE events SET name='Evento de promoção atualizado',description='Descrição editorial B',cover_asset_id=?2 WHERE id=?1")
                .bind(&event.0).bind(&asset.0).execute(crate::db::pool()).await.expect("metadata B");
            let option: (String,) = sqlx::query_as("SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?1 AND o.external_key='a'").bind(&event.0).fetch_one(crate::db::pool()).await.expect("option B");
            sqlx::query("UPDATE custom_question_options SET image_asset_id=?2 WHERE id=?1")
                .bind(&option.0)
                .bind(&asset.0)
                .execute(crate::db::pool())
                .await
                .expect("option asset B");
            let package = crate::event_package::export(&event.0)
                .await
                .expect("export B");
            fs::write(root.join("package-b.zip"), package).expect("gravar package B");
        }
        "prod-b" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor PROD B");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event PROD B");
            let before = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest antes B")
                .0;
            let used_before: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM pools WHERE event_id=?1),(SELECT COUNT(*) FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)")
                .bind(&event.0)
                .fetch_one(crate::db::pool())
                .await
                .expect("uso antes B");
            assert_eq!(used_before, (1, 1));
            let package = package_smoke_package(&root, "package-b.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview SafeUpdate");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::SafeUpdate
            );
            assert!(!preview.manifest.safe_changes.is_empty());
            assert!(preview.manifest.blocked_changes.is_empty());
            assert_eq!(preview.added_asset_count, 1);
            let result =
                crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
                    .await
                    .expect("apply SafeUpdate");
            assert_eq!(
                result.result.action,
                crate::custom_event_manifest::ImportAction::SafeUpdate
            );
            let after = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest depois B")
                .0;
            assert_eq!(
                package_smoke_structural_view(before),
                package_smoke_structural_view(after.clone())
            );
            assert_eq!(after.name, "Evento de promoção atualizado");
            assert_eq!(after.description.as_deref(), Some("Descrição editorial B"));
            let asset_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets")
                .fetch_one(crate::db::pool())
                .await
                .expect("assets B");
            assert_eq!(asset_count.0, 2);
            let used_after: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM pools WHERE event_id=?1),(SELECT COUNT(*) FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)")
                .bind(&event.0)
                .fetch_one(crate::db::pool())
                .await
                .expect("uso depois B");
            assert_eq!(used_after, used_before);
        }
        "dev-c" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event DEV C");
            let item: (String,) = sqlx::query_as(
                "SELECT id FROM prediction_items WHERE event_id=?1 AND external_key='choice'",
            )
            .bind(&event.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("item C");
            sqlx::query(
                "UPDATE prediction_items SET title='Título estrutural incompatível' WHERE id=?1",
            )
            .bind(&item.0)
            .execute(crate::db::pool())
            .await
            .expect("alteração estrutural C");
            let package = crate::event_package::export(&event.0)
                .await
                .expect("export C");
            fs::write(root.join("package-c.zip"), package).expect("gravar package C");
        }
        "prod-c" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor PROD C");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event PROD C");
            let package_b = package_smoke_package(&root, "package-b.zip");
            let stale_preview = crate::event_package::preview(&package_b)
                .await
                .expect("preview stale");
            sqlx::query("UPDATE events SET description='alteração concorrente' WHERE id=?1")
                .bind(&event.0)
                .execute(crate::db::pool())
                .await
                .expect("concorrência");
            assert!(crate::event_package::apply(
                &package_b,
                &stale_preview.manifest.base_fingerprint,
                &actor.0
            )
            .await
            .is_err());
            let package = package_smoke_package(&root, "package-c.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview Conflict");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::Conflict
            );
            assert!(preview
                .manifest
                .blocked_changes
                .iter()
                .any(|change| change.path.contains("title")));
            let before = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest antes C")
                .0;
            assert!(crate::event_package::apply(
                &package,
                &preview.manifest.base_fingerprint,
                &actor.0
            )
            .await
            .is_err());
            let after = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest depois C")
                .0;
            assert_eq!(before, after);
        }
        "vma-dev-a" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-dev'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA DEV");
            let master = package_smoke_master([30, 100, 180]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor.0)
                .await
                .expect("asset VMA DEV");
            let manifest = vma_smoke_manifest(&hash);
            let content = serde_json::to_string(&manifest).expect("serializar VMA DEV");
            let preview = crate::custom_event_manifest::preview(&content)
                .await
                .expect("preview VMA Create");
            assert_eq!(
                preview.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            assert_eq!(
                (preview.item_count, preview.option_count, preview.link_count),
                (19, 121, 4)
            );
            let result = crate::custom_event_manifest::apply_admin(
                &content,
                &preview.base_fingerprint,
                &actor.0,
            )
            .await
            .expect("apply VMA DEV");
            let event_id = result.event_id.expect("event VMA DEV");
            let package = crate::event_package::export(&event_id)
                .await
                .expect("export VMA A");
            fs::write(root.join("vma-package-a.zip"), package).expect("gravar VMA A");
        }
        "vma-prod-a" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA PROD");
            let package = package_smoke_package(&root, "vma-package-a.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview VMA PROD Create");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            assert_eq!(
                (
                    preview.manifest.item_count,
                    preview.manifest.option_count,
                    preview.manifest.link_count
                ),
                (19, 121, 4)
            );
            assert_eq!(preview.asset_count, 1);
            let applied =
                crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
                    .await
                    .expect("apply VMA PROD");
            let event_id = applied.result.event_id.expect("event VMA PROD");
            sqlx::query("UPDATE events SET status='active' WHERE id=?1")
                .bind(&event_id)
                .execute(crate::db::pool())
                .await
                .expect("publicar VMA PROD");
            let repeat = crate::event_package::preview(&package)
                .await
                .expect("preview VMA NoChange");
            assert_eq!(
                repeat.manifest.action,
                crate::custom_event_manifest::ImportAction::NoChange
            );
            let prod_package = crate::event_package::export(&event_id)
                .await
                .expect("export VMA PROD A");
            let dev_parsed = crate::event_package::parse(&package).expect("parse VMA DEV A");
            let prod_parsed = crate::event_package::parse(&prod_package).expect("parse VMA PROD A");
            assert_eq!(dev_parsed.manifest, prod_parsed.manifest);
            assert_eq!(
                dev_parsed.assets.keys().collect::<HashSet<_>>(),
                prod_parsed.assets.keys().collect::<HashSet<_>>()
            );
        }
        "vma-dev-b" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-dev'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA DEV B");
            let master = package_smoke_master([180, 80, 30]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor.0)
                .await
                .expect("asset VMA B");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug='vma-2026'")
                .fetch_one(crate::db::pool())
                .await
                .expect("event VMA DEV B");
            let mut manifest = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest VMA DEV B")
                .0;
            let base_fingerprint = crate::custom_event_manifest::fingerprint(&manifest)
                .expect("base fingerprint VMA B");
            let asset = crate::custom_event_manifest::AssetReference {
                kind: "asset".into(),
                sha256: hash,
                media_type: "image/webp".into(),
            };
            manifest.cover_asset = Some(asset.clone());
            let option = manifest
                .items
                .iter_mut()
                .find(|item| item.external_key == "best-pop")
                .and_then(|item| item.options.first_mut())
                .expect("opção VMA B");
            option.image_asset = Some(asset);
            let content =
                crate::custom_event_manifest::canonical_json(&manifest).expect("serializar VMA B");
            crate::custom_event_manifest::apply_admin(&content, &base_fingerprint, &actor.0)
                .await
                .expect("aplicar alteração VMA DEV B");
            let package = crate::event_package::export(&event.0)
                .await
                .expect("export VMA B");
            fs::write(root.join("vma-package-b.zip"), package).expect("gravar VMA B");
        }
        "vma-prod-b" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA PROD B");
            let package = package_smoke_package(&root, "vma-package-b.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview VMA SafeUpdate");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::SafeUpdate
            );
            assert_eq!(
                (
                    preview.manifest.item_count,
                    preview.manifest.option_count,
                    preview.manifest.link_count
                ),
                (19, 121, 4)
            );
            assert!(preview.manifest.blocked_changes.is_empty());
            assert_eq!(preview.added_asset_count, 1);
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug='vma-2026'")
                .fetch_one(crate::db::pool())
                .await
                .expect("event VMA PROD B");
            crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
                .await
                .expect("apply VMA SafeUpdate");
            let after = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest VMA depois B")
                .0;
            assert_eq!(
                (
                    after.items.len(),
                    after
                        .items
                        .iter()
                        .map(|item| item.options.len())
                        .sum::<usize>()
                ),
                (19, 121)
            );
            assert_eq!(
                after
                    .items
                    .iter()
                    .map(|item| item
                        .options
                        .iter()
                        .map(|option| option.links.len())
                        .sum::<usize>())
                    .sum::<usize>(),
                4
            );
            let asset_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets")
                .fetch_one(crate::db::pool())
                .await
                .expect("assets VMA PROD B");
            assert_eq!(
                asset_count.0, 4,
                "2 assets do smoke genérico + 2 assets do VMA"
            );
        }
        _ => panic!("stage de package smoke desconhecido: {stage}"),
    }
}

#[tokio::test]
#[ignore = "smoke explícito usa processos e dois bancos SQLite independentes"]
async fn package_promotion_two_sqlite_smoke() {
    if let Ok(stage) = std::env::var("PACKAGE_SMOKE_STAGE") {
        run_package_smoke_stage(&stage).await;
        return;
    }
    let root =
        std::env::temp_dir().join(format!("presumidos-package-smoke-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("diretório do smoke");
    let executable = std::env::current_exe().expect("binário de testes");
    for stage in [
        "dev-a",
        "prod-a",
        "dev-b",
        "prod-b",
        "dev-c",
        "prod-c",
        "vma-dev-a",
        "vma-prod-a",
        "vma-dev-b",
        "vma-prod-b",
    ] {
        let status = Command::new(&executable)
            .arg("--exact")
            .arg("http_tests::package_promotion_two_sqlite_smoke")
            .arg("--ignored")
            .arg("--nocapture")
            .env("PACKAGE_SMOKE_ROOT", &root)
            .env("PACKAGE_SMOKE_STAGE", stage)
            .status()
            .expect("iniciar etapa do smoke");
        assert!(status.success(), "etapa {stage} falhou");
    }
    let _ = fs::remove_dir_all(root);
}
