use crate::error::ServerFnError;
use crate::models::{Event, EventKind, EventStatus};

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct AdminEventRow {
    id: String,
    name: String,
    slug: String,
    kind: String,
    status: String,
    created_by: Option<String>,
    created_by_username: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    created_at: String,
    updated_at: String,
    description: Option<String>,
    cover_url: Option<String>,
    cover_asset_id: Option<String>,
    external_url: Option<String>,
    pool_creation_enabled: i64,
    current_published_version_id: Option<String>,
    working_version_id: Option<String>,
    current_version_number: Option<i64>,
    item_count: i64,
    option_count: i64,
    pool_count: i64,
    archived_at: Option<String>,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct AdminEventStateRow {
    id: String,
    name: String,
    slug: String,
    kind: String,
    status: String,
    created_by: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    created_at: String,
    updated_at: String,
    description: Option<String>,
    cover_url: Option<String>,
    cover_asset_id: Option<String>,
    external_url: Option<String>,
    pool_creation_enabled: i64,
    current_published_version_id: Option<String>,
    archived_at: Option<String>,
}

#[cfg(feature = "server")]
fn admin_event_from_row(row: AdminEventStateRow) -> Event {
    let AdminEventStateRow {
        id,
        name,
        slug,
        kind,
        status,
        created_by,
        starts_at,
        ends_at,
        created_at,
        updated_at,
        description,
        cover_url,
        cover_asset_id,
        external_url,
        pool_creation_enabled,
        current_published_version_id,
        archived_at,
    } = row;
    Event {
        id,
        name,
        slug,
        kind: if kind == "custom" {
            EventKind::Custom
        } else {
            EventKind::Football
        },
        origin: if created_by.is_some() {
            crate::models::EventOrigin::User
        } else {
            crate::models::EventOrigin::System
        },
        status: match status.as_str() {
            "draft" => EventStatus::Draft,
            "finished" => EventStatus::Finished,
            _ => EventStatus::Active,
        },
        created_by,
        starts_at,
        ends_at,
        created_at,
        updated_at,
        description,
        cover_url,
        cover_asset_url: cover_asset_id
            .as_ref()
            .map(|asset_id| format!("/media/assets/{asset_id}/cover")),
        cover_asset_id,
        external_url,
        pool_creation_enabled: pool_creation_enabled != 0,
        current_published_version_id,
        archived_at,
    }
}

#[cfg(feature = "server")]
pub async fn list_events_admin(
    token: String,
) -> Result<Vec<crate::models::AdminEventRecord>, ServerFnError> {
    use crate::auth::require_admin;

    crate::security::apply_security_headers();
    require_admin(&token).await?;
    let rows: Vec<AdminEventRow> = sqlx::query_as(
        "SELECT e.id, COALESCE(v.name,e.name) AS name, e.slug, e.kind, e.status, e.created_by, u.username AS created_by_username,
                e.starts_at, e.ends_at, e.created_at, e.updated_at, COALESCE(v.description,e.description) AS description,
                COALESCE(v.cover_url,e.cover_url) AS cover_url, COALESCE(v.cover_asset_id,e.cover_asset_id) AS cover_asset_id, COALESCE(v.external_url,e.external_url) AS external_url,
                e.pool_creation_enabled, e.current_published_version_id, e.archived_at,
                (SELECT w.id FROM event_versions w WHERE w.event_id=e.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1) AS working_version_id,
                (SELECT v2.version_number FROM event_versions v2 WHERE v2.id=e.current_published_version_id) AS current_version_number,
                (SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id=COALESCE(e.current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=e.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1))) AS item_count,
                (SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_version_id=COALESCE(e.current_published_version_id,(SELECT w.id FROM event_versions w WHERE w.event_id=e.id AND w.state='working' ORDER BY w.version_number DESC LIMIT 1))) AS option_count,
                (SELECT COUNT(*) FROM pools p WHERE p.event_id=e.id) AS pool_count
         FROM events e LEFT JOIN users u ON u.id=e.created_by
         LEFT JOIN event_versions v ON v.id=e.current_published_version_id
         ORDER BY CASE WHEN e.ends_at IS NULL THEN 0 ELSE 1 END, datetime(e.ends_at) DESC, e.created_at DESC",
    )
    .fetch_all(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("admin_list_events", e))?;
    Ok(rows
        .into_iter()
        .map(|row| crate::models::AdminEventRecord {
            id: row.id,
            name: row.name,
            slug: row.slug,
            kind: if row.kind == "custom" {
                EventKind::Custom
            } else {
                EventKind::Football
            },
            origin: if row.created_by.is_some() {
                crate::models::EventOrigin::User
            } else {
                crate::models::EventOrigin::System
            },
            status: match row.status.as_str() {
                "draft" => EventStatus::Draft,
                "finished" => EventStatus::Finished,
                _ => EventStatus::Active,
            },
            created_by: row.created_by,
            created_by_username: row.created_by_username,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            description: row.description,
            cover_url: row.cover_url,
            cover_asset_url: row
                .cover_asset_id
                .map(|asset_id| format!("/media/assets/{asset_id}/cover")),
            external_url: row.external_url,
            pool_creation_enabled: row.pool_creation_enabled != 0,
            current_published_version_id: row.current_published_version_id,
            working_version_id: row.working_version_id,
            current_version_number: row.current_version_number,
            item_count: row.item_count,
            option_count: row.option_count,
            pool_count: row.pool_count,
            archived_at: row.archived_at,
        })
        .collect())
}

