use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePoolBody {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) event_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateEventBody {
    pub(crate) name: String,
    pub(crate) starts_at: Option<String>,
    pub(crate) ends_at: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateItemBody {
    pub(crate) title: String,
    pub(crate) lock_at: String,
    pub(crate) reveal_at: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateNumericItemBody {
    pub(crate) title: String,
    pub(crate) lock_at: String,
    pub(crate) reveal_at: String,
    pub(crate) decimal_places: i64,
    pub(crate) unit_label: Option<String>,
    pub(crate) min_value: Option<String>,
    pub(crate) max_value: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateMultipleChoiceItemBody {
    pub(crate) title: String,
    pub(crate) lock_at: String,
    pub(crate) reveal_at: String,
    pub(crate) min_selections: i64,
    pub(crate) max_selections: Option<i64>,
}
#[derive(Deserialize)]
pub(crate) struct CreateOptionBody {
    pub(crate) label: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateOptionMediaBody {
    pub(crate) image_url: Option<String>,
    #[serde(default)]
    pub(crate) links: Vec<crate::custom_events::BuilderOptionLink>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateEventBody {
    pub(crate) name: String,
    pub(crate) starts_at: Option<String>,
    pub(crate) ends_at: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) cover_url: Option<String>,
    pub(crate) external_url: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateItemBody {
    pub(crate) title: String,
    pub(crate) lock_at: String,
    pub(crate) reveal_at: String,
}
#[derive(Deserialize)]
pub(crate) struct MoveBody {
    pub(crate) direction: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestPreviewBody {
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) filename: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestApplyBody {
    pub(crate) content: String,
    pub(crate) base_fingerprint: String,
    #[serde(default)]
    pub(crate) filename: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EventAvailabilityBody {
    pub(crate) enabled: bool,
}
