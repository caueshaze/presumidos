use crate::error::ServerFnError;

/// Identificador estável do único evento operacional desta fase.
pub const WORLD_CUP_2026_SLUG: &str = "world-cup-2026";

/// Resolve o evento padrão sem acoplar os fluxos de pool ao ID do seed.
#[cfg(feature = "server")]
pub async fn world_cup_2026_event_id(db: &sqlx::SqlitePool) -> Result<String, ServerFnError> {
    let event: Option<(String,)> = sqlx::query_as("SELECT id FROM events WHERE slug = ?1")
        .bind(WORLD_CUP_2026_SLUG)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("world_cup_2026_event_lookup", e))?;

    event.map(|(id,)| id).ok_or_else(|| {
        crate::security::internal_error("world_cup_2026_event_missing", "seed ausente")
    })
}
