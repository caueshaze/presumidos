use crate::error::ServerFnError;
use uuid::Uuid;

use super::core::{owner, text, MAX_ITEMS};

pub async fn add_numeric_item(
    token: String,
    event_id: String,
    title: String,
    lock_at: String,
    reveal_at: String,
    decimal_places: i64,
    unit_label: Option<String>,
    min_value: Option<String>,
    max_value: Option<String>,
    csrf: String,
) -> Result<String, ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    let title = text("Pergunta", title, 240)?;
    crate::custom_event_manifest::validate_single_choice_timing(
        &title, "builder", &lock_at, &reveal_at,
    )
    .map_err(crate::security::public_error)?;
    let places = crate::numeric::validate_question(decimal_places, None, None)
        .map_err(crate::security::public_error)?;
    let unit_label = unit_label
        .map(|v| crate::security::normalize_required_text("Unidade", v, 1, 60))
        .transpose()?;
    let min = min_value
        .filter(|v| !v.trim().is_empty())
        .map(|v| crate::numeric::parse_scaled(&v, places))
        .transpose()
        .map_err(crate::security::public_error)?;
    let max = max_value
        .filter(|v| !v.trim().is_empty())
        .map(|v| crate::numeric::parse_scaled(&v, places))
        .transpose()
        .map_err(crate::security::public_error)?;
    crate::numeric::validate_question(decimal_places, min, max)
        .map_err(crate::security::public_error)?;
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1")
            .bind(&version_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("numeric_builder_count", e))?;
    if count.0 as usize >= MAX_ITEMS {
        return Err(crate::security::public_error(
            "Limite de perguntas atingido.",
        ));
    }
    let order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM prediction_items WHERE event_version_id=?1",
    )
    .bind(&version_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("numeric_builder_order", e))?;
    let id = Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("numeric_builder_begin", e))?;
    sqlx::query("INSERT INTO prediction_items(id,event_id,event_version_id,external_key,kind,title,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,?4,'numeric',?5,?6,?7,?8,'open')").bind(&id).bind(&event_id).bind(&version_id).bind(format!("builder-{}",Uuid::new_v4())).bind(title).bind(lock_at).bind(reveal_at).bind(order.0).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("numeric_builder_item",e))?;
    sqlx::query("INSERT INTO numeric_questions(item_id,decimal_places,unit_label,min_value_scaled,max_value_scaled) VALUES(?1,?2,?3,?4,?5)").bind(&id).bind(decimal_places).bind(unit_label).bind(min).bind(max).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("numeric_builder_question",e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("numeric_builder_commit", e))?;
    Ok(id)
}
pub async fn add_multiple_choice_item(
    token: String,
    event_id: String,
    title: String,
    lock_at: String,
    reveal_at: String,
    min_selections: i64,
    max_selections: Option<i64>,
    csrf: String,
) -> Result<String, ServerFnError> {
    let (_, db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    let title = text("Pergunta", title, 240)?;
    crate::custom_event_manifest::validate_single_choice_timing(
        &title, "builder", &lock_at, &reveal_at,
    )
    .map_err(crate::security::public_error)?;
    if min_selections < 1 || max_selections.is_some_and(|max| max < min_selections) {
        return Err(crate::security::public_error(
            "Mínimo/máximo de escolhas inválido.",
        ));
    }
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1")
            .bind(&version_id)
            .fetch_one(db)
            .await
            .map_err(|e| crate::security::internal_error("multiple_choice_builder_count", e))?;
    if count.0 as usize >= MAX_ITEMS {
        return Err(crate::security::public_error(
            "Limite de perguntas atingido.",
        ));
    }
    let order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM prediction_items WHERE event_version_id=?1",
    )
    .bind(&version_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("multiple_choice_builder_order", e))?;
    let id = Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_builder_begin", e))?;
    sqlx::query("INSERT INTO prediction_items(id,event_id,event_version_id,external_key,kind,title,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,?4,'multiple_choice',?5,?6,?7,?8,'open')").bind(&id).bind(&event_id).bind(&version_id).bind(format!("builder-{}",Uuid::new_v4())).bind(title).bind(lock_at).bind(reveal_at).bind(order.0).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("multiple_choice_builder_item",e))?;
    sqlx::query("INSERT INTO multiple_choice_questions(item_id,min_selections,max_selections) VALUES(?1,?2,?3)").bind(&id).bind(min_selections).bind(max_selections).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("multiple_choice_builder_question",e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_builder_commit", e))?;
    Ok(id)
}
