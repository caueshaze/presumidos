use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;
pub const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_ITEMS: usize = 100;
pub const MAX_OPTIONS_PER_ITEM: usize = 100;
pub const MAX_LINKS_PER_OPTION: usize = 20;

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetReference {
    pub kind: String,
    pub sha256: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub description: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub cover_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_asset: Option<AssetReference>,
    pub external_url: Option<String>,
    pub items: Vec<CustomEventManifestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifestOption {
    pub external_key: String,
    pub label: String,
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_asset: Option<AssetReference>,
    #[serde(default)]
    pub links: Vec<CustomEventManifestOptionLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventManifestOptionLink {
    pub kind: String,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestDiffEntry {
    pub category: String,
    pub path: String,
    pub change: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportAction {
    Create,
    NoChange,
    SafeUpdate,
    Conflict,
    Rejected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestPreview {
    pub action: ImportAction,
    pub name: String,
    pub slug: String,
    pub schema_version: u32,
    pub item_count: usize,
    pub option_count: usize,
    pub link_count: usize,
    pub manifest_fingerprint: String,
    pub base_fingerprint: String,
    pub safe_changes: Vec<ManifestDiffEntry>,
    pub blocked_changes: Vec<ManifestDiffEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestApplyResult {
    pub action: ImportAction,
    pub event_id: Option<String>,
    pub item_count: usize,
    pub option_count: usize,
    pub link_count: usize,
    pub version_id: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedPlan {
    pub(crate) preview: ManifestPreview,
}
