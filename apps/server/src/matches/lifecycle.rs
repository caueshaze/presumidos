use crate::error::ServerFnError;

#[cfg(feature = "server")]
pub(super) async fn knockout_released_flag() -> Result<bool, ServerFnError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = 'knockout_released'")
            .fetch_optional(crate::db::pool())
            .await
            .map_err(|error| crate::security::internal_error("knockout_released_flag", error))?;
    Ok(row.map(|(value,)| value == "1").unwrap_or(false))
}

#[cfg(feature = "server")]
pub async fn is_knockout_released() -> Result<bool, ServerFnError> {
    knockout_released_flag().await
}

#[cfg(feature = "server")]
pub(super) async fn token_is_admin(token: &str) -> bool {
    crate::auth::require_admin(token).await.is_ok()
}

/// Fecha o flag operacional das partidas de edições encerradas, sem inventar
/// resultado oficial nem alterar a pontuação.
#[cfg(feature = "server")]
pub(crate) async fn force_finish_matches_for_ended_events() -> Result<u64, ServerFnError> {
    force_finish_matches_in(
        crate::db::pool(),
        "AND (e.status = 'finished' OR (e.ends_at IS NOT NULL AND datetime(e.ends_at) <= datetime('now')))",
        None,
    )
    .await
}

#[cfg(feature = "server")]
pub(crate) async fn force_finish_matches_for_event(event_id: &str) -> Result<u64, ServerFnError> {
    force_finish_matches_in(crate::db::pool(), "AND e.id = ?1", Some(event_id)).await
}

#[cfg(feature = "server")]
async fn force_finish_matches_in(
    db: &sqlx::SqlitePool,
    event_clause: &str,
    event_id: Option<&str>,
) -> Result<u64, ServerFnError> {
    let sql = format!(
        "UPDATE matches SET finished = 1 WHERE finished = 0 AND EXISTS (
             SELECT 1 FROM prediction_items pi JOIN events e ON e.id = pi.event_id
             WHERE pi.id = matches.prediction_item_id {event_clause}
         )"
    );
    let mut query = sqlx::query(&sql);
    if let Some(event_id) = event_id {
        query = query.bind(event_id);
    }
    let result = query
        .execute(db)
        .await
        .map_err(|error| crate::security::internal_error("force_finish_event_matches", error))?;
    Ok(result.rows_affected())
}

/// Cria, se necessário, a revisão de trabalho isolada da versão publicada.
#[cfg(feature = "server")]
pub(crate) async fn ensure_football_working_revision(
    event_id: &str,
    actor: &str,
) -> Result<String, ServerFnError> {
    use uuid::Uuid;

    let db = crate::db::pool();
    let mut tx = db
        .begin()
        .await
        .map_err(|error| crate::security::internal_error("football_revision_begin", error))?;
    if let Some((version_id,)) = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| crate::security::internal_error("football_revision_existing", error))? {
        tx.commit().await.map_err(|error| {
            crate::security::internal_error("football_revision_existing_commit", error)
        })?;
        return Ok(version_id);
    }
    let Some((published_id, published_number, name, description, cover_url, cover_asset_id, external_url, base_fingerprint)) = sqlx::query_as::<_, (String, i64, String, Option<String>, Option<String>, Option<String>, Option<String>, String)>(
        "SELECT id,version_number,name,description,cover_url,cover_asset_id,external_url,fingerprint FROM event_versions WHERE event_id=?1 AND state='published' ORDER BY is_current_published DESC, version_number DESC LIMIT 1",
    )
    .bind(event_id).fetch_optional(&mut *tx).await
    .map_err(|error| crate::security::internal_error("football_revision_published", error))? else {
        return Err(crate::security::public_error("O evento de futebol ainda não possui versão publicada."));
    };
    let version_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO event_versions (id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,base_fingerprint,created_by) VALUES(?1,?2,?3,'working',0,?4,?5,?6,?7,?8,?9,?10,?11)")
        .bind(&version_id).bind(event_id).bind(published_number + 1).bind(&name).bind(&description).bind(&cover_url).bind(&cover_asset_id).bind(&external_url).bind(format!("football-working-{version_id}")).bind(&base_fingerprint).bind(actor)
        .execute(&mut *tx).await.map_err(|error| crate::security::internal_error("football_revision_insert", error))?;
    let items: Vec<(String, Option<String>, String, String, Option<String>, String, String, i64, String)> = sqlx::query_as(
        "SELECT id,external_key,kind,title,description,lock_at,reveal_at,sort_order,status FROM prediction_items WHERE event_version_id=?1 ORDER BY sort_order,id",
    ).bind(&published_id).fetch_all(&mut *tx).await.map_err(|error| crate::security::internal_error("football_revision_items", error))?;
    for (
        old_item_id,
        external_key,
        kind,
        title,
        item_description,
        lock_at,
        reveal_at,
        sort_order,
        status,
    ) in &items
    {
        let new_item_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO prediction_items (id,event_id,event_version_id,external_key,kind,title,description,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)")
            .bind(&new_item_id).bind(event_id).bind(&version_id).bind(external_key).bind(kind).bind(title).bind(item_description).bind(lock_at).bind(reveal_at).bind(sort_order).bind(status)
            .execute(&mut *tx).await.map_err(|error| crate::security::internal_error("football_revision_item_copy", error))?;
        let matches: Vec<(String, String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id,home_team,away_team,kickoff,group_name,phase FROM matches WHERE prediction_item_id=?1 AND event_version_id=?2",
        ).bind(old_item_id).bind(&published_id).fetch_all(&mut *tx).await.map_err(|error| crate::security::internal_error("football_revision_matches", error))?;
        for (old_match_id, home, away, kickoff, group_name, phase) in matches {
            sqlx::query("INSERT INTO matches (id,prediction_item_id,event_version_id,source_match_id,home_team,away_team,kickoff,group_name,phase) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)")
                .bind(Uuid::new_v4().to_string()).bind(&new_item_id).bind(&version_id).bind(&old_match_id).bind(home).bind(away).bind(kickoff).bind(group_name).bind(phase)
                .execute(&mut *tx).await.map_err(|error| crate::security::internal_error("football_revision_match_copy", error))?;
        }
    }
    tx.commit()
        .await
        .map_err(|error| crate::security::internal_error("football_revision_commit", error))?;
    Ok(version_id)
}

