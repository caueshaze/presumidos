use crate::{
    error::ServerFnError,
    models::{PoolTieBreakConfig, PoolTieBreakMode, TieBreakPriority},
};

async fn priorities_for(
    pool_id: &str,
    mode: &PoolTieBreakMode,
) -> Result<Vec<TieBreakPriority>, ServerFnError> {
    let db = crate::db::pool();
    let sql = match mode {
        PoolTieBreakMode::Inherit => "SELECT pi.id,pi.title,pi.kind,pi.tie_break_priority FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id WHERE p.id=?1 AND pi.tie_break_priority IS NOT NULL ORDER BY pi.tie_break_priority,pi.id",
        PoolTieBreakMode::Custom => "SELECT pi.id,pi.title,pi.kind,t.priority FROM pool_tiebreak_items t JOIN prediction_items pi ON pi.id=t.item_id WHERE t.pool_id=?1 ORDER BY t.priority,pi.id",
        PoolTieBreakMode::Disabled => return Ok(Vec::new()),
    };
    let rows: Vec<(String, String, String, i64)> = sqlx::query_as(sql)
        .bind(pool_id)
        .fetch_all(db)
        .await
        .map_err(|e| crate::security::internal_error("pool_tiebreak_priorities", e))?;
    Ok(rows
        .into_iter()
        .map(|(item_id, title, kind, priority)| TieBreakPriority {
            item_id,
            title,
            kind,
            priority,
        })
        .collect())
}

pub async fn effective_priorities_for_pool(
    pool_id: &str,
) -> Result<Vec<TieBreakPriority>, ServerFnError> {
    let db = crate::db::pool();
    let mode: Option<(String,)> = sqlx::query_as("SELECT COALESCE(c.mode,'inherit') FROM pools p LEFT JOIN pool_tiebreak_configs c ON c.pool_id=p.id WHERE p.id=?1")
        .bind(pool_id).fetch_optional(db).await.map_err(|e| crate::security::internal_error("pool_tiebreak_effective", e))?;
    let mode = mode.ok_or_else(|| crate::security::public_error("Bolão não encontrado."))?;
    priorities_for(
        pool_id,
        &PoolTieBreakMode::parse(&mode.0).unwrap_or(PoolTieBreakMode::Inherit),
    )
    .await
}

pub async fn config(token: String, pool_id: String) -> Result<PoolTieBreakConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let row: Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT p.id,COALESCE(c.mode,'inherit'),p.created_by,p.predictions_closed_at,p.closed_at
         FROM pools p LEFT JOIN pool_tiebreak_configs c ON c.pool_id=p.id
         WHERE p.id=?1 AND EXISTS(SELECT 1 FROM pool_members WHERE pool_id=p.id AND user_id=?2)",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("pool_tiebreak_config", e))?;
    let Some((_, mode, owner, predictions_closed_at, closed_at)) = row else {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    };
    let mode = PoolTieBreakMode::parse(&mode)
        .ok_or_else(|| crate::security::public_error("Configuração de desempate inválida."))?;
    let effective_priorities = priorities_for(&pool_id, &mode).await?;
    let custom_priorities = priorities_for(&pool_id, &PoolTieBreakMode::Custom).await?;
    Ok(PoolTieBreakConfig {
        can_edit: owner.as_deref() == Some(&session.user_id)
            && predictions_closed_at.is_none()
            && closed_at.is_none(),
        mode,
        effective_priorities,
        custom_priorities,
    })
}

