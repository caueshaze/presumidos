use super::*;

/// Apagar bolão: o criador consegue; um membro comum não. Os registros filhos
/// (membros, ajustes) somem junto, e os palpites globais do usuário permanecem.
#[tokio::test]
async fn pool_creator_can_delete_pool() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let creator_email = format!("del-creator-{suffix}@teste.com");
    let member_email = format!("del-member-{suffix}@teste.com");
    let creator_id = seed_user(
        &format!("delcreator-{suffix}"),
        &creator_email,
        "senha-correta-123",
        false,
    )
    .await;
    let member_id = seed_user(
        &format!("delmember-{suffix}"),
        &member_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &creator_id).await;
    add_membership(&pool_id, &creator_id).await;
    add_membership(&pool_id, &member_id).await;

    let del_url = format!("{base}/api/pools/{pool_id}/delete");

    // Membro comum NÃO pode apagar.
    let (member_token, member_csrf) = seed_session(&member_id).await;
    let member_c = client_with_session(base, &member_token);
    let denied = member_c
        .post(&del_url)
        .header("X-CSRF-Token", &member_csrf)
        .send()
        .await
        .expect("delete por membro comum");
    assert!(
        !denied.status().is_success(),
        "membro comum nao deveria apagar"
    );

    // Pool e membros continuam existindo após a tentativa barrada.
    let still_there: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE id = ?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("contar pool");
    assert_eq!(still_there.0, 1);

    // Criador apaga.
    let (creator_token, creator_csrf) = seed_session(&creator_id).await;
    let creator_c = client_with_session(base, &creator_token);
    let deleted = creator_c
        .post(&del_url)
        .header("X-CSRF-Token", &creator_csrf)
        .send()
        .await
        .expect("delete pelo criador");
    assert!(
        deleted.status().is_success(),
        "criador deveria poder apagar"
    );

    // Pool e pool_members somem; nenhum órfão.
    let pools_left: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE id = ?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("contar pool apos delete");
    assert_eq!(pools_left.0, 0, "bolao deveria ter sido apagado");

    let members_left: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id = ?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar membros apos delete");
    assert_eq!(
        members_left.0, 0,
        "membros do bolao deveriam ter sido removidos"
    );
}
