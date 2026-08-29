pub(crate) async fn insert_pool(name: &str, created_by: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let code = uuid::Uuid::new_v4().simple().to_string();
    let code = code[..8].to_uppercase();
    let event_id: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug = ?1")
        .bind(crate::events::WORLD_CUP_2026_SLUG)
        .fetch_one(crate::db::pool())
        .await
        .expect("evento da Copa seedado para fixture");
    sqlx::query("INSERT INTO pools (id, event_id, name, invite_code, created_by) VALUES (?1, ?2, ?3, ?4, ?5)")
        .bind(&id)
        .bind(&event_id.0)
        .bind(name)
        .bind(&code)
        .bind(created_by)
        .execute(crate::db::pool())
        .await
        .expect("inserir bolao de teste");
    id
}

pub(crate) async fn add_membership(pool_id: &str, user_id: &str) {
    sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(pool_id)
        .bind(user_id)
        .execute(crate::db::pool())
        .await
        .expect("inserir membro de teste");
}

/// Membership com `joined_at` explícito (para testar elegibilidade por data de entrada).
pub(crate) async fn add_membership_at(pool_id: &str, user_id: &str, joined_at: &str) {
    sqlx::query(
        "INSERT OR IGNORE INTO pool_members (pool_id, user_id, joined_at) VALUES (?1, ?2, ?3)",
    )
    .bind(pool_id)
    .bind(user_id)
    .bind(joined_at)
    .execute(crate::db::pool())
    .await
    .expect("inserir membro com joined_at");
}

/// Partida com resultado oficial já lançado (entra no cálculo do ranking).
pub(crate) async fn insert_finished_match(
    home: &str,
    away: &str,
    kickoff: &str,
    home_score: i64,
    away_score: i64,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let prediction_item_id = insert_prediction_item(home, away, kickoff).await;
    sqlx::query(
        "INSERT INTO matches (id, prediction_item_id, home_team, away_team, kickoff, group_name, phase,
                              home_score, away_score, finished)
         VALUES (?1, ?2, ?3, ?4, ?5, 'A', 'Fase de grupos', ?6, ?7, 1)",
    )
    .bind(&id)
    .bind(&prediction_item_id)
    .bind(home)
    .bind(away)
    .bind(kickoff)
    .bind(home_score)
    .bind(away_score)
    .execute(crate::db::pool())
    .await
    .expect("inserir partida finalizada");
    id
}

pub(crate) async fn insert_match(home: &str, away: &str, kickoff: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let prediction_item_id = insert_prediction_item(home, away, kickoff).await;
    sqlx::query(
        "INSERT INTO matches (id, prediction_item_id, home_team, away_team, kickoff, group_name, phase)
         VALUES (?1, ?2, ?3, ?4, ?5, 'A', 'Fase de grupos')",
    )
    .bind(&id)
    .bind(&prediction_item_id)
    .bind(home)
    .bind(away)
    .bind(kickoff)
    .execute(crate::db::pool())
    .await
    .expect("inserir partida de teste");
    id
}

pub(crate) async fn insert_custom_question(
    event_id: &str,
    title: &str,
    lock_at: &str,
    reveal_at: &str,
    labels: &[&str],
) -> (String, Vec<String>) {
    assert!(
        labels.len() >= 2,
        "pergunta single choice exige duas opções"
    );
    let item_id = uuid::Uuid::new_v4().to_string();
    let version_id: (String,) = sqlx::query_as(
        "SELECT COALESCE(current_published_version_id, (SELECT id FROM event_versions WHERE event_id=?1 ORDER BY version_number DESC LIMIT 1)) FROM events WHERE id=?1",
    )
    .bind(event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("versão custom");
    sqlx::query("INSERT INTO prediction_items (id,event_id,event_version_id,kind,title,lock_at,reveal_at,sort_order,status) VALUES (?1,?2,?3,'single_choice',?4,?5,?6,999,'open')")
        .bind(&item_id).bind(event_id).bind(&version_id.0).bind(title).bind(lock_at).bind(reveal_at).execute(crate::db::pool()).await.expect("item custom");
    sqlx::query("INSERT INTO custom_questions (item_id,points) VALUES (?1,1)")
        .bind(&item_id)
        .execute(crate::db::pool())
        .await
        .expect("pergunta custom");
    let mut ids = Vec::new();
    for (sort_order, label) in labels.iter().enumerate() {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO custom_question_options (id,item_id,label,sort_order) VALUES (?1,?2,?3,?4)").bind(&id).bind(&item_id).bind(label).bind(sort_order as i64).execute(crate::db::pool()).await.expect("opção custom");
        ids.push(id);
    }
    (item_id, ids)
}

pub(crate) async fn insert_custom_event_pool(owner: &str, name: &str) -> (String, String) {
    let event_id = uuid::Uuid::new_v4().to_string();
    let pool_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO events (id,name,slug,kind,status,created_by) VALUES (?1,?2,?3,'custom','active',?4)",
    )
    .bind(&event_id)
    .bind(name)
    .bind(format!("event-{event_id}"))
    .bind(owner)
    .execute(crate::db::pool())
    .await
    .expect("evento custom");
    let version_id = ensure_published_version(&event_id, name, owner).await;
    sqlx::query(
        "INSERT INTO pools (id,event_id,event_version_id,name,invite_code,created_by) VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(&pool_id)
    .bind(&event_id)
    .bind(&version_id)
    .bind(name)
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(owner)
    .execute(crate::db::pool())
    .await
    .expect("pool custom");
    add_membership(&pool_id, owner).await;
    (event_id, pool_id)
}

pub(crate) async fn ensure_published_version(event_id: &str, name: &str, owner: &str) -> String {
    if let Some((version_id,)) = sqlx::query_as::<_, (String,)>(
        "SELECT current_published_version_id FROM events WHERE id=?1 AND current_published_version_id IS NOT NULL",
    )
    .bind(event_id)
    .fetch_optional(crate::db::pool())
    .await
    .expect("ler versão publicada")
    {
        return version_id;
    }
    let version_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,created_by) VALUES(?1,?2,1,'published',1,?3,?4)")
        .bind(&version_id)
        .bind(event_id)
        .bind(name)
        .bind(owner)
        .execute(crate::db::pool())
        .await
        .expect("criar versão publicada");
    sqlx::query("UPDATE events SET current_published_version_id=?2 WHERE id=?1")
        .bind(event_id)
        .bind(&version_id)
        .execute(crate::db::pool())
        .await
        .expect("associar versão publicada");
    version_id
}

pub(crate) async fn insert_prediction_item(home: &str, away: &str, kickoff: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let event_id: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug = ?1")
        .bind(crate::events::WORLD_CUP_2026_SLUG)
        .fetch_one(crate::db::pool())
        .await
        .expect("evento Copa para fixture");
    sqlx::query(
        "INSERT INTO prediction_items
            (id, event_id, kind, title, lock_at, reveal_at, sort_order, status)
         VALUES (?1, ?2, 'football_match', ?3, ?4, ?4, 0, 'open')",
    )
    .bind(&id)
    .bind(&event_id.0)
    .bind(format!("{home} x {away}"))
    .bind(kickoff)
    .execute(crate::db::pool())
    .await
    .expect("prediction item para fixture");
    id
}
