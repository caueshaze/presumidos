use crate::{error::ServerFnError, models::*};

#[cfg(feature = "server")]
pub(crate) type PoolMemberUserRow = (String, String, String, bool, Option<String>, Option<String>);

#[cfg(feature = "server")]
pub(crate) type PoolSummaryRow = (
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    Option<String>,
);

#[cfg(feature = "server")]
pub(crate) fn event_summary(
    id: String,
    name: String,
    slug: String,
    kind: String,
    status: String,
    ends_at: Option<String>,
) -> EventSummary {
    let is_historical = status == "finished"
        || ends_at
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|value| value <= chrono::Utc::now());
    EventSummary {
        id,
        name,
        slug,
        kind: if kind == "custom" {
            EventKind::Custom
        } else {
            EventKind::Football
        },
        status: match status.as_str() {
            "draft" => EventStatus::Draft,
            "finished" => EventStatus::Finished,
            _ => EventStatus::Active,
        },
        ends_at,
        description: None,
        cover_url: None,
        external_url: None,
        cover_asset_url: None,
        is_historical,
    }
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct PoolReportRow {
    pub(crate) id: String,
    pub(crate) pool_id: String,
    pub(crate) pool_name: String,
    pub(crate) invite_code: String,
    pub(crate) reporter_user_id: Option<String>,
    pub(crate) reporter_username: Option<String>,
    pub(crate) category: String,
    pub(crate) details: String,
    pub(crate) status: String,
    pub(crate) reviewed_by: Option<String>,
    pub(crate) reviewed_by_username: Option<String>,
    pub(crate) reviewed_at: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[cfg(feature = "server")]
pub(crate) fn sqlite_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(feature = "server")]
pub(crate) async fn ensure_pool_membership(
    db: &sqlx::SqlitePool,
    pool_id: &str,
    user_id: &str,
    error_context: &str,
) -> Result<(), ServerFnError> {
    let membership: Option<(String,)> =
        sqlx::query_as("SELECT pool_id FROM pool_members WHERE pool_id = ?1 AND user_id = ?2")
            .bind(pool_id)
            .bind(user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error(error_context, e))?;

    if membership.is_none() {
        Err(crate::security::public_error(
            "Voce nao e membro deste bolao.",
        ))
    } else {
        Ok(())
    }
}

#[cfg(feature = "server")]
pub(crate) async fn generate_invite_code(pool: &sqlx::SqlitePool) -> Result<String, ServerFnError> {
    use uuid::Uuid;

    for _ in 0..5 {
        let code = Uuid::new_v4().simple().to_string()[..6].to_uppercase();

        let exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM pools WHERE invite_code = ?1")
                .bind(&code)
                .fetch_optional(pool)
                .await
                .map_err(|e| crate::security::internal_error("generate_invite_code_lookup", e))?;

        if exists.is_none() {
            return Ok(code);
        }
    }

    Err(crate::security::public_error(
        "Não foi possível gerar um código de convite. Tente novamente.",
    ))
}

#[cfg(feature = "server")]
pub(crate) fn normalize_invite_code(value: String) -> Result<String, ServerFnError> {
    let value = crate::security::normalize_required_text("Codigo de convite", value, 6, 64)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(crate::security::public_error("Codigo de convite invalido."));
    }
    Ok(value.to_uppercase())
}

#[cfg(feature = "server")]
pub(crate) fn timestamp_elapsed(value: &str) -> bool {
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(value) {
        return parsed <= chrono::Utc::now();
    }
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .map(|parsed| parsed <= chrono::Utc::now().naive_utc())
        .unwrap_or(false)
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct InviteRecord {
    pub(crate) pool_id: String,
    pub(crate) event_id: String,
    pub(crate) pool_name: String,
    pub(crate) created_by: String,
    pub(crate) description: String,
    pub(crate) visible_rules: String,
    pub(crate) join_closed_at: Option<String>,
    pub(crate) event_name: String,
    pub(crate) event_slug: String,
    pub(crate) event_kind: String,
    pub(crate) event_status: String,
    pub(crate) event_ends_at: Option<String>,
    pub(crate) event_description: Option<String>,
    pub(crate) cover_url: Option<String>,
    pub(crate) cover_asset_id: Option<String>,
    pub(crate) creator_display_name: String,
    pub(crate) member_count: i64,
    pub(crate) lock_deadline: Option<String>,
}

#[cfg(feature = "server")]
pub(crate) async fn resolve_invite(
    db: &sqlx::SqlitePool,
    invite_code: &str,
) -> Result<Option<InviteRecord>, ServerFnError> {
    sqlx::query_as::<_, InviteRecord>(
        "SELECT p.id AS pool_id,p.event_id,p.name AS pool_name,p.created_by,p.description,p.visible_rules,p.join_closed_at,
                v.name AS event_name,e.slug AS event_slug,e.kind AS event_kind,e.status AS event_status,
                e.ends_at AS event_ends_at,v.description AS event_description,v.cover_url,v.cover_asset_id,
                u.username AS creator_display_name,
                (SELECT COUNT(*) FROM pool_members pm2 WHERE pm2.pool_id=p.id) AS member_count,
                COALESCE((SELECT MIN(pi.lock_at) FROM prediction_items pi
                          WHERE pi.event_version_id=p.event_version_id),e.ends_at) AS lock_deadline
         FROM pools p
         JOIN event_versions v ON v.id=p.event_version_id
         JOIN events e ON e.id=p.event_id
         JOIN users u ON u.id=p.created_by
         WHERE upper(p.invite_code)=?1",
    )
    .bind(invite_code)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("invite_resolve", e))
}
