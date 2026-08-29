use super::*;

#[tokio::test]
async fn contextual_asset_upload_deduplicates_and_enforces_draft_privacy() {
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;

    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner = seed_user(
        &format!("asset-owner-{suffix}"),
        &format!("asset-owner-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let other = seed_user(
        &format!("asset-other-{suffix}"),
        &format!("asset-other-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let event_id = uuid::Uuid::new_v4().to_string();
    let slug = format!("asset-smoke-{suffix}");
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by) VALUES(?1,?2,?3,'custom','draft',?4)")
        .bind(&event_id)
        .bind("Asset smoke")
        .bind(&slug)
        .bind(&owner)
        .execute(crate::db::pool())
    .await
    .expect("evento draft de asset");
    sqlx::query("INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,created_by) VALUES(?1,?2,1,'working',0,?3,?4)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&event_id)
        .bind("Asset smoke")
        .bind(&owner)
        .execute(crate::db::pool())
        .await
        .expect("versão working de asset");
    let (item_id, options) = insert_custom_question(
        &event_id,
        "Imagem editorial",
        "2099-01-01T00:00:00Z",
        "2099-01-02T00:00:00Z",
        &["A", "B"],
    )
    .await;
    let (owner_token, owner_csrf) = seed_session(&owner).await;
    let (other_token, other_csrf) = seed_session(&other).await;
    let owner_client = client_with_session(base, &owner_token);
    let other_client = client_with_session(base, &other_token);

    let fixture_digest = sha2::Sha256::digest(suffix.as_bytes());
    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
        24,
        12,
        Rgb([fixture_digest[0], fixture_digest[1], fixture_digest[2]]),
    ));
    let mut image_bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut image_bytes, ImageOutputFormat::Png)
        .expect("fixture png");
    let png = image_bytes.into_inner();

    let upload = |path: String, csrf: String, bytes: Vec<u8>| async {
        owner_client
            .post(path)
            .header("X-CSRF-Token", csrf)
            .multipart(
                reqwest::multipart::Form::new().part(
                    "file",
                    reqwest::multipart::Part::bytes(bytes)
                        .file_name("editorial.png")
                        .mime_str("image/png")
                        .expect("mime png"),
                ),
            )
            .send()
            .await
            .expect("upload de asset")
    };
    let cover = upload(
        format!("{base}/api/custom/events/{event_id}/cover"),
        owner_csrf.clone(),
        png.clone(),
    )
    .await;
    assert!(cover.status().is_success(), "upload cover");
    let cover_asset: crate::assets::AssetResponse = cover.json().await.expect("asset cover");

    let denied = other_client
        .post(format!("{base}/api/custom/events/{event_id}/cover"))
        .header("X-CSRF-Token", &other_csrf)
        .multipart(
            reqwest::multipart::Form::new().part(
                "file",
                reqwest::multipart::Part::bytes(png.clone())
                    .file_name("editorial.png")
                    .mime_str("image/png")
                    .unwrap(),
            ),
        )
        .send()
        .await
        .expect("upload negado");
    assert!(!denied.status().is_success(), "outro dono não edita draft");

    let option_upload = upload(
        format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options/{}/image",
            options[0]
        ),
        owner_csrf.clone(),
        png.clone(),
    )
    .await;
    assert!(option_upload.status().is_success(), "upload option");
    let option_asset: crate::assets::AssetResponse = option_upload.json().await.unwrap();
    assert_eq!(
        cover_asset.asset_id, option_asset.asset_id,
        "dedup por hash"
    );
    let assets_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets WHERE sha256=?1")
        .bind(&cover_asset.sha256)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert_eq!(assets_count.0, 1);

    let private = other_client
        .get(format!("{base}{}", cover_asset.variants["cover"]))
        .send()
        .await
        .expect("serving draft privado");
    assert!(!private.status().is_success());
    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let public = other_client
        .get(format!("{base}{}", cover_asset.variants["cover"]))
        .send()
        .await
        .expect("serving publicado");
    assert_eq!(public.status(), reqwest::StatusCode::OK);
    assert_eq!(public.headers()["content-type"], "image/webp");
    assert!(public.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("immutable"));

    let removed = owner_client
        .post(format!("{base}/api/custom/events/{event_id}/cover/remove"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("remover cover");
    assert_eq!(
        removed.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "mídia publicada só pode ser alterada por administrador"
    );
    let still_stored: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets WHERE id=?1")
        .bind(&cover_asset.asset_id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert_eq!(still_stored.0, 1, "remove desassocia, não apaga asset");
}