pub async fn update_config(
    token: String,
    pool_id: String,
    mode: PoolTieBreakMode,
    item_ids: Vec<String>,
    csrf: String,
) -> Result<PoolTieBreakConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if mode != PoolTieBreakMode::Custom && !item_ids.is_empty() {
        return Err(crate::security::public_error(
            "Somente o modo personalizado aceita prioridades.",
        ));
    }
    if mode == PoolTieBreakMode::Custom && item_ids.is_empty() {
        return Err(crate::security::public_error(
            "O modo personalizado precisa de pelo menos uma pergunta.",
        ));
    }
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("pool_tiebreak_begin", e))?;
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT p.id FROM pools p JOIN events e ON e.id=p.event_id
         WHERE p.id=?1 AND p.created_by=?2 AND e.kind='custom' AND p.predictions_closed_at IS NULL AND p.closed_at IS NULL",
    ).bind(&pool_id).bind(&session.user_id).fetch_optional(&mut *tx).await
        .map_err(|e| crate::security::internal_error("pool_tiebreak_owner", e))?;
    if allowed.is_none() {
        return Err(crate::security::public_error(
            "Somente o dono pode alterar o desempate enquanto os palpites estiverem abertos.",
        ));
    }
    let distinct = item_ids.iter().collect::<std::collections::HashSet<_>>();
    if distinct.len() != item_ids.len() {
        return Err(crate::security::public_error(
            "Uma pergunta não pode aparecer duas vezes no desempate.",
        ));
    }
    if mode == PoolTieBreakMode::Custom {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items pi JOIN pools p ON p.event_version_id=pi.event_version_id WHERE p.id=?1 AND pi.id IN (SELECT value FROM json_each(?2)) AND pi.kind IN ('single_choice','numeric','multiple_choice')")
            .bind(&pool_id).bind(serde_json::to_string(&item_ids).unwrap()).fetch_one(&mut *tx).await
            .map_err(|e| crate::security::internal_error("pool_tiebreak_items", e))?;
        if count.0 != item_ids.len() as i64 {
            return Err(crate::security::public_error(
                "Há pergunta inválida para este bolão.",
            ));
        }
    }
    sqlx::query("INSERT INTO pool_tiebreak_configs(pool_id,mode,updated_at) VALUES(?1,?2,datetime('now')) ON CONFLICT(pool_id) DO UPDATE SET mode=excluded.mode,updated_at=excluded.updated_at")
        .bind(&pool_id).bind(mode.as_str()).execute(&mut *tx).await.map_err(|e| crate::security::internal_error("pool_tiebreak_config_save", e))?;
    sqlx::query("DELETE FROM pool_tiebreak_items WHERE pool_id=?1")
        .bind(&pool_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("pool_tiebreak_delete", e))?;
    for (priority, item_id) in item_ids.iter().enumerate() {
        sqlx::query("INSERT INTO pool_tiebreak_items(pool_id,item_id,priority) VALUES(?1,?2,?3)")
            .bind(&pool_id)
            .bind(item_id)
            .bind(priority as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("pool_tiebreak_insert", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("pool_tiebreak_commit", e))?;
    config(String::new(), pool_id).await
}

pub async fn update_event_default(
    token: String,
    event_id: String,
    item_ids: Vec<String>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let (_, db, version_id) = crate::custom_events::owner(token, &event_id, Some(csrf)).await?;
    let distinct = item_ids.iter().collect::<std::collections::HashSet<_>>();
    if distinct.len() != item_ids.len() {
        return Err(crate::security::public_error(
            "Uma pergunta não pode aparecer duas vezes no desempate.",
        ));
    }
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("event_tiebreak_begin", e))?;
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1 AND id IN (SELECT value FROM json_each(?2))")
        .bind(&version_id).bind(serde_json::to_string(&item_ids).unwrap()).fetch_one(&mut *tx).await.map_err(|e| crate::security::internal_error("event_tiebreak_validate", e))?;
    if count.0 != item_ids.len() as i64 {
        return Err(crate::security::public_error(
            "Há pergunta inválida nesta edição.",
        ));
    }
    sqlx::query("UPDATE prediction_items SET tie_break_priority=NULL WHERE event_version_id=?1")
        .bind(&version_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("event_tiebreak_clear", e))?;
    for (priority, item_id) in item_ids.iter().enumerate() {
        sqlx::query(
            "UPDATE prediction_items SET tie_break_priority=?3 WHERE id=?1 AND event_version_id=?2",
        )
        .bind(item_id)
        .bind(&version_id)
        .bind(priority as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("event_tiebreak_set", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("event_tiebreak_commit", e))?;
    Ok(())
}
