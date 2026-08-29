use super::*;
use crate::error::ServerFnError;
#[cfg(feature = "server")]
use sqlx::{sqlite::SqliteConnection, Sqlite, SqlitePool};

pub async fn apply_admin(
    bytes: &str,
    expected: &str,
    actor: &str,
) -> Result<ManifestApplyResult, ServerFnError> {
    let mut m = parse_and_validate(bytes).map_err(crate::security::public_error)?;
    m.schema_version = CURRENT_SCHEMA_VERSION;
    apply_manifest(&m, expected, Some(actor)).await
}

#[cfg(feature = "server")]
pub(crate) async fn apply_normalized(
    m: &CustomEventManifest,
    expected: &str,
    actor: &str,
) -> Result<ManifestApplyResult, ServerFnError> {
    apply_manifest(m, expected, Some(actor)).await
}

/// Compatibility wrapper used by the legacy CLI. Importing is deliberately
/// revision-only now; a separate publish operation is required.
#[cfg(feature = "server")]
pub async fn import(m: CustomEventManifest, apply: bool) -> Result<(usize, usize), ServerFnError> {
    let mut m = m;
    m.schema_version = CURRENT_SCHEMA_VERSION;
    let (i, o, _) = counts(&m);
    if !apply {
        return Ok((i, o));
    }
    let db = crate::db::pool();
    let actor: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE is_admin=1 ORDER BY created_at LIMIT 1")
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_actor", e))?;
    let expected = if let Some((id,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM events WHERE slug=?1")
            .bind(&m.slug)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("legacy_manifest_lookup", e))?
    {
        fingerprint(&load_manifest(db, &id).await?).map_err(crate::security::public_error)?
    } else {
        absent_fingerprint(&m.slug)
    };
    let _ = apply_manifest(&m, &expected, actor.as_ref().map(|value| value.0.as_str())).await?;
    Ok((i, o))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> &'static str {
        r#"{"schemaVersion":1,"name":"Premiação Teste","slug":"premiacao-teste","kind":"custom","items":[{"externalKey":"melhor-filme","kind":"single_choice","title":"Melhor Filme","lockAt":"2026-09-27T19:30:00-04:00","revealAt":"2026-09-27T19:30:00-04:00","options":[{"externalKey":"a","label":"A"},{"externalKey":"b","label":"B"}]}]}"#
    }
    #[test]
    fn accepts_legacy_and_versioned() {
        assert_eq!(parse_and_validate(sample()).unwrap().schema_version, 1);
        assert_eq!(
            parse_and_validate(&sample().replace("\"schemaVersion\":1,", ""))
                .unwrap()
                .schema_version,
            1
        );
    }
    #[test]
    fn rejects_unknown_version_and_duplicates() {
        assert_eq!(
            parse_and_validate(&sample().replace("\"schemaVersion\":1", "\"schemaVersion\":2"))
                .unwrap()
                .schema_version,
            2
        );
        assert!(parse_and_validate(
            &sample().replace("\"schemaVersion\":1", "\"schemaVersion\":99")
        )
        .is_err());
        assert!(parse_and_validate(
            &sample().replace("\"externalKey\":\"b\"", "\"externalKey\":\"a\"")
        )
        .is_err());
    }
    #[test]
    fn canonical_fingerprint_ignores_json_formatting() {
        let a = parse_and_validate(sample()).unwrap();
        let b = sample().replace(
            "\"name\":\"Premiação Teste\"",
            "\"name\": \" Premiação Teste \"",
        );
        let b = parse_and_validate(&b).unwrap();
        assert_eq!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
        let reordered = r#"{"items":[{"options":[{"label":"A","externalKey":"a"},{"label":"B","externalKey":"b"}],"revealAt":"2026-09-27T19:30:00-04:00","lockAt":"2026-09-27T19:30:00-04:00","title":"Melhor Filme","kind":"single_choice","externalKey":"melhor-filme"}],"kind":"custom","slug":"premiacao-teste","name":"Premiação Teste","schemaVersion":1}"#;
        assert_eq!(
            fingerprint(&a).unwrap(),
            fingerprint(&parse_and_validate(reordered).unwrap()).unwrap()
        );
        let canonical = canonical_json(&a).unwrap();
        assert!(canonical.ends_with('\n'));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&canonical).unwrap()["schemaVersion"],
            2
        );
    }

    #[test]
    fn asset_references_are_v2_only_and_part_of_canonical_fingerprint() {
        let hash = "a".repeat(64);
        let v1 = sample().replace(
            "\"label\":\"A\"",
            &format!("\"label\":\"A\",\"imageAsset\":{{\"kind\":\"asset\",\"sha256\":\"{hash}\",\"mediaType\":\"image/webp\"}}"),
        );
        assert!(parse_and_validate(&v1).is_err());
        let v2 = v1.replace("\"schemaVersion\":1", "\"schemaVersion\":2");
        let parsed = parse_and_validate(&v2).expect("asset ref v2");
        assert_eq!(
            parsed.items[0].options[0]
                .image_asset
                .as_ref()
                .unwrap()
                .sha256,
            hash
        );
        assert_eq!(
            canonical_json(&parsed)
                .unwrap()
                .matches("imageAsset")
                .count(),
            1
        );
    }
    #[test]
    fn rejects_reversed_windows() {
        assert!(validate_event_window(
            &Some("2026-10-02T00:00:00Z".into()),
            &Some("2026-10-01T00:00:00Z".into())
        )
        .is_err());
        assert!(validate_single_choice_timing(
            "Categoria",
            "key",
            "2026-10-02T00:00:00Z",
            "2026-10-01T00:00:00Z"
        )
        .is_err());
    }

    #[test]
    fn published_name_is_safe_but_option_label_is_structural() {
        let current = parse_and_validate(sample()).unwrap();
        let renamed =
            parse_and_validate(&sample().replace("Premiação Teste", "Premiação Teste 2026"))
                .unwrap();
        assert!(projection(&current) == projection(&renamed));
        assert!(safe_diff(&current, &renamed)
            .iter()
            .any(|change| change.path == "Event.name"));

        let changed_label =
            parse_and_validate(&sample().replace("\"label\":\"A\"", "\"label\":\"Outra opção\""))
                .unwrap();
        assert!(structural_diff(&current, &changed_label)
            .iter()
            .any(|change| change.path.contains("label")));

        let mut removed_option = current.clone();
        removed_option.items[0].options.pop();
        assert!(structural_diff(&current, &removed_option)
            .iter()
            .any(|change| change.path.contains("Option 'b'") && change.change == "removida"));
    }
}
