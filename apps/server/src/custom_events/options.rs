use crate::error::ServerFnError;
use uuid::Uuid;

use super::core::{owner, text, MAX_OPTIONS};
use super::BuilderOptionLink;

pub async fn update_option(
    token: String,
    event_id: String,
    item_id: String,
    option_id: String,
    label: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    // O rótulo é conteúdo editorial: alterar o texto não muda o ID da opção,
    // sua ordem, o gabarito nem a pontuação de palpites já registrados.
    let (session, db, version_id) = editorial_owner(token, &event_id, csrf).await?;
    let label = text("Opção", label, 240)?;
    let changed=sqlx::query("UPDATE custom_question_options SET label=?4 WHERE id=?1 AND item_id=?2 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?2 AND event_version_id=?3)").bind(&option_id).bind(&item_id).bind(&version_id).bind(label).execute(db).await.map_err(|e|crate::security::internal_error("event_update_option",e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error("Opção inválida."));
    }
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_option_label_changed",
        "event",
        Some(&event_id),
        None,
        serde_json::json!({"optionId": option_id}),
    )
    .await?;
    Ok(())
}

async fn editorial_owner(
    token: String,
    event_id: &str,
    csrf: String,
) -> Result<(crate::auth::AuthSession, &'static sqlx::SqlitePool, String), ServerFnError> {
    owner(token, event_id, Some(csrf)).await
}

pub async fn update_option_media(
    token: String,
    event_id: String,
    item_id: String,
    option_id: String,
    image_url: Option<String>,
    links: Vec<BuilderOptionLink>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (session, db, version_id) = editorial_owner(token, &event_id, csrf).await?;
    if links.len() > crate::custom_event_manifest::MAX_LINKS_PER_OPTION {
        return Err(crate::security::public_error("Limite de links atingido."));
    }
    let link_count = links.len();
    let image_url = crate::custom_event_manifest::validate_optional_http_url(image_url, "imageUrl")
        .map_err(crate::security::public_error)?;
    let mut normalized_links = Vec::with_capacity(links.len());
    for link in links {
        normalized_links.push((
            text("Tipo do link", link.kind, 64)?,
            text("Rótulo do link", link.label, 240)?,
            crate::custom_event_manifest::validate_optional_http_url(Some(link.url), "url")
                .map_err(crate::security::public_error)?
                .ok_or_else(|| crate::security::public_error("URL do link inválida."))?,
        ));
    }
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("event_update_option_media_begin", e))?;
    let changed = sqlx::query("UPDATE custom_question_options SET image_url=?4 WHERE id=?1 AND item_id=?2 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?2 AND event_version_id=?3)")
        .bind(&option_id).bind(&item_id).bind(&version_id).bind(&image_url).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("event_update_option_media", e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error("Opção inválida."));
    }
    sqlx::query("DELETE FROM option_links WHERE option_id=?1")
        .bind(&option_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("event_replace_option_links", e))?;
    for (sort, (kind, label, url)) in normalized_links.into_iter().enumerate() {
        sqlx::query("INSERT INTO option_links(id,option_id,kind,label,url,sort_order) VALUES(?1,?2,?3,?4,?5,?6)")
            .bind(Uuid::new_v4().to_string()).bind(&option_id).bind(kind).bind(label).bind(url).bind(sort as i64).execute(&mut *tx).await
            .map_err(|e| crate::security::internal_error("event_insert_option_link", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("event_update_option_media_commit", e))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_option_editorial_media_changed",
        "event",
        Some(&event_id),
        None,
        serde_json::json!({"optionId": option_id, "linkCount": link_count}),
    )
    .await?;
    Ok(())
}
pub async fn delete_option(
    token: String,
    event_id: String,
    item_id: String,
    option_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    sqlx::query("DELETE FROM custom_question_options WHERE id=?1 AND item_id=?2 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?2 AND event_version_id=?3)").bind(option_id).bind(item_id).bind(version_id).execute(db).await.map_err(|e|crate::security::internal_error("event_delete_option",e))?;
    Ok(())
}
pub async fn move_option(
    token: String,
    event_id: String,
    item_id: String,
    option_id: String,
    direction: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    let current: Option<(i64,)> = sqlx::query_as("SELECT sort_order FROM custom_question_options WHERE id=?1 AND item_id=?2 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?2 AND event_version_id=?3)").bind(&option_id).bind(&item_id).bind(&version_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("event_move_option",e))?;
    let Some((order,)) = current else {
        return Err(crate::security::public_error("Opção inválida."));
    };
    let adjacent: Option<(String,i64)> = sqlx::query_as(if direction < 0 { "SELECT id,sort_order FROM custom_question_options WHERE item_id=?1 AND sort_order<?2 ORDER BY sort_order DESC LIMIT 1" } else { "SELECT id,sort_order FROM custom_question_options WHERE item_id=?1 AND sort_order>?2 ORDER BY sort_order ASC LIMIT 1" }).bind(&item_id).bind(order).fetch_optional(db).await.map_err(|e|crate::security::internal_error("event_move_option_adjacent",e))?;
    if let Some((other, other_order)) = adjacent {
        let mut tx = db
            .begin()
            .await
            .map_err(|e| crate::security::internal_error("event_move_option_begin", e))?;
        sqlx::query("UPDATE custom_question_options SET sort_order=-1 WHERE id=?1")
            .bind(&option_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_move_option_first", e))?;
        sqlx::query("UPDATE custom_question_options SET sort_order=?2 WHERE id=?1")
            .bind(other)
            .bind(order)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_move_option_second", e))?;
        sqlx::query("UPDATE custom_question_options SET sort_order=?2 WHERE id=?1")
            .bind(&option_id)
            .bind(other_order)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("event_move_option_third", e))?;
        tx.commit()
            .await
            .map_err(|e| crate::security::internal_error("event_move_option_commit", e))?;
    }
    Ok(())
}
pub async fn add_option(
    token: String,
    event_id: String,
    item_id: String,
    label: String,
    csrf: String,
) -> Result<String, ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    let label = text("Opção", label, 240)?;
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM custom_question_options WHERE item_id=?1")
            .bind(&item_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("event_options_count", e))?;
    if count.0 as usize >= MAX_OPTIONS {
        return Err(crate::security::public_error("Limite de opções atingido."));
    }
    let next_order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM custom_question_options WHERE item_id=?1 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?1 AND event_version_id=?2)",
    )
    .bind(&item_id)
    .bind(&version_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("event_next_option_order", e))?;
    let id = Uuid::new_v4().to_string();
    let changed=sqlx::query("INSERT INTO custom_question_options(id,item_id,external_key,label,sort_order) SELECT ?1,pi.id,?2,?3,?4 FROM prediction_items pi WHERE pi.id=?5 AND pi.event_version_id=?6 AND pi.kind IN ('single_choice','multiple_choice')").bind(&id).bind(format!("builder-{}",Uuid::new_v4())).bind(label).bind(next_order.0).bind(&item_id).bind(&version_id).execute(db).await.map_err(|e|crate::security::internal_error("event_add_option",e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error("Pergunta inválida."));
    }
    Ok(id)
}
