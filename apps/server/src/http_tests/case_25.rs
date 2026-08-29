use super::*;

#[tokio::test]
async fn pool_member_can_leave_preserving_data_and_rejoin() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner_id = seed_user(
        &format!("leave-owner-{suffix}"),
        &format!("leave-owner-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let member_id = seed_user(
        &format!("leave-member-{suffix}"),
        &format!("leave-member-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let (event_id, pool_id) =
        insert_custom_event_pool(&owner_id, &format!("Bolao leave {suffix}")).await;
    add_membership(&pool_id, &member_id).await;
    let item_id = uuid::Uuid::new_v4().to_string();
    let option_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO prediction_items(id,event_id,kind,title,lock_at,reveal_at,sort_order,status)
         VALUES(?1,?2,'single_choice','Pergunta preservada','2099-01-01T00:00:00Z','2099-01-01T00:00:00Z',0,'open')",
    )
    .bind(&item_id)
    .bind(&event_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir item do bolao leave");
    sqlx::query("INSERT INTO custom_questions(item_id,points) VALUES(?1,1)")
        .bind(&item_id)
        .execute(crate::db::pool())
        .await
        .expect("inserir pergunta do bolao leave");
    sqlx::query(
        "INSERT INTO custom_question_options(id,item_id,label,sort_order)
         VALUES(?1,?2,'Opcao preservada',0)",
    )
    .bind(&option_id)
    .bind(&item_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir opcao do bolao leave");
    let prediction_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO predictions(id,pool_id,user_id,item_id,match_id,home_score,away_score)
         VALUES(?1,?2,?3,?4,NULL,NULL,NULL)",
    )
    .bind(&prediction_id)
    .bind(&pool_id)
    .bind(&member_id)
    .bind(&item_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir palpite do bolao leave");
    sqlx::query("INSERT INTO custom_prediction_values(prediction_id,option_id) VALUES(?1,?2)")
        .bind(&prediction_id)
        .bind(&option_id)
        .execute(crate::db::pool())
        .await
        .expect("inserir valor do palpite do bolao leave");
    let invite_code: (String,) = sqlx::query_as("SELECT invite_code FROM pools WHERE id=?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("ler codigo de convite");

    let (member_token, member_csrf) = seed_session(&member_id).await;
    let member = client_with_session(base, &member_token);
    let left = member
        .post(format!("{base}/api/pools/{pool_id}/leave"))
        .header("X-CSRF-Token", &member_csrf)
        .send()
        .await
        .expect("sair do bolao");
    assert!(left.status().is_success(), "membro deveria poder sair");

    let membership_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar membership apos saida");
    assert_eq!(membership_count.0, 0);
    let prediction_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar palpite preservado");
    assert_eq!(prediction_count.0, 1);

    let inaccessible = member
        .get(format!("{base}/api/pools/{pool_id}/member-predictions"))
        .send()
        .await
        .expect("acesso apos saida");
    assert!(!inaccessible.status().is_success());

    let rejoined = member
        .post(format!("{base}/api/pools/join"))
        .header("X-CSRF-Token", &member_csrf)
        .json(&json!({ "inviteCode": invite_code.0 }))
        .send()
        .await
        .expect("reentrar no bolao");
    assert!(
        rejoined.status().is_success(),
        "reingresso deveria funcionar"
    );
    let restored: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar palpite apos reingresso");
    assert_eq!(restored.0, 1);

    let (owner_token, owner_csrf) = seed_session(&owner_id).await;
    let owner = client_with_session(base, &owner_token);
    let owner_leave = owner
        .post(format!("{base}/api/pools/{pool_id}/leave"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("tentativa de saida do dono");
    assert!(!owner_leave.status().is_success(), "dono nao deveria sair");
}
