use crate::{
    error::ServerFnError,
    models::{OptionLink, PoolEditorialConfig, PoolEditorialOption},
};

#[derive(Clone)]
pub struct PoolEditorialLinkInput {
    pub kind: String,
    pub label: String,
    pub url: String,
}

async fn owner(
    token: String,
    pool_id: &str,
    csrf: Option<String>,
) -> Result<crate::auth::AuthSession, ServerFnError> {
    crate::security::validate_uuid("Bolão", pool_id)?;
    let session = crate::auth::require_user(&token).await?;
    if let Some(csrf) = csrf {
        crate::security::require_csrf(&session.csrf_token, &csrf)?;
    }
    let allowed: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM pools WHERE id=?1 AND created_by=?2 AND predictions_closed_at IS NULL AND closed_at IS NULL",
    )
    .bind(pool_id).bind(&session.user_id).fetch_optional(crate::db::pool()).await
    .map_err(|e| crate::security::internal_error("pool_editorial_owner", e))?;
    if allowed.is_none() {
        return Err(crate::security::public_error(
            "Somente o dono pode editar este bolão enquanto os palpites estiverem abertos.",
        ));
    }
    Ok(session)
}

pub async fn editorial_config(
    token: String,
    pool_id: String,
) -> Result<PoolEditorialConfig, ServerFnError> {
    owner(token, &pool_id, None).await?;
    let db = crate::db::pool();
    let name: (String,) = sqlx::query_as("SELECT name FROM pools WHERE id=?1")
        .bind(&pool_id)
        .fetch_one(db)
        .await
        .map_err(|e| crate::security::internal_error("pool_editorial_load", e))?;
    let rows: Vec<(String, String, String, String, i64)> = sqlx::query_as(
        "SELECT pi.id,pi.title,o.id,o.label,EXISTS(SELECT 1 FROM pool_option_link_overrides x WHERE x.pool_id=?1 AND x.option_id=o.id)
         FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
         JOIN custom_question_options o ON o.item_id=pi.id
         WHERE p.id=?1 ORDER BY pi.sort_order,o.sort_order",
    ).bind(&pool_id).fetch_all(db).await.map_err(|e| crate::security::internal_error("pool_editorial_options", e))?;
    let mut options = Vec::with_capacity(rows.len());
    for (item_id, item_title, option_id, option_label, is_customized) in rows {
        let links = effective_links(db, &pool_id, &option_id).await?;
        options.push(PoolEditorialOption {
            item_id,
            item_title,
            option_id,
            option_label,
            links,
            is_customized: is_customized != 0,
        });
    }
    Ok(PoolEditorialConfig {
        pool_id,
        name: name.0,
        options,
    })
}

pub async fn update_editorial_name(
    token: String,
    pool_id: String,
    name: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = owner(token, &pool_id, Some(csrf)).await?;
    let name = crate::security::normalize_required_text("Nome do bolão", name, 3, 80)?;
    sqlx::query("UPDATE pools SET name=?2 WHERE id=?1")
        .bind(&pool_id)
        .bind(&name)
        .execute(crate::db::pool())
        .await
        .map_err(|e| crate::security::internal_error("pool_editorial_name", e))?;
    audit(
        &session.user_id,
        &pool_id,
        "pool_editorial_name_updated",
        serde_json::json!({"fields":["name"]}),
    )
    .await
}

