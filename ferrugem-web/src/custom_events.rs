use crate::error::ServerFnError;
use crate::models::Event;
use serde::Serialize;
use uuid::Uuid;

pub const MAX_ITEMS: usize = 100;
pub const MAX_OPTIONS: usize = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderOption {
    pub id: String,
    pub label: String,
    pub sort_order: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub lock_at: String,
    pub reveal_at: String,
    pub sort_order: i64,
    pub correct_option_id: Option<String>,
    pub decimal_places: Option<i64>,
    pub unit_label: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub result_value: Option<String>,
    pub min_selections: Option<i64>,
    pub max_selections: Option<i64>,
    pub options: Vec<BuilderOption>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderDraft {
    pub event: Event,
    pub items: Vec<BuilderItem>,
}

fn text(label: &str, value: String, max: usize) -> Result<String, ServerFnError> {
    crate::security::normalize_required_text(label, value, 1, max)
}

fn slugify(name: &str) -> String {
    let folded = name
        .replace(['á', 'à', 'â', 'ã', 'ä'], "a")
        .replace(['é', 'ê', 'ë'], "e")
        .replace(['í', 'ï'], "i")
        .replace(['ó', 'ô', 'õ', 'ö'], "o")
        .replace(['ú', 'ü'], "u")
        .replace('ç', "c");
    let mut slug = String::new();
    for c in folded.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "evento".into()
    } else {
        slug.to_string()
    }
}

async fn owner(
    token: String,
    event_id: &str,
    csrf: Option<String>,
) -> Result<(crate::auth::AuthSession, &'static sqlx::SqlitePool), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    if let Some(csrf) = csrf {
        crate::security::require_csrf(&session.csrf_token, &csrf)?;
    }
    let db = crate::db::pool();
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM events WHERE id=?1 AND kind='custom' AND status='draft' AND created_by=?2",
    )
    .bind(event_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("event_builder_owner", e))?;
    if allowed.is_none() {
        return Err(crate::security::public_error(
            "Somente o dono pode editar este rascunho.",
        ));
    }
    Ok((session, db))
}

pub async fn create(
    token: String,
    name: String,
    starts_at: Option<String>,
    ends_at: Option<String>,
    csrf: String,
) -> Result<Event, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let name = text("Nome do evento", name, 120)?;
    crate::custom_event_manifest::validate_event_window(&starts_at, &ends_at)
        .map_err(crate::security::public_error)?;
    let db = crate::db::pool();
    let base = slugify(&name);
    let mut slug = base.clone();
    let mut n = 2;
    while sqlx::query_as::<_, (String,)>("SELECT id FROM events WHERE slug=?1")
        .bind(&slug)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("event_slug", e))?
        .is_some()
    {
        slug = format!("{base}-{n}");
        n += 1;
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by,starts_at,ends_at) VALUES(?1,?2,?3,'custom','draft',?4,?5,?6)").bind(&id).bind(&name).bind(&slug).bind(&session.user_id).bind(&starts_at).bind(&ends_at).execute(db).await.map_err(|e|crate::security::internal_error("event_create",e))?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_created",
        "event",
        Some(&id),
        None,
        serde_json::json!({"name":name}),
    )
    .await?;
    get_owned(&session.user_id, &id).await
}

type EventRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
);
fn event(row: EventRow) -> Event {
    Event {
        id: row.0,
        name: row.1,
        slug: row.2,
        kind: if row.3 == "custom" {
            crate::models::EventKind::Custom
        } else {
            crate::models::EventKind::Football
        },
        status: match row.4.as_str() {
            "draft" => crate::models::EventStatus::Draft,
            "finished" => crate::models::EventStatus::Finished,
            _ => crate::models::EventStatus::Active,
        },
        created_by: row.5,
        starts_at: row.6,
        ends_at: row.7,
        created_at: row.8,
        updated_at: row.9,
    }
}
pub async fn mine(token: String) -> Result<Vec<Event>, ServerFnError> {
    let s = crate::auth::require_user(&token).await?;
    let rows:Vec<EventRow>=sqlx::query_as("SELECT id,name,slug,kind,status,created_by,starts_at,ends_at,created_at,updated_at FROM events WHERE created_by=?1 ORDER BY created_at DESC").bind(s.user_id).fetch_all(crate::db::pool()).await.map_err(|e|crate::security::internal_error("events_mine",e))?;
    Ok(rows.into_iter().map(event).collect())
}
pub async fn get(token: String, id: String) -> Result<Event, ServerFnError> {
    let s = crate::auth::require_user(&token).await?;
    get_owned(&s.user_id, &id).await
}
pub async fn draft(token: String, id: String) -> Result<BuilderDraft, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let event = get_owned(&session.user_id, &id).await?;
    let rows:Vec<(String,String,String,String,String,i64,Option<String>,Option<i64>,Option<String>,Option<i64>,Option<i64>,Option<i64>,Option<i64>,Option<i64>)>=sqlx::query_as("SELECT pi.id,pi.kind,pi.title,pi.lock_at,pi.reveal_at,pi.sort_order,q.correct_option_id,n.decimal_places,n.unit_label,n.min_value_scaled,n.max_value_scaled,n.result_value_scaled,mq.min_selections,mq.max_selections FROM prediction_items pi LEFT JOIN custom_questions q ON q.item_id=pi.id LEFT JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_id=?1 ORDER BY pi.sort_order,pi.id").bind(&id).fetch_all(crate::db::pool()).await.map_err(|e|crate::security::internal_error("event_draft_items",e))?;
    let mut items = Vec::with_capacity(rows.len());
    for (
        id,
        kind,
        title,
        lock_at,
        reveal_at,
        sort_order,
        correct_option_id,
        decimal_places,
        unit_label,
        min_scaled,
        max_scaled,
        result_scaled,
        min_selections,
        max_selections,
    ) in rows
    {
        let options=sqlx::query_as::<_,(String,String,i64)>("SELECT id,label,sort_order FROM custom_question_options WHERE item_id=?1 ORDER BY sort_order,id").bind(&id).fetch_all(crate::db::pool()).await.map_err(|e|crate::security::internal_error("event_draft_options",e))?.into_iter().map(|(id,label,sort_order)|BuilderOption{id,label,sort_order}).collect();
        items.push(BuilderItem {
            id,
            kind,
            title,
            lock_at,
            reveal_at,
            sort_order,
            correct_option_id,
            min_value: min_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            max_value: max_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            result_value: result_scaled
                .zip(decimal_places)
                .map(|(v, p)| crate::numeric::display_scaled(v, p as u8)),
            decimal_places,
            unit_label,
            min_selections,
            max_selections,
            options,
        });
    }
    Ok(BuilderDraft { event, items })
}