#[cfg(feature = "server")]
pub(super) async fn working_match_for(
    db: &sqlx::SqlitePool,
    event_id: &str,
    match_id: &str,
) -> Result<(String, String), ServerFnError> {
    let direct: Option<(String, String)> = sqlx::query_as("SELECT m.id,m.prediction_item_id FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id JOIN event_versions v ON v.id=pi.event_version_id WHERE m.id=?1 AND pi.event_id=?2 AND v.state='working'")
        .bind(match_id).bind(event_id).fetch_optional(db).await.map_err(|error| crate::security::internal_error("football_working_match_lookup", error))?;
    if let Some(found) = direct {
        return Ok(found);
    }
    sqlx::query_as("SELECT m.id,m.prediction_item_id FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id JOIN event_versions v ON v.id=pi.event_version_id WHERE m.source_match_id=?1 AND pi.event_id=?2 AND v.state='working'")
        .bind(match_id).bind(event_id).fetch_optional(db).await.map_err(|error| crate::security::internal_error("football_working_match_source_lookup", error))?
        .ok_or_else(|| crate::security::public_error("Não foi possível localizar a partida na revisão de trabalho."))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::force_finish_matches_in;

    #[tokio::test]
    async fn elapsed_event_finishes_matches_without_inventing_a_result() {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql("CREATE TABLE events (id TEXT PRIMARY KEY, status TEXT NOT NULL, ends_at TEXT); CREATE TABLE prediction_items (id TEXT PRIMARY KEY, event_id TEXT NOT NULL); CREATE TABLE matches (id TEXT PRIMARY KEY, prediction_item_id TEXT NOT NULL, finished INTEGER NOT NULL DEFAULT 0, home_score INTEGER, away_score INTEGER); INSERT INTO events VALUES ('past','active','2020-01-01T00:00:00Z'),('future','active','2099-01-01T00:00:00Z'); INSERT INTO prediction_items VALUES ('past-item','past'),('future-item','future'); INSERT INTO matches (id,prediction_item_id) VALUES ('past-match','past-item'),('future-match','future-item');").execute(&db).await.unwrap();
        assert_eq!(force_finish_matches_in(&db, "AND (e.status = 'finished' OR (e.ends_at IS NOT NULL AND datetime(e.ends_at) <= datetime('now')))", None).await.unwrap(), 1);
        let past: (bool, Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT finished,home_score,away_score FROM matches WHERE id='past-match'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(past, (true, None, None));
        let future: (bool,) =
            sqlx::query_as("SELECT finished FROM matches WHERE id='future-match'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert!(!future.0);
    }
}