pub async fn replace_option_links(
    token: String,
    pool_id: String,
    option_id: String,
    links: Vec<PoolEditorialLinkInput>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = owner(token, &pool_id, Some(csrf)).await?;
    crate::security::validate_uuid("Opção", &option_id)?;
    validate_pool_option(&pool_id, &option_id).await?;
    if links.len() > crate::custom_event_manifest::MAX_LINKS_PER_OPTION {
        return Err(crate::security::public_error("Limite de links atingido."));
    }
    let mut normalized = Vec::with_capacity(links.len());
    for link in links {
        let kind = crate::security::normalize_required_text("Tipo do link", link.kind, 1, 64)?;
        if !matches!(kind.as_str(), "video" | "audio" | "official" | "other") {
            return Err(crate::security::public_error("Tipo do link inválido."));
        }
        let label = crate::security::normalize_required_text("Rótulo do link", link.label, 1, 240)?;
        let url =
            crate::custom_event_manifest::validate_optional_http_url(Some(link.url), "URL do link")
                .map_err(crate::security::public_error)?
                .ok_or_else(|| crate::security::public_error("URL do link inválida."))?;
        normalized.push((kind, label, url));
    }
    let db = crate::db::pool();
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("pool_editorial_begin", e))?;
    sqlx::query("INSERT INTO pool_option_link_overrides(pool_id,option_id) VALUES(?1,?2) ON CONFLICT(pool_id,option_id) DO UPDATE SET updated_at=datetime('now')")
        .bind(&pool_id).bind(&option_id).execute(&mut *tx).await.map_err(|e| crate::security::internal_error("pool_editorial_override", e))?;
    sqlx::query("DELETE FROM pool_option_editorial_links WHERE pool_id=?1 AND option_id=?2")
        .bind(&pool_id)
        .bind(&option_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::security::internal_error("pool_editorial_clear", e))?;
    for (sort_order, (kind, label, url)) in normalized.iter().enumerate() {
        sqlx::query("INSERT INTO pool_option_editorial_links(id,pool_id,option_id,kind,label,url,sort_order) VALUES(?1,?2,?3,?4,?5,?6,?7)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(&pool_id).bind(&option_id).bind(kind).bind(label).bind(url).bind(sort_order as i64).execute(&mut *tx).await
            .map_err(|e| crate::security::internal_error("pool_editorial_link", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("pool_editorial_commit", e))?;
    audit(
        &session.user_id,
        &pool_id,
        "pool_editorial_links_replaced",
        serde_json::json!({"optionId":option_id,"linkCount":normalized.len()}),
    )
    .await
}

pub async fn reset_option_links(
    token: String,
    pool_id: String,
    option_id: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = owner(token, &pool_id, Some(csrf)).await?;
    crate::security::validate_uuid("Opção", &option_id)?;
    validate_pool_option(&pool_id, &option_id).await?;
    let changed =
        sqlx::query("DELETE FROM pool_option_link_overrides WHERE pool_id=?1 AND option_id=?2")
            .bind(&pool_id)
            .bind(&option_id)
            .execute(crate::db::pool())
            .await
            .map_err(|e| crate::security::internal_error("pool_editorial_reset", e))?;
    if changed.rows_affected() == 0 {
        return Err(crate::security::public_error(
            "Esta opção já usa o padrão do evento.",
        ));
    }
    audit(
        &session.user_id,
        &pool_id,
        "pool_editorial_links_restored",
        serde_json::json!({"optionId":option_id}),
    )
    .await
}

pub async fn effective_links(
    db: &sqlx::SqlitePool,
    pool_id: &str,
    option_id: &str,
) -> Result<Vec<OptionLink>, ServerFnError> {
    let overridden: (i64,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pool_option_link_overrides WHERE pool_id=?1 AND option_id=?2)",
    )
    .bind(pool_id)
    .bind(option_id)
    .fetch_one(db)
    .await
    .map_err(|e| crate::security::internal_error("pool_editorial_effective", e))?;
    let sql = if overridden.0 != 0 {
        "SELECT kind,label,url,sort_order FROM pool_option_editorial_links WHERE pool_id=?1 AND option_id=?2 ORDER BY sort_order,id"
    } else {
        "SELECT kind,label,url,sort_order FROM option_links WHERE option_id=?2 ORDER BY sort_order,id"
    };
    let links = sqlx::query_as::<_, (String, String, String, i64)>(sql)
        .bind(pool_id)
        .bind(option_id)
        .fetch_all(db)
        .await
        .map_err(|e| crate::security::internal_error("pool_editorial_links", e))?
        .into_iter()
        .map(|(kind, label, url, sort_order)| OptionLink {
            kind,
            label,
            url,
            sort_order,
        })
        .collect();
    Ok(links)
}

async fn audit(
    actor: &str,
    pool_id: &str,
    action: &str,
    details: serde_json::Value,
) -> Result<(), ServerFnError> {
    crate::security::append_audit_log(
        crate::db::pool(),
        Some(actor),
        action,
        "pool",
        Some(pool_id),
        None,
        details,
    )
    .await
}

async fn validate_pool_option(pool_id: &str, option_id: &str) -> Result<(), ServerFnError> {
    let valid: (i64,) = sqlx::query_as(
        "SELECT EXISTS(
            SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
            JOIN custom_question_options o ON o.item_id=pi.id
            WHERE p.id=?1 AND o.id=?2
        )",
    )
    .bind(pool_id)
    .bind(option_id)
    .fetch_one(crate::db::pool())
    .await
    .map_err(|e| crate::security::internal_error("pool_editorial_option", e))?;
    if valid.0 == 0 {
        return Err(crate::security::public_error(
            "Esta opção não pertence ao bolão.",
        ));
    }
    Ok(())
}