pub async fn update_metadata(
    token: String,
    event_id: String,
    name: String,
    starts_at: Option<String>,
    ends_at: Option<String>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    let name = text("Nome do evento", name, 120)?;
    crate::custom_event_manifest::validate_event_window(&starts_at, &ends_at)
        .map_err(crate::security::public_error)?;
    sqlx::query(
        "UPDATE events SET name=?2,starts_at=?3,ends_at=?4,updated_at=datetime('now') WHERE id=?1",
    )
    .bind(event_id)
    .bind(name)
    .bind(starts_at)
    .bind(ends_at)
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("event_update_metadata", e))?;
    Ok(())
}
pub async fn update_item(
    token: String,
    event_id: String,
    item_id: String,
    title: String,
    lock_at: String,
    reveal_at: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    let title = text("Pergunta", title, 240)?;
    crate::custom_event_manifest::validate_single_choice_timing(
        &title, "builder", &lock_at, &reveal_at,
    )
    .map_err(crate::security::public_error)?;
    let changed=sqlx::query("UPDATE prediction_items SET title=?3,lock_at=?4,reveal_at=?5,updated_at=datetime('now') WHERE id=?1 AND event_id=?2").bind(item_id).bind(event_id).bind(title).bind(lock_at).bind(reveal_at).execute(db).await.map_err(|e|crate::security::internal_error("event_update_item",e))?;
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
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    sqlx::query("DELETE FROM prediction_items WHERE id=?1 AND event_id=?2")
        .bind(item_id)
        .bind(event_id)
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
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    let current: Option<(i64,)> =
        sqlx::query_as("SELECT sort_order FROM prediction_items WHERE id=?1 AND event_id=?2")
            .bind(&item_id)
            .bind(&event_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("event_move_item", e))?;
    let Some((order,)) = current else {
        return Err(crate::security::public_error("Pergunta inválida."));
    };
    let adjacent:Option<(String,i64)>=sqlx::query_as(if direction<0{"SELECT id,sort_order FROM prediction_items WHERE event_id=?1 AND sort_order<?2 ORDER BY sort_order DESC LIMIT 1"}else{"SELECT id,sort_order FROM prediction_items WHERE event_id=?1 AND sort_order>?2 ORDER BY sort_order ASC LIMIT 1"}).bind(&event_id).bind(order).fetch_optional(db).await.map_err(|e|crate::security::internal_error("event_move_item_adjacent",e))?;
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
pub async fn update_option(
    token: String,
    event_id: String,
    item_id: String,
    option_id: String,
    label: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    let label = text("Opção", label, 240)?;
    let changed=sqlx::query("UPDATE custom_question_options SET label=?4 WHERE id=?1 AND item_id=?2 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?2 AND event_id=?3)").bind(option_id).bind(item_id).bind(event_id).bind(label).execute(db).await.map_err(|e|crate::security::internal_error("event_update_option",e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error("Opção inválida."));
    }
    Ok(())
}
pub async fn delete_option(
    token: String,
    event_id: String,
    item_id: String,
    option_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    sqlx::query("DELETE FROM custom_question_options WHERE id=?1 AND item_id=?2 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?2 AND event_id=?3)").bind(option_id).bind(item_id).bind(event_id).execute(db).await.map_err(|e|crate::security::internal_error("event_delete_option",e))?;
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
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    let current: Option<(i64,)> = sqlx::query_as("SELECT sort_order FROM custom_question_options WHERE id=?1 AND item_id=?2 AND EXISTS(SELECT 1 FROM prediction_items WHERE id=?2 AND event_id=?3)").bind(&option_id).bind(&item_id).bind(&event_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("event_move_option",e))?;
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
pub async fn delete(token: String, event_id: String, csrf: String) -> Result<(), ServerFnError> {
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("event_delete_begin", e))?;
    sqlx::query("DELETE FROM prediction_items WHERE event_id=?1")
        .bind(&event_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("event_delete_items", e))?;
    sqlx::query("DELETE FROM events WHERE id=?1 AND status='draft'")
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("event_delete", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("event_delete_commit", e))?;
    Ok(())
}
async fn get_owned(user: &str, id: &str) -> Result<Event, ServerFnError> {
    let row:Option<EventRow>=sqlx::query_as("SELECT id,name,slug,kind,status,created_by,starts_at,ends_at,created_at,updated_at FROM events WHERE id=?1 AND created_by=?2").bind(id).bind(user).fetch_optional(crate::db::pool()).await.map_err(|e|crate::security::internal_error("event_get",e))?;
    row.map(event)
        .ok_or_else(|| crate::security::public_error("Evento não encontrado."))
}

pub async fn add_item(
    token: String,
    event_id: String,
    title: String,
    lock_at: String,
    reveal_at: String,
    csrf: String,
) -> Result<String, ServerFnError> {
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
    let title = text("Pergunta", title, 240)?;
    crate::custom_event_manifest::validate_single_choice_timing(
        &title, "builder", &lock_at, &reveal_at,
    )
    .map_err(crate::security::public_error)?;
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_id=?1")
        .bind(&event_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("event_items_count", e))?;
    if count.0 as usize >= MAX_ITEMS {
        return Err(crate::security::public_error(
            "Limite de perguntas atingido.",
        ));
    }
    let next_order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM prediction_items WHERE event_id=?1",
    )
    .bind(&event_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("event_next_item_order", e))?;
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO prediction_items(id,event_id,external_key,kind,title,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,'single_choice',?4,?5,?6,?7,'open')").bind(&id).bind(&event_id).bind(format!("builder-{}",Uuid::new_v4())).bind(title).bind(lock_at).bind(reveal_at).bind(next_order.0).execute(db).await.map_err(|e|crate::security::internal_error("event_add_item",e))?;
    sqlx::query("INSERT INTO custom_questions(item_id,points) VALUES(?1,1)")
        .bind(&id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("event_add_question", e))?;
    Ok(id)
}

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
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
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
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_id=?1")
        .bind(&event_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("numeric_builder_count", e))?;
    if count.0 as usize >= MAX_ITEMS {
        return Err(crate::security::public_error(
            "Limite de perguntas atingido.",
        ));
    }
    let order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM prediction_items WHERE event_id=?1",
    )
    .bind(&event_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("numeric_builder_order", e))?;
    let id = Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("numeric_builder_begin", e))?;
    sqlx::query("INSERT INTO prediction_items(id,event_id,external_key,kind,title,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,'numeric',?4,?5,?6,?7,'open')").bind(&id).bind(&event_id).bind(format!("builder-{}",Uuid::new_v4())).bind(title).bind(lock_at).bind(reveal_at).bind(order.0).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("numeric_builder_item",e))?;
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
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
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
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_id=?1")
        .bind(&event_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_builder_count", e))?;
    if count.0 as usize >= MAX_ITEMS {
        return Err(crate::security::public_error(
            "Limite de perguntas atingido.",
        ));
    }
    let order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM prediction_items WHERE event_id=?1",
    )
    .bind(&event_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("multiple_choice_builder_order", e))?;
    let id = Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_builder_begin", e))?;
    sqlx::query("INSERT INTO prediction_items(id,event_id,external_key,kind,title,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,'multiple_choice',?4,?5,?6,?7,'open')").bind(&id).bind(&event_id).bind(format!("builder-{}",Uuid::new_v4())).bind(title).bind(lock_at).bind(reveal_at).bind(order.0).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("multiple_choice_builder_item",e))?;
    sqlx::query("INSERT INTO multiple_choice_questions(item_id,min_selections,max_selections) VALUES(?1,?2,?3)").bind(&id).bind(min_selections).bind(max_selections).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("multiple_choice_builder_question",e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("multiple_choice_builder_commit", e))?;
    Ok(id)
}
pub async fn add_option(
    token: String,
    event_id: String,
    item_id: String,
    label: String,
    csrf: String,
) -> Result<String, ServerFnError> {
    let (_, db) = owner(token, &event_id, Some(csrf)).await?;
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
        "SELECT COALESCE(MAX(sort_order),-1)+1 FROM custom_question_options WHERE item_id=?1",
    )
    .bind(&item_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("event_next_option_order", e))?;
    let id = Uuid::new_v4().to_string();
    let changed=sqlx::query("INSERT INTO custom_question_options(id,item_id,external_key,label,sort_order) SELECT ?1,pi.id,?2,?3,?4 FROM prediction_items pi WHERE pi.id=?5 AND pi.event_id=?6 AND pi.kind IN ('single_choice','multiple_choice')").bind(&id).bind(format!("builder-{}",Uuid::new_v4())).bind(label).bind(next_order.0).bind(&item_id).bind(&event_id).execute(db).await.map_err(|e|crate::security::internal_error("event_add_option",e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error("Pergunta inválida."));
    }
    Ok(id)
}
pub async fn publish(token: String, event_id: String, csrf: String) -> Result<(), ServerFnError> {
    let (s, db) = owner(token, &event_id, Some(csrf)).await?;
    let invalid:Option<(String,)>=sqlx::query_as("SELECT pi.title FROM prediction_items pi LEFT JOIN custom_question_options o ON o.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_id=?1 AND pi.kind IN ('single_choice','multiple_choice') GROUP BY pi.id HAVING COUNT(o.id)<2 OR (pi.kind='multiple_choice' AND (mq.min_selections<1 OR COALESCE(mq.max_selections,COUNT(o.id))<mq.min_selections OR COALESCE(mq.max_selections,COUNT(o.id))>COUNT(o.id))) LIMIT 1").bind(&event_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("event_publish_validate",e))?;
    if let Some((t,)) = invalid {
        return Err(crate::security::public_error(&format!(
            "{t} precisa ter pelo menos 2 opções."
        )));
    }
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_id=?1")
        .bind(&event_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("event_publish_count", e))?;
    if total.0 == 0 {
        return Err(crate::security::public_error(
            "O evento precisa ter pelo menos uma pergunta.",
        ));
    }
    sqlx::query("UPDATE events SET status='active',updated_at=datetime('now') WHERE id=?1 AND status='draft'").bind(&event_id).execute(db).await.map_err(|e|crate::security::internal_error("event_publish",e))?;
    crate::security::append_audit_log(
        db,
        Some(&s.user_id),
        "event_published",
        "event",
        Some(&event_id),
        None,
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}
