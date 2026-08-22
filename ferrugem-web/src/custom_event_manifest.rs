use crate::error::ServerFnError;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifest {
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub items: Vec<CustomEventManifestItem>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifestItem {
    pub external_key: String,
    pub kind: String,
    pub title: String,
    pub description: Option<String>,
    pub lock_at: String,
    pub reveal_at: String,
    #[serde(default)]
    pub options: Vec<CustomEventManifestOption>,
    pub decimal_places: Option<i64>,
    pub unit_label: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub min_selections: Option<i64>,
    pub max_selections: Option<i64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifestOption {
    pub external_key: String,
    pub label: String,
}

/// Invariantes compartilhadas pelo importador e pelo Builder; o Builder pode
/// salvar rascunhos incompletos, mas não pode aceitar tempos inválidos.
pub fn validate_event_window(
    starts_at: &Option<String>,
    ends_at: &Option<String>,
) -> Result<(), String> {
    if let Some(value) = starts_at {
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| "startsAt inválido")?;
    }
    if let Some(value) = ends_at {
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| "endsAt inválido")?;
    }
    if let (Some(a), Some(b)) = (starts_at, ends_at) {
        if chrono::DateTime::parse_from_rfc3339(a).map_err(|_| "startsAt inválido")?
            >= chrono::DateTime::parse_from_rfc3339(b).map_err(|_| "endsAt inválido")?
        {
            return Err("startsAt deve preceder endsAt".into());
        }
    }
    Ok(())
}
pub fn validate_single_choice_timing(
    title: &str,
    external_key: &str,
    lock_at: &str,
    reveal_at: &str,
) -> Result<(), String> {
    if title.trim().is_empty() || external_key.trim().is_empty() {
        return Err("item inválido".into());
    }
    let lock = chrono::DateTime::parse_from_rfc3339(lock_at).map_err(|_| "lockAt inválido")?;
    let reveal =
        chrono::DateTime::parse_from_rfc3339(reveal_at).map_err(|_| "revealAt inválido")?;
    if lock > reveal {
        return Err("lockAt deve preceder ou igualar revealAt".into());
    }
    Ok(())
}

pub fn parse_and_validate(bytes: &str) -> Result<CustomEventManifest, String> {
    let m: CustomEventManifest =
        serde_json::from_str(bytes).map_err(|e| format!("JSON inválido: {e}"))?;
    if m.kind != "custom"
        || m.name.trim().is_empty()
        || m.slug.trim().is_empty()
        || m.items.is_empty()
    {
        return Err("manifesto custom inválido".into());
    }
    let mut keys = std::collections::HashSet::new();
    for item in &m.items {
        if !matches!(
            item.kind.as_str(),
            "single_choice" | "numeric" | "multiple_choice"
        ) || item.external_key.trim().is_empty()
            || item.title.trim().is_empty()
            || !keys.insert(&item.external_key)
        {
            return Err("item inválido ou externalKey duplicada".into());
        }
        validate_single_choice_timing(
            &item.title,
            &item.external_key,
            &item.lock_at,
            &item.reveal_at,
        )?;
        if item.kind == "single_choice" || item.kind == "multiple_choice" {
            if item.options.len() < 2 {
                return Err("pergunta baseada em opções requer pelo menos duas options".into());
            }
            let mut options = std::collections::HashSet::new();
            for o in &item.options {
                if o.external_key.trim().is_empty()
                    || o.label.trim().is_empty()
                    || !options.insert(&o.external_key)
                {
                    return Err("option inválida ou externalKey duplicada".into());
                }
            }
            if item.kind == "multiple_choice" {
                let min = item.min_selections.unwrap_or(1);
                let max = item.max_selections.unwrap_or(item.options.len() as i64);
                if min < 1 || max < min || max > item.options.len() as i64 {
                    return Err("min/max multiple_choice inválido".into());
                }
                if item.decimal_places.is_some()
                    || item.unit_label.is_some()
                    || item.min_value.is_some()
                    || item.max_value.is_some()
                {
                    return Err("multiple_choice não aceita campos numeric".into());
                }
            }
        } else {
            if !item.options.is_empty() {
                return Err("numeric não aceita options".into());
            }
            let places =
                crate::numeric::validate_question(item.decimal_places.unwrap_or(-1), None, None)?;
            let min = item
                .min_value
                .as_ref()
                .map(|v| crate::numeric::parse_scaled(v, places))
                .transpose()?;
            let max = item
                .max_value
                .as_ref()
                .map(|v| crate::numeric::parse_scaled(v, places))
                .transpose()?;
            crate::numeric::validate_question(places as i64, min, max)?;
            if item
                .unit_label
                .as_ref()
                .is_some_and(|v| v.trim().is_empty())
            {
                return Err("unidade inválida".into());
            }
        }
    }
    validate_event_window(&m.starts_at, &m.ends_at)?;
    Ok(m)
}

