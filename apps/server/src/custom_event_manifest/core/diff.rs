use super::*;

fn push(v: &mut Vec<ManifestDiffEntry>, category: &str, path: String, change: impl Into<String>) {
    v.push(ManifestDiffEntry {
        category: category.into(),
        path,
        change: change.into(),
    });
}

pub(crate) fn safe_diff(
    a: &CustomEventManifest,
    b: &CustomEventManifest,
) -> Vec<ManifestDiffEntry> {
    let mut v = Vec::new();
    if a.name != b.name {
        push(&mut v, "safe", "Event.name".into(), "alterado");
    }
    if a.description != b.description {
        push(
            &mut v,
            "safe",
            "Event.description".into(),
            change_kind(&a.description, &b.description),
        );
    }
    if a.cover_url != b.cover_url {
        push(
            &mut v,
            "safe",
            "Event.coverUrl".into(),
            change_kind(&a.cover_url, &b.cover_url),
        );
    }
    if a.cover_asset != b.cover_asset {
        push(
            &mut v,
            "safe",
            "Event.coverAsset".into(),
            change_kind(&a.cover_asset, &b.cover_asset),
        );
    }
    if a.external_url != b.external_url {
        push(
            &mut v,
            "safe",
            "Event.externalUrl".into(),
            change_kind(&a.external_url, &b.external_url),
        );
    }
    for item in &b.items {
        let Some(old_item) = a.items.iter().find(|i| i.external_key == item.external_key) else {
            continue;
        };
        for option in &item.options {
            let Some(old) = old_item
                .options
                .iter()
                .find(|o| o.external_key == option.external_key)
            else {
                continue;
            };
            let prefix = format!("Option '{}'", option.external_key);
            if old.image_url != option.image_url {
                push(
                    &mut v,
                    "safe",
                    format!("{prefix}.imageUrl"),
                    change_kind(&old.image_url, &option.image_url),
                );
            }
            if old.image_asset != option.image_asset {
                push(
                    &mut v,
                    "safe",
                    format!("{prefix}.imageAsset"),
                    change_kind(&old.image_asset, &option.image_asset),
                );
            }
            if old.links != option.links {
                let delta = option.links.len() as i64 - old.links.len() as i64;
                let text = if delta > 0 {
                    format!("+{delta}")
                } else if delta < 0 {
                    delta.to_string()
                } else {
                    "alterado".into()
                };
                push(&mut v, "safe", format!("{prefix}.links"), text);
            }
        }
    }
    v
}

pub(crate) fn structural_diff(
    a: &CustomEventManifest,
    b: &CustomEventManifest,
) -> Vec<ManifestDiffEntry> {
    let mut v = Vec::new();
    if a.slug != b.slug {
        push(&mut v, "blocked", "Event.slug".into(), "alterado");
    }
    if a.starts_at != b.starts_at {
        push(&mut v, "blocked", "Event.startsAt".into(), "alterado");
    }
    if a.ends_at != b.ends_at {
        push(&mut v, "blocked", "Event.endsAt".into(), "alterado");
    }
    if a.items.len() != b.items.len() {
        push(
            &mut v,
            "blocked",
            "Event.items".into(),
            "quantidade alterada",
        );
    }
    for (index, item) in b.items.iter().enumerate() {
        let Some(old) = a.items.iter().find(|i| i.external_key == item.external_key) else {
            push(
                &mut v,
                "blocked",
                format!("Item '{}'", item.external_key),
                "adicionado",
            );
            continue;
        };
        let path = format!("Item '{}'", item.external_key);
        if old.kind != item.kind {
            push(&mut v, "blocked", format!("{path}.kind"), "alterado");
        }
        if old.title != item.title {
            push(&mut v, "blocked", format!("{path}.title"), "alterado");
        }
        if old.description != item.description {
            push(&mut v, "blocked", format!("{path}.description"), "alterado");
        }
        if old.lock_at != item.lock_at {
            push(&mut v, "blocked", format!("{path}.lockAt"), "alterado");
        }
        if old.reveal_at != item.reveal_at {
            push(&mut v, "blocked", format!("{path}.revealAt"), "alterado");
        }
        if old.decimal_places != item.decimal_places
            || old.unit_label != item.unit_label
            || old.min_value != item.min_value
            || old.max_value != item.max_value
        {
            push(
                &mut v,
                "blocked",
                format!("{path}.numericConfig"),
                "alterado",
            );
        }
        if old.min_selections != item.min_selections || old.max_selections != item.max_selections {
            push(
                &mut v,
                "blocked",
                format!("{path}.selectionRules"),
                "alterado",
            );
        }
        if old.options.len() != item.options.len() {
            push(
                &mut v,
                "blocked",
                format!("{path}.options"),
                "quantidade alterada",
            );
        }
        for option in &item.options {
            if let Some(old_option) = old
                .options
                .iter()
                .find(|o| o.external_key == option.external_key)
            {
                if old_option.label != option.label {
                    push(
                        &mut v,
                        "blocked",
                        format!("{path}.Option '{}'.label", option.external_key),
                        "alterado",
                    );
                }
            } else {
                push(
                    &mut v,
                    "blocked",
                    format!("{path}.Option '{}'", option.external_key),
                    "adicionada",
                );
            }
        }
        if a.items.get(index).map(|i| &i.external_key) != Some(&item.external_key) {
            push(&mut v, "blocked", format!("{path}.order"), "alterada");
        }
    }
    for old_item in &a.items {
        if !b
            .items
            .iter()
            .any(|item| item.external_key == old_item.external_key)
        {
            push(
                &mut v,
                "blocked",
                format!("Item '{}'", old_item.external_key),
                "removido",
            );
        }
    }
    for old_item in &a.items {
        let Some(new_item) = b
            .items
            .iter()
            .find(|item| item.external_key == old_item.external_key)
        else {
            continue;
        };
        for old_option in &old_item.options {
            if !new_item
                .options
                .iter()
                .any(|option| option.external_key == old_option.external_key)
            {
                push(
                    &mut v,
                    "blocked",
                    format!(
                        "Item '{}'.Option '{}'",
                        old_item.external_key, old_option.external_key
                    ),
                    "removida",
                );
            }
        }
    }
    v
}
