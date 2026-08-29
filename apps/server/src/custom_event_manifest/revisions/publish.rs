use super::super::*;
use super::working::RestoreVersionResult;
use crate::error::ServerFnError;

pub async fn restore_published_version(
    event_id: &str,
    source_version_id: &str,
    actor: &str,
) -> Result<RestoreVersionResult, ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    crate::security::validate_uuid("Versão", source_version_id)?;
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("version_restore_begin", e))?;

    let source: (String, i64, String, Option<String>, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT v.id,v.version_number,v.name,v.description,v.cover_url,v.cover_asset_id,v.external_url
         FROM event_versions v JOIN events e ON e.id=v.event_id
         WHERE v.id=?1 AND v.event_id=?2 AND v.state='published' AND e.kind='custom'",
    )
    .bind(source_version_id)
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_source", e))?
    .ok_or_else(|| crate::security::public_error("A versão publicada não foi encontrada."))?;

    let manifest = load_manifest_conn(&mut *tx, event_id, Some(source_version_id)).await?;
    let restored_fingerprint = fingerprint(&manifest).map_err(crate::security::public_error)?;
    let current_fingerprint: Option<(String,)> = sqlx::query_as(
        "SELECT fingerprint FROM event_versions WHERE event_id=?1 AND state='published' AND is_current_published=1",
    )
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_current", e))?;
    let base_fingerprint = match current_fingerprint.map(|row| row.0) {
        Some(value) if !value.is_empty() => value,
        _ => fingerprint(&load_manifest_conn(&mut *tx, event_id, None).await?)
            .map_err(crate::security::public_error)?,
    };

    let working: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_working", e))?;
    let replaced_working_version_id = working.as_ref().map(|row| row.0.clone());
    if let Some((working_id,)) = working {
        let pool_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pools WHERE event_version_id=?1")
                .bind(&working_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| crate::security::internal_error("version_restore_working_pools", e))?;
        if pool_count.0 > 0 {
            return Err(crate::security::public_error(
                "A revisão atual já está vinculada a bolões e não pode ser substituída.",
            ));
        }
        sqlx::query("DELETE FROM official_results WHERE event_version_id=?1")
            .bind(&working_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("version_restore_working_results", e))?;
        sqlx::query("DELETE FROM prediction_items WHERE event_version_id=?1")
            .bind(&working_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("version_restore_working_items", e))?;
        sqlx::query("DELETE FROM event_versions WHERE id=?1 AND state='working'")
            .bind(&working_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("version_restore_working_delete", e))?;
    }

    let version_id = uuid::Uuid::new_v4().to_string();
    let next_number: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(version_number),0)+1 FROM event_versions WHERE event_id=?1",
    )
    .bind(event_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_number", e))?;
    sqlx::query(
        "INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,base_fingerprint,created_by)
         VALUES(?1,?2,?3,'working',0,?4,?5,?6,?7,?8,?9,?10,?11)",
    )
    .bind(&version_id)
    .bind(event_id)
    .bind(next_number.0)
    .bind(&source.2)
    .bind(&source.3)
    .bind(&source.4)
    .bind(&source.5)
    .bind(&source.6)
    .bind(&restored_fingerprint)
    .bind(&base_fingerprint)
    .bind(actor)
    .execute(&mut *tx)
    .await
    .map_err(|e| crate::security::internal_error("version_restore_insert", e))?;
    insert_items(&mut tx, event_id, &version_id, &manifest).await?;
    sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,ip_address,details_json) VALUES(?1,?2,'event_version_restored','event',?3,NULL,?4)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(actor)
        .bind(event_id)
        .bind(serde_json::json!({
            "sourceVersionId": source.0,
            "sourceVersionNumber": source.1,
            "newVersionId": version_id,
            "newVersionNumber": next_number.0,
            "replacedWorkingVersionId": replaced_working_version_id,
            "itemCount": manifest.items.len(),
        }).to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("version_restore_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("version_restore_commit", e))?;
    Ok(RestoreVersionResult {
        version_id,
        version_number: next_number.0,
        source_version_id: source.0,
        source_version_number: source.1,
        replaced_working_version_id,
    })
}

#[cfg(feature = "server")]
pub async fn publish_working_revision(
    event_id: &str,
    version_id: Option<&str>,
    actor: &str,
) -> Result<(), ServerFnError> {
    crate::security::validate_uuid("Evento", event_id)?;
    let db = crate::db::pool();
    let mut tx = db
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| crate::security::internal_error("working_publish_begin", e))?;
    let version: (String, i64) = if let Some(version_id) = version_id {
        sqlx::query_as("SELECT id,version_number FROM event_versions WHERE id=?1 AND event_id=?2 AND state='working'")
            .bind(version_id).bind(event_id).fetch_optional(&mut *tx).await
            .map_err(|e| crate::security::internal_error("working_publish_lookup", e))?
            .ok_or_else(|| crate::security::public_error("Revisão de evento não encontrada ou já publicada."))?
    } else {
        sqlx::query_as("SELECT id,version_number FROM event_versions WHERE event_id=?1 AND state='working' ORDER BY version_number DESC LIMIT 1")
            .bind(event_id).fetch_optional(&mut *tx).await
            .map_err(|e| crate::security::internal_error("working_publish_latest", e))?
            .ok_or_else(|| crate::security::public_error("Não há revisão pendente para publicar."))?
    };
    let invalid: Option<(String,)> = sqlx::query_as(
        "SELECT pi.title FROM prediction_items pi LEFT JOIN custom_question_options o ON o.item_id=pi.id LEFT JOIN multiple_choice_questions mq ON mq.item_id=pi.id WHERE pi.event_version_id=?1 AND pi.kind IN ('single_choice','multiple_choice') GROUP BY pi.id HAVING COUNT(o.id)<2 OR (pi.kind='multiple_choice' AND (mq.min_selections<1 OR COALESCE(mq.max_selections,COUNT(o.id))<mq.min_selections OR COALESCE(mq.max_selections,COUNT(o.id))>COUNT(o.id))) LIMIT 1",
    )
    .bind(&version.0).fetch_optional(&mut *tx).await
    .map_err(|e| crate::security::internal_error("working_publish_validate", e))?;
    if let Some((title,)) = invalid {
        return Err(crate::security::public_error(&format!(
            "{title} precisa ter pelo menos 2 opções."
        )));
    }
    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE event_version_id=?1")
            .bind(&version.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("working_publish_count", e))?;
    if total.0 == 0 {
        return Err(crate::security::public_error(
            "O evento precisa ter pelo menos uma pergunta.",
        ));
    }
    sqlx::query("UPDATE event_versions SET state='published',is_current_published=0,updated_at=datetime('now') WHERE event_id=?1 AND state='published'")
        .bind(event_id).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("working_publish_previous", e))?;
    sqlx::query("UPDATE event_versions SET state='published',is_current_published=1,updated_at=datetime('now') WHERE id=?1")
        .bind(&version.0).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("working_publish_version", e))?;
    sqlx::query("UPDATE events SET current_published_version_id=?2,status='active',updated_at=datetime('now') WHERE id=?1")
        .bind(event_id).bind(&version.0).execute(&mut *tx).await
        .map_err(|e| crate::security::internal_error("working_publish_event", e))?;
    sqlx::query("INSERT INTO audit_logs(id,actor_user_id,action,target_type,target_id,ip_address,details_json) VALUES(?1,?2,'event_version_published','event',?3,NULL,?4)")
        .bind(uuid::Uuid::new_v4().to_string()).bind(actor).bind(event_id)
        .bind(serde_json::json!({"versionId":version.0,"versionNumber":version.1}).to_string())
        .execute(&mut *tx).await.map_err(|e| crate::security::internal_error("working_publish_audit", e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("working_publish_commit", e))?;
    Ok(())
}