#[cfg(feature = "server")]
pub async fn import(
    manifest: CustomEventManifest,
    apply: bool,
) -> Result<(usize, usize), ServerFnError> {
    let count: usize = manifest.items.iter().map(|i| i.options.len()).sum();
    if !apply {
        return Ok((manifest.items.len(), count));
    }
    let db = crate::db::pool();
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
        .bind(&manifest.slug)
        .fetch_optional(db)
        .await
        .map_err(|e| crate::security::internal_error("manifest_event_lookup", e))?;
    if let Some((existing_id,)) = existing {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT external_key, title FROM prediction_items WHERE event_id=?1 ORDER BY external_key",
        )
        .bind(&existing_id)
        .fetch_all(db)
        .await
        .map_err(|e| crate::security::internal_error("manifest_existing_items", e))?;
        let same = rows.len() == manifest.items.len()
            && rows.iter().all(|(key, title)| {
                manifest
                    .items
                    .iter()
                    .any(|item| key == &item.external_key && title == &item.title)
            });
        if same {
            return Ok((manifest.items.len(), count));
        }
        return Err(crate::security::public_error(
            "Evento existente diverge do manifesto; importação não altera estrutura usada.",
        ));
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("manifest_begin", e))?;
    let event_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,starts_at,ends_at) VALUES(?1,?2,?3,'custom','active',?4,?5)").bind(&event_id).bind(&manifest.name).bind(&manifest.slug).bind(&manifest.starts_at).bind(&manifest.ends_at).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_event",e))?;
    for (order, item) in manifest.items.iter().enumerate() {
        let item_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO prediction_items(id,event_id,external_key,kind,title,description,lock_at,reveal_at,sort_order,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'open')").bind(&item_id).bind(&event_id).bind(&item.external_key).bind(&item.kind).bind(&item.title).bind(&item.description).bind(&item.lock_at).bind(&item.reveal_at).bind(order as i64).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_item",e))?;
        if item.kind == "single_choice" {
            sqlx::query("INSERT INTO custom_questions(item_id,points) VALUES(?1,1)")
                .bind(&item_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| crate::security::internal_error("manifest_question", e))?;
            for (sort, o) in item.options.iter().enumerate() {
                sqlx::query("INSERT INTO custom_question_options(id,item_id,external_key,label,sort_order) VALUES(?1,?2,?3,?4,?5)").bind(uuid::Uuid::new_v4().to_string()).bind(&item_id).bind(&o.external_key).bind(&o.label).bind(sort as i64).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_option",e))?;
            }
        } else if item.kind == "multiple_choice" {
            sqlx::query("INSERT INTO multiple_choice_questions(item_id,min_selections,max_selections) VALUES(?1,?2,?3)").bind(&item_id).bind(item.min_selections.unwrap_or(1)).bind(item.max_selections).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_multiple_choice",e))?;
            for (sort, o) in item.options.iter().enumerate() {
                sqlx::query("INSERT INTO custom_question_options(id,item_id,external_key,label,sort_order) VALUES(?1,?2,?3,?4,?5)").bind(uuid::Uuid::new_v4().to_string()).bind(&item_id).bind(&o.external_key).bind(&o.label).bind(sort as i64).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_multiple_option",e))?;
            }
        } else {
            let places = item.decimal_places.expect("validated") as u8;
            let min = item
                .min_value
                .as_ref()
                .map(|v| crate::numeric::parse_scaled(v, places))
                .transpose()
                .map_err(crate::security::public_error)?;
            let max = item
                .max_value
                .as_ref()
                .map(|v| crate::numeric::parse_scaled(v, places))
                .transpose()
                .map_err(crate::security::public_error)?;
            sqlx::query("INSERT INTO numeric_questions(item_id,decimal_places,unit_label,min_value_scaled,max_value_scaled) VALUES(?1,?2,?3,?4,?5)").bind(&item_id).bind(places).bind(&item.unit_label).bind(min).bind(max).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("manifest_numeric",e))?;
        }
    }
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("manifest_commit", e))?;
    Ok((manifest.items.len(), count))
}

#[cfg(test)]
mod tests {
    use super::{parse_and_validate, validate_event_window, validate_single_choice_timing};
    #[test]
    fn accepts_generic_single_choice_manifest() {
        let manifest = r#"{"name":"Premiação Teste","slug":"premiacao-teste","kind":"custom","startsAt":"2026-09-27T19:30:00-04:00","endsAt":"2026-09-27T21:30:00-04:00","items":[{"externalKey":"melhor-filme","kind":"single_choice","title":"Melhor Filme","description":null,"lockAt":"2026-09-27T19:30:00-04:00","revealAt":"2026-09-27T19:30:00-04:00","options":[{"externalKey":"a","label":"A"},{"externalKey":"b","label":"B"}]}]}"#;
        let parsed = parse_and_validate(manifest).unwrap();
        assert_eq!(parsed.items.len(), 1);
    }

    #[test]
    fn accepts_exact_numeric_manifest_and_rejects_options() {
        let valid = r#"{"name":"N","slug":"n","kind":"custom","items":[{"externalKey":"awards","kind":"numeric","title":"Quantos?","lockAt":"2026-09-27T19:30:00-04:00","revealAt":"2026-09-27T20:30:00-04:00","decimalPlaces":2,"minValue":"0","maxValue":"19.50","unitLabel":"prêmios"}]}"#;
        assert!(parse_and_validate(valid).is_ok());
        let invalid = valid.replace(
            "\"unitLabel\":\"prêmios\"",
            "\"options\":[{\"externalKey\":\"x\",\"label\":\"x\"}]",
        );
        assert!(parse_and_validate(&invalid).is_err());
    }
    #[test]
    fn rejects_duplicate_option_keys_before_database_write() {
        let manifest = r#"{"name":"X","slug":"x","kind":"custom","items":[{"externalKey":"a","kind":"single_choice","title":"A","lockAt":"2026-09-27T19:30:00-04:00","revealAt":"2026-09-27T19:30:00-04:00","options":[{"externalKey":"same","label":"A"},{"externalKey":"same","label":"B"}]}]}"#;
        assert!(parse_and_validate(manifest).is_err());
    }
    #[test]
    fn shared_timing_invariants_reject_reversed_windows() {
        assert!(validate_event_window(
            &Some("2026-10-02T00:00:00Z".into()),
            &Some("2026-10-01T00:00:00Z".into()),
        )
        .is_err());
        assert!(validate_single_choice_timing(
            "Categoria",
            "builder-key",
            "2026-10-02T00:00:00Z",
            "2026-10-01T00:00:00Z",
        )
        .is_err());
    }
}
