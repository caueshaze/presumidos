use crate::error::ServerFnError;

use super::core::get_owned;
use super::{BuilderDraft, BuilderItem, BuilderOption, BuilderOptionLink, BuilderVersion};

pub async fn draft(token: String, id: String) -> Result<BuilderDraft, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let mut event = get_owned(&session.user_id, &id).await?;
    let working: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("event_draft_working", e))?;
    let is_admin: (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id=?1")
        .bind(&session.user_id)
        .fetch_one(crate::db::pool())
        .await
        .map_err(|e| crate::security::internal_error("event_draft_admin", e))?;
    let version_id = if let Some((working_id,)) = working {
        if is_admin.0 || event.status == crate::models::EventStatus::Draft {
            crate::custom_event_manifest::ensure_working_revision(&id, &session.user_id).await?
        } else {
            working_id
        }
    } else if is_admin.0 && event.current_published_version_id.is_some() {
        crate::custom_event_manifest::ensure_working_revision(&id, &session.user_id).await?
    } else if let Some(id) = event.current_published_version_id.clone() {
        id
    } else {
        return Err(crate::security::public_error(
            "Evento sem versão para editar.",
        ));
    };
    if let Some((name, description, cover_url, cover_asset_id, external_url)) = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT name,description,cover_url,cover_asset_id,external_url FROM event_versions WHERE id=?1",
    )
    .bind(&version_id)
    .fetch_optional(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("event_draft_version_metadata", e))?
    {
        event.name = name;
        event.description = description;
        event.cover_url = cover_url;
        event.cover_asset_id = cover_asset_id;
        event.cover_asset_url = event.cover_asset_id.clone().map(|asset_id| format!("/media/assets/{asset_id}/cover"));
        event.external_url = external_url;
    }
    let cover_asset_id: Option<(String,)> =
        sqlx::query_as("SELECT cover_asset_id FROM event_versions WHERE id=?1")
            .bind(&version_id)
            .fetch_optional(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("event_draft_cover_asset", e))?;
    event.cover_asset_id = cover_asset_id.map(|row| row.0);
    event.cover_asset_url = event
        .cover_asset_id
        .clone()
        .map(|asset_id| format!("/media/assets/{asset_id}/cover"));
    let rows:Vec<(String,String,String,String,String,i64,Option<String>,Option<i64>,Option<String>,Option<i64>,Option<i64>,Option<i64>,Option<i64>,Option<i64>)>=sqlx::query_as("SELECT pi.id,pi.kind,pi.title,pi.lock_at,pi.reveal_at,pi.sort_order,q.correct_option_id,n.decimal_places,n.unit_label,n.min_value_scaled,n.max_value_scaled,n.result_value_scaled,mq.min_selections,mq.max_selections FROM prediction_items pi LEFT JOIN custom_questions q ON q.item_id=pi.id LEFT JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_version_id=?1 ORDER BY pi.sort_order,pi.id").bind(&version_id).fetch_all(crate::db::pool()).await.map_err(|e|crate::security::internal_error("event_draft_items",e))?;
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
        let option_rows = sqlx::query_as::<_, (String, String, i64, Option<String>, Option<String>)>("SELECT o.id,o.label,o.sort_order,o.image_url,a.id FROM custom_question_options o LEFT JOIN assets a ON a.id=o.image_asset_id WHERE o.item_id=?1 ORDER BY o.sort_order,o.id").bind(&id).fetch_all(crate::db::pool()).await.map_err(|e|crate::security::internal_error("event_draft_options",e))?;
        let mut options = Vec::with_capacity(option_rows.len());
        for (option_id, label, sort_order, image_url, image_asset_id) in option_rows {
            let links = sqlx::query_as::<_, (String, String, String, i64)>(
                "SELECT kind,label,url,sort_order FROM option_links WHERE option_id=?1 ORDER BY sort_order,id",
            )
            .bind(&option_id)
            .fetch_all(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("event_draft_option_links", e))?
            .into_iter()
            .map(|(kind, label, url, sort_order)| BuilderOptionLink { kind, label, url, sort_order })
            .collect();
            options.push(BuilderOption {
                id: option_id,
                label,
                sort_order,
                image_url,
                image_asset_url: image_asset_id.map(|id| format!("/media/assets/{id}/card")),
                links,
            });
        }
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
    let version_rows: Vec<(String, i64, String, i64, String, String, Option<String>, String, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT v.id,v.version_number,v.state,v.is_current_published,v.name,v.fingerprint,v.base_fingerprint,v.created_at,v.updated_at,
                (SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id=v.id),
                (SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_version_id=v.id),
                (SELECT COUNT(*) FROM pools p WHERE p.event_version_id=v.id)
         FROM event_versions v WHERE v.event_id=?1 ORDER BY v.version_number DESC",
    )
    .bind(&id)
    .fetch_all(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("event_draft_versions", e))?;
    let versions = version_rows
        .into_iter()
        .map(
            |(
                id,
                version_number,
                state,
                is_current_published,
                name,
                fingerprint,
                base_fingerprint,
                created_at,
                updated_at,
                item_count,
                option_count,
                pool_count,
            )| BuilderVersion {
                id,
                version_number,
                state,
                is_current_published: is_current_published != 0,
                name,
                fingerprint,
                base_fingerprint,
                created_at,
                updated_at,
                item_count,
                option_count,
                pool_count,
            },
        )
        .collect();
    Ok(BuilderDraft {
        event,
        items,
        versions,
    })
}