#[cfg(feature = "server")]
pub async fn set_pool_creation_enabled(
    token: String,
    event_id: String,
    enabled: bool,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Evento", &event_id)?;
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let changed = sqlx::query(
        "UPDATE events SET pool_creation_enabled=?2, updated_at=datetime('now') WHERE id=?1 AND archived_at IS NULL",
    )
    .bind(&event_id)
    .bind(if enabled { 1_i64 } else { 0_i64 })
    .execute(db)
    .await
    .map_err(|e| crate::security::internal_error("admin_event_pool_creation", e))?;
    if changed.rows_affected() != 1 {
        return Err(crate::security::public_error("Evento não encontrado."));
    }
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_pool_creation_changed",
        "event",
        Some(&event_id),
        None,
        serde_json::json!({"enabled": enabled}),
    )
    .await
}

#[cfg(feature = "server")]
pub async fn finish_event(
    token: String,
    event_id: String,
    csrf_token: String,
) -> Result<Event, ServerFnError> {
    use crate::auth::require_recent_admin;

    crate::security::apply_security_headers();
    crate::security::validate_uuid("Evento", &event_id)?;
    let headers = crate::security::current_headers();
    let session = require_recent_admin(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let row: Option<AdminEventStateRow> =
        sqlx::query_as::<_, AdminEventStateRow>(
            "SELECT e.id, COALESCE(v.name,e.name) AS name, e.slug, e.kind, e.status, e.created_by, e.starts_at, e.ends_at, e.created_at, e.updated_at, COALESCE(v.description,e.description) AS description, COALESCE(v.cover_url,e.cover_url) AS cover_url, COALESCE(v.cover_asset_id,e.cover_asset_id) AS cover_asset_id, COALESCE(v.external_url,e.external_url) AS external_url, e.pool_creation_enabled, e.current_published_version_id, e.archived_at
             FROM events e LEFT JOIN event_versions v ON v.id=e.current_published_version_id WHERE e.id = ?1 AND e.archived_at IS NULL",
        )
        .bind(&event_id)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("admin_finish_event_load", e))?;
    let Some(row) = row else {
        return Err(crate::security::public_error("Evento não encontrado."));
    };
    if row.status == "finished" {
        return Ok(admin_event_from_row(row));
    }
    let ended = row
        .ends_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|value| value <= chrono::Utc::now());
    if !ended {
        return Err(crate::security::public_error(
            "A edição só pode ser encerrada depois da data de término.",
        ));
    }
    sqlx::query("UPDATE events SET status='finished', updated_at=datetime('now') WHERE id=?1")
        .bind(&event_id)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("admin_finish_event_update", e))?;
    let forced_matches = crate::matches::force_finish_matches_for_event(&event_id).await?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_finished",
        "event",
        Some(&event_id),
        Some(&crate::security::client_ip(&headers)),
        serde_json::json!({"previous_status": "active", "ends_at": row.ends_at, "matches_force_finished": forced_matches}),
    )
    .await?;
    let mut finished = admin_event_from_row(row);
    finished.status = EventStatus::Finished;
    Ok(finished)
}
