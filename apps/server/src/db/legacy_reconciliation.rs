//! Preservação de predictions globais incompatíveis com o schema por Pool.

use sqlx::SqlitePool;

pub(crate) async fn reconcile_legacy_predictions(pool: &SqlitePool) -> Result<u64, String> {
    let columns: Vec<(i64, String, String, i64, Option<String>, i64)> =
        sqlx::query_as("PRAGMA table_info(predictions)")
            .fetch_all(pool)
            .await
            .map_err(|error| format!("falha ao inspecionar predictions legadas: {error}"))?;
    if columns.is_empty()
        || columns
            .iter()
            .any(|(_, name, _, _, _, _)| name == "pool_id")
    {
        return Ok(0);
    }
    let required: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('users','pools','pool_members','prediction_items','matches')").fetch_one(pool).await.map_err(|error| format!("falha ao verificar schema legado: {error}"))?;
    if required.0 != 5 {
        return Ok(0);
    }
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("falha ao iniciar reconciliação legada: {error}"))?;
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS legacy_predictions_without_pool (source_id TEXT PRIMARY KEY,user_id TEXT NOT NULL,match_id TEXT NOT NULL,home_score INTEGER NOT NULL,away_score INTEGER NOT NULL,submitted_at TEXT NOT NULL,qualifier TEXT,went_to_penalties INTEGER NOT NULL,penalty_home_score INTEGER,penalty_away_score INTEGER,reason TEXT NOT NULL,archived_at TEXT NOT NULL DEFAULT(datetime('now')))").execute(&mut *tx).await.map_err(|error| format!("falha ao criar arquivo de predictions legadas: {error}"))?;
    sqlx::query("INSERT OR IGNORE INTO legacy_predictions_without_pool (source_id,user_id,match_id,home_score,away_score,submitted_at,qualifier,went_to_penalties,penalty_home_score,penalty_away_score,reason) SELECT old.id,old.user_id,old.match_id,old.home_score,old.away_score,old.submitted_at,old.qualifier,old.went_to_penalties,old.penalty_home_score,old.penalty_away_score,'no_pool_for_prediction_event' FROM predictions old WHERE NOT EXISTS (SELECT 1 FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id JOIN pools p ON p.event_id=pi.event_id JOIN pool_members pm ON pm.pool_id=p.id AND pm.user_id=old.user_id WHERE m.id=old.match_id)").execute(&mut *tx).await.map_err(|error| format!("falha ao arquivar predictions sem pool: {error}"))?;
    let deleted = sqlx::query("DELETE FROM predictions WHERE id IN (SELECT source_id FROM legacy_predictions_without_pool)").execute(&mut *tx).await.map_err(|error| format!("falha ao remover somente predictions já arquivadas: {error}"))?;
    tx.commit()
        .await
        .map_err(|error| format!("falha ao confirmar reconciliação legada: {error}"))?;
    Ok(deleted.rows_affected())
}
