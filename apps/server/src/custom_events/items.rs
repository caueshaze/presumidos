use crate::error::ServerFnError;
use uuid::Uuid;

use super::core::{owner, text, MAX_ITEMS};

pub async fn update_item(
    token: String,
    event_id: String,
    item_id: String,
    title: String,
    lock_at: String,
    reveal_at: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    let title = text("Pergunta", title, 240)?;
    crate::custom_event_manifest::validate_single_choice_timing(
        &title, "builder", &lock_at, &reveal_at,
    )
    .map_err(crate::security::public_error)?;
    let changed=sqlx::query("UPDATE prediction_items SET title=?3,lock_at=?4,reveal_at=?5,updated_at=datetime('now') WHERE id=?1 AND event_version_id=?2").bind(item_id).bind(version_id).bind(title).bind(lock_at).bind(reveal_at).execute(db).await.map_err(|e|crate::security::internal_error("event_update_item",e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error("Pergunta inválida."));
    }
    Ok(())
}
pub async fn delete_item(
    token: String,
    event_id: String,
    item_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    sqlx::query("DELETE FROM prediction_items WHERE id=?1 AND event_version_id=?2")
        .bind(item_id)
        .bind(version_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("event_delete_item", e))?;
    Ok(())
}
pub async fn move_item(
    token: String,
    event_id: String,
    item_id: String,
    direction: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    let current: Option<(i64,)> = sqlx::query_as(
        "SELECT sort_order FROM prediction_items WHERE id=?1 AND event_version_id=?2",
    )
    .bind(&item_id)
    .bind(&version_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("event_move_item", e))?;
    let Some((order,)) = current else {
        return Err(crate::security::public_error("Pergunta inválida."));
    };
    let adjacent:Option<(String,i64)>=sqlx::query_as(if direction<0{"SELECT id,sort_order FROM prediction_items WHERE event_version_id=?1 AND sort_order<?2 ORDER BY sort_order DESC LIMIT 1"}else{"SELECT id,sort_order FROM prediction_items WHERE event_version_id=?1 AND sort_order>?2 ORDER BY sort_order ASC LIMIT 1"}).bind(&version_id).bind(order).fetch_optional(db).await.map_err(|e|crate::security::internal_error("event_move_item_adjacent",e))?;
    if let Some((other, other_order)) = adjacent {
        let mut tx = db
            .begin()
            .await
            .map_err(|e| crate::security::internal_error("event_move_item_begin", e))?;
        sqlx::query("UPDATE prediction_items SET sort_order=?2 WHERE id=?1")
            .bind(&item_id)
            .bind(other_order)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_move_item_first", e))?;
        sqlx::query("UPDATE prediction_items SET sort_order=?2 WHERE id=?1")
            .bind(other)
            .bind(order)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_move_item_second", e))?;
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("event_move_item_commit", e))?;
    }
    Ok(())
}
pub async fn add_item(
    token: String,
    event_id: String,
    title: String,
    lock_at: String,
    reveal_at: String,
    csrf: String,
) -> Result<String, ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    let title = text("Pergunta", title, 240)?;
    crate::custom_event_manifest::validate_single_choice_timing(
        &title, "builder", &lock_at, &reveal_at,
    )
    .map_err(crate::security::public_error)?;
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1")
            .bind(&version_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("event_items_count", e))?;
    if count.0 as usize >= MAX_ITEMS {
        return Err(crate::security::public_error(
            "Limite de perguntas atingido.",
        ));
    }
    let next_order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM prediction_items WHERE event_version_id=?1",
    )
    .bind(&version_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("event_next_item_order", e))?;
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO prediction_items(id,event_id,event_version_id,external_key,kind,title,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,?4,'single_choice',?5,?6,?7,?8,'open')").bind(&id).bind(&event_id).bind(&version_id).bind(format!("builder-{}",Uuid::new_v4())).bind(title).bind(lock_at).bind(reveal_at).bind(next_order.0).execute(db).await.map_err(|e|crate::security::internal_error("event_add_item",e))?;
    sqlx::query("INSERT INTO custom_questions(item_id,points) VALUES(?1,1)")
        .bind(&id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("event_add_question", e))?;
    Ok(id)
}
