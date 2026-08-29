use crate::error::ServerFnError;
use crate::models::CustomPredictionValue;

pub async fn set_option_media_seen(
    token: String,
    pool_id: String,
    option_id: String,
    seen: bool,
    csrf_token: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf_token)?;
    let db = crate::db::pool();
    let allowed: (i64,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM pool_members pm JOIN pools p ON p.id=pm.pool_id JOIN prediction_items pi ON pi.event_version_id=p.event_version_id JOIN custom_question_options o ON o.item_id=pi.id JOIN option_links l ON l.option_id=o.id WHERE pm.pool_id=?1 AND pm.user_id=?2 AND o.id=?3)")
        .bind(&pool_id).bind(&session.user_id).bind(&option_id).fetch_one(db).await.map_err(|e| crate::security::internal_error("set_option_media_seen_allowed", e))?;
    if allowed.0 == 0 {
        return Err(crate::security::public_error(
            "Mídia não disponível neste bolão.",
        ));
    }
    if seen {
        sqlx::query("INSERT INTO option_media_progress(user_id,option_id) VALUES(?1,?2) ON CONFLICT(user_id,option_id) DO UPDATE SET seen_at=datetime('now')").bind(&session.user_id).bind(&option_id).execute(db).await.map_err(|e| crate::security::internal_error("set_option_media_seen", e))?;
    } else {
        sqlx::query("DELETE FROM option_media_progress WHERE user_id=?1 AND option_id=?2")
            .bind(&session.user_id)
            .bind(&option_id)
            .execute(db)
            .await
            .map_err(|e| crate::security::internal_error("clear_option_media_seen", e))?;
    }
    Ok(())
}

#[cfg(feature = "server")]
pub async fn event_showcase(
    token: String,
    pool_id: String,
) -> Result<crate::models::EventShowcase, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let row: Option<(String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,String,i64,i64)> = sqlx::query_as("SELECT v.name,v.description,v.cover_url,v.external_url,v.cover_asset_id,e.starts_at,e.ends_at,e.status,(SELECT COUNT(*) FROM prediction_items pi WHERE pi.event_version_id=p.event_version_id),(SELECT COUNT(*) FROM predictions pr WHERE pr.pool_id=p.id AND pr.user_id=?2) FROM pools p JOIN events e ON e.id=p.event_id JOIN event_versions v ON v.id=p.event_version_id JOIN pool_members pm ON pm.pool_id=p.id AND pm.user_id=?2 WHERE p.id=?1")
        .bind(&pool_id).bind(&session.user_id).fetch_optional(db).await.map_err(|e| crate::security::internal_error("event_showcase", e))?;
    let Some((
        name,
        description,
        cover_url,
        external_url,
        cover_asset_id,
        starts_at,
        ends_at,
        status,
        item_count,
        answered_count,
    )) = row
    else {
        return Err(crate::security::public_error(
            "Bolão não encontrado ou sem acesso.",
        ));
    };
    let is_historical = status == "finished"
        || ends_at
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|value| value <= chrono::Utc::now());
    Ok(crate::models::EventShowcase {
        name,
        description,
        cover_url,
        cover_asset_url: cover_asset_id.map(|asset_id| format!("/media/assets/{asset_id}/cover")),
        external_url,
        starts_at,
        ends_at,
        item_count,
        answered_count,
        is_historical,
    })
}

#[cfg(feature = "server")]
pub async fn custom_prediction_value(
    prediction_id: &str,
) -> Result<Option<CustomPredictionValue>, ServerFnError> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT prediction_id, option_id FROM custom_prediction_values WHERE prediction_id=?1",
    )
    .bind(prediction_id)
    .fetch_optional(crate::db::pool())
    .await
    .map(|v| {
        v.map(|(prediction_id, option_id)| CustomPredictionValue {
            prediction_id,
            option_id,
        })
    })
    .map_err(|e| crate::security::internal_error("custom_prediction_value", e))
}
