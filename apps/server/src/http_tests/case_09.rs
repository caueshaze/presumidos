use super::*;

#[tokio::test]
async fn vma_manifest_imports_as_generic_custom_event_without_matches() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let slug = format!("vma-{}", suffix.simple());
    let _admin = seed_user(
        &format!("vma-admin-{suffix}"),
        &format!("vma-admin-{suffix}@test"),
        "senha-correta-123",
        true,
    )
    .await;
    let mut manifest = crate::custom_event_manifest::parse_and_validate(include_str!(
        "../../resources/events/vma-2026.json"
    ))
    .expect("manifesto VMA válido");
    manifest.slug = slug.clone();
    let summary = crate::custom_event_manifest::import(manifest, true)
        .await
        .expect("importação VMA idempotente");
    assert_eq!(summary, (19, 121));
    let (event_id, actor, version_id): (String, String, String) = sqlx::query_as(
        "SELECT e.id,u.id,v.id FROM events e JOIN users u ON u.is_admin=1 JOIN event_versions v ON v.event_id=e.id AND v.state='working' WHERE e.slug=?1 LIMIT 1",
    )
    .bind(&slug)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_as::<_, (String,)>("SELECT status FROM events WHERE id=?1")
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap()
            .0,
        "draft"
    );
    crate::custom_event_manifest::publish_working_revision(&event_id, Some(&version_id), &actor)
        .await
        .unwrap();
    let mut second_manifest = crate::custom_event_manifest::parse_and_validate(include_str!(
        "../../resources/events/vma-2026.json"
    ))
    .unwrap();
    second_manifest.slug = slug.clone();
    assert_eq!(
        crate::custom_event_manifest::import(second_manifest, true)
            .await
            .unwrap(),
        (19, 121),
        "reimportação idempotente contra a versão publicada"
    );
    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM prediction_items pi JOIN events e ON e.id=pi.event_id WHERE e.slug=?1),
           (SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id JOIN events e ON e.id=pi.event_id WHERE e.slug=?1),
           (SELECT COUNT(*) FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id JOIN events e ON e.id=pi.event_id WHERE e.slug=?1)",
    )
    .bind(&slug)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(counts, (19, 121, 0));
    let suffix = uuid::Uuid::new_v4();
    let user = seed_user(
        &format!("vma-{suffix}"),
        &format!("vma-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    let pool_id = uuid::Uuid::new_v4().to_string();
    let published_version: (String,) = sqlx::query_as(
        "SELECT current_published_version_id FROM events WHERE id=?1 AND current_published_version_id IS NOT NULL",
    )
    .bind(&event.0)
    .fetch_one(crate::db::pool())
    .await
    .expect("versão publicada VMA");
    sqlx::query(
        "INSERT INTO pools(id,event_id,event_version_id,name,invite_code,created_by) VALUES(?1,?2,?3,?4,?5,?6)",
    )
    .bind(&pool_id)
    .bind(&event.0)
    .bind(&published_version.0)
    .bind("VMA smoke")
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(&user)
    .execute(crate::db::pool())
    .await
    .unwrap();
    add_membership(&pool_id, &user).await;
    let configs: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM custom_pool_item_scoring WHERE pool_id=?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(configs.0, 19, "pool novo materializa scoring por item");

    let (token, csrf) = seed_session(&user).await;
    let client = client_with_session(base, &token);
    let pools: Vec<crate::models::PoolSummary> = client
        .get(format!("{base}/api/pools"))
        .send()
        .await
        .expect("listar pools VMA")
        .json()
        .await
        .expect("JSON de pools VMA");
    let vma_pool = pools
        .iter()
        .find(|pool| pool.id == pool_id)
        .expect("pool VMA");
    assert_eq!(vma_pool.event.kind, crate::models::EventKind::Custom);
    assert_eq!(vma_pool.event.slug, slug);

    let available: Vec<crate::models::Event> = client
        .get(format!("{base}/api/custom/events/available"))
        .send()
        .await
        .expect("listar eventos publicados")
        .json()
        .await
        .expect("JSON de eventos publicados");
    assert!(available.iter().any(|event| event.slug == slug));

    let questions: Vec<crate::models::CustomQuestion> = client
        .get(format!("{base}/api/custom/questions?poolId={pool_id}"))
        .send()
        .await
        .expect("carregar perguntas VMA")
        .json()
        .await
        .expect("JSON de perguntas VMA");
    assert_eq!(questions.len(), 19);
    assert_eq!(
        questions
            .iter()
            .map(|question| question.options.len())
            .sum::<usize>(),
        121
    );
    assert!(questions
        .iter()
        .all(|question| question.kind == crate::models::PredictionItemKind::SingleChoice));
    assert!(questions
        .windows(2)
        .all(|pair| pair[0].sort_order <= pair[1].sort_order));
    assert_eq!(
        questions
            .iter()
            .flat_map(|question| question.options.iter())
            .filter(|option| !option.links.is_empty())
            .count(),
        4,
        "links editoriais do manifesto não duplicam nem exigem tipo especial",
    );
    let media_option = questions
        .iter()
        .flat_map(|question| question.options.iter())
        .find(|option| !option.links.is_empty())
        .unwrap();
    let media_response = client
        .post(format!("{base}/api/custom/media-progress"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":pool_id,"optionId":media_option.id,"seen":true}))
        .send()
        .await
        .unwrap();
    assert!(media_response.status().is_success());
    let progress: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM option_media_progress WHERE user_id=?1 AND option_id=?2",
    )
    .bind(&user)
    .bind(&media_option.id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(progress.0, 1);
    let predictions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&user)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(predictions.0, 0, "checklist de mídia não cria Prediction");
    let other = seed_user(
        &format!("vma-other-{suffix}"),
        &format!("vma-other-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    add_membership(&pool_id, &other).await;
    let (other_token, _) = seed_session(&other).await;
    let other_questions: Vec<crate::models::CustomQuestion> =
        client_with_session(base, &other_token)
            .get(format!("{base}/api/custom/questions?poolId={pool_id}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    assert!(
        !other_questions
            .iter()
            .flat_map(|question| question.options.iter())
            .any(|option| option.id == media_option.id && option.media_seen),
        "progresso de mídia é isolado por usuário"
    );
}
