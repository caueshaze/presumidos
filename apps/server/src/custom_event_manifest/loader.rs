use super::*;
use crate::error::ServerFnError;
#[cfg(feature = "server")]
use sqlx::{sqlite::SqliteConnection, Sqlite, SqlitePool};

#[cfg(feature = "server")]
pub(crate) async fn load_manifest_conn(
    conn: &mut SqliteConnection,
    event_id: &str,
    version_id: Option<&str>,
) -> Result<CustomEventManifest, ServerFnError> {
    let event: Option<(String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>)> = sqlx::query_as("SELECT v.name,e.slug,e.kind,v.description,e.starts_at,e.ends_at,v.cover_url,v.external_url,v.cover_asset_id,a.sha256,a.media_type FROM events e JOIN event_versions v ON v.id=COALESCE(?2,e.current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=e.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) LEFT JOIN assets a ON a.id=v.cover_asset_id WHERE e.id=?1").bind(event_id).bind(version_id).fetch_optional(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_event", e))?;
    let Some((
        name,
        slug,
        kind,
        description,
        starts_at,
        ends_at,
        cover_url,
        external_url,
        _cover_asset_id,
        cover_sha256,
        cover_media_type,
    )) = event
    else {
        return Err(crate::security::public_error("Evento não encontrado."));
    };
    let rows: Vec<(String,String,String,Option<String>,String,String,i64,Option<i64>,Option<String>,Option<i64>,Option<i64>,Option<i64>,Option<i64>)> = sqlx::query_as("SELECT pi.external_key,pi.kind,pi.title,pi.description,pi.lock_at,pi.reveal_at,pi.sort_order,n.decimal_places,n.unit_label,n.min_value_scaled,n.max_value_scaled,mq.min_selections,mq.max_selections FROM prediction_items pi LEFT JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_version_id=COALESCE(?2,(SELECT COALESCE(current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=events.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) FROM events WHERE id=?1)) ORDER BY pi.sort_order,pi.id").bind(event_id).bind(version_id).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_items", e))?;
    let mut items = Vec::new();
    for (
        external_key,
        kind,
        title,
        item_description,
        lock_at,
        reveal_at,
        _sort,
        decimal_places,
        unit_label,
        min_scaled,
        max_scaled,
        min_selections,
        max_selections,
    ) in rows
    {
        let Some(external_key) = Some(external_key) else {
            return Err(crate::security::public_error("item custom sem externalKey"));
        };
        let options_rows: Vec<(String,String,i64,Option<String>,Option<String>,Option<String>)> = sqlx::query_as("SELECT o.external_key,o.label,o.sort_order,o.image_url,a.sha256,a.media_type FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id LEFT JOIN assets a ON a.id=o.image_asset_id WHERE pi.event_version_id=COALESCE(?3,(SELECT COALESCE(current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=events.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) FROM events WHERE id=?1)) AND pi.external_key=?2 ORDER BY o.sort_order,o.id").bind(event_id).bind(&external_key).bind(version_id).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_options", e))?;
        let mut options = Vec::new();
        for (option_key, label, _sort, image_url, image_sha256, image_media_type) in options_rows {
            let links: Vec<(String,String,String,i64)> = sqlx::query_as("SELECT l.kind,l.label,l.url,l.sort_order FROM option_links l JOIN custom_question_options o ON o.id=l.option_id JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_version_id=COALESCE(?4,(SELECT COALESCE(current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=events.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1)) FROM events WHERE id=?1)) AND pi.external_key=?2 AND o.external_key=?3 ORDER BY l.sort_order,l.id").bind(event_id).bind(&external_key).bind(&option_key).bind(version_id).fetch_all(&mut *conn).await.map_err(|e| crate::security::internal_error("manifest_export_links", e))?;
            options.push(CustomEventManifestOption {
                external_key: option_key,
                label,
                image_url,
                image_asset: image_sha256
                    .zip(image_media_type)
                    .map(|(sha256, media_type)| AssetReference {
                        kind: "asset".into(),
                        sha256,
                        media_type,
                    }),
                links: links
                    .into_iter()
                    .map(|(kind, label, url, _)| CustomEventManifestOptionLink { kind, label, url })
                    .collect(),
            });
        }
        let values = decimal_places
            .map(|p| {
                let p = p as u8;
                (
                    min_scaled.map(|v| crate::numeric::display_scaled(v, p)),
                    max_scaled.map(|v| crate::numeric::display_scaled(v, p)),
                )
            })
            .unwrap_or((None, None));
        items.push(CustomEventManifestItem {
            external_key,
            kind,
            title,
            description: item_description,
            lock_at,
            reveal_at,
            options,
            decimal_places,
            unit_label,
            min_value: values.0,
            max_value: values.1,
            min_selections,
            max_selections,
        });
    }
    let m = CustomEventManifest {
        schema_version: CURRENT_SCHEMA_VERSION,
        name,
        slug,
        kind,
        description,
        starts_at,
        ends_at,
        cover_url,
        cover_asset: cover_sha256
            .zip(cover_media_type)
            .map(|(sha256, media_type)| AssetReference {
                kind: "asset".into(),
                sha256,
                media_type,
            }),
        external_url,
        items,
    };
    parse_and_validate(
        &serde_json::to_string(&m)
            .map_err(|e| crate::security::internal_error("manifest_export_serialize", e))?,
    )
    .map_err(crate::security::public_error)
}

#[cfg(feature = "server")]
pub(crate) async fn load_manifest(
    db: &SqlitePool,
    event_id: &str,
) -> Result<CustomEventManifest, ServerFnError> {
    let mut conn = db
        .acquire()
        .await
        .map_err(|e| crate::security::internal_error("manifest_export_connection", e))?;
    load_manifest_conn(&mut conn, event_id, None).await
}
