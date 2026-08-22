use crate::error::ServerFnError;

pub const FOOTBALL_MATCH_KIND: &str = "football_match";

#[cfg(feature = "server")]
pub fn football_match_title(home_team: &str, away_team: &str) -> String {
    format!("{home_team} x {away_team}")
}

/// Cria o item pertencente ao match dentro da transação que cria a partida.
#[cfg(feature = "server")]
pub async fn create_football_match_item(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    event_id: &str,
    item_id: &str,
    home_team: &str,
    away_team: &str,
    kickoff: &str,
) -> Result<(), ServerFnError> {
    let sort_order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM prediction_items WHERE event_id = ?1",
    )
    .bind(event_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| crate::security::internal_error("create_prediction_item_sort_order", e))?;

    sqlx::query(
        "INSERT INTO prediction_items
            (id, event_id, kind, title, lock_at, reveal_at, sort_order, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, 'open')",
    )
    .bind(item_id)
    .bind(event_id)
    .bind(FOOTBALL_MATCH_KIND)
    .bind(football_match_title(home_team, away_team))
    .bind(kickoff)
    .bind(sort_order.0)
    .execute(&mut **tx)
    .await
    .map_err(|e| crate::security::internal_error("create_prediction_item_insert", e))?;
    Ok(())
}
