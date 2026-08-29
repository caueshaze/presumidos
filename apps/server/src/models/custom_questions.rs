use super::{PredictionItemKind, PredictionItemStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomQuestionOption {
    pub id: String,
    pub label: String,
    pub sort_order: i64,
    pub image_url: Option<String>,
    pub image_asset_url: Option<String>,
    pub links: Vec<OptionLink>,
    pub media_seen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OptionLink {
    pub kind: String,
    pub label: String,
    pub url: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomQuestion {
    pub item_id: String,
    pub kind: PredictionItemKind,
    pub title: String,
    pub lock_at: String,
    pub reveal_at: String,
    pub sort_order: i64,
    pub status: PredictionItemStatus,
    pub current_option_id: Option<String>,
    pub correct_option_id: Option<String>,
    pub correct_points: i64,
    pub incorrect_points: i64,
    pub options: Vec<CustomQuestionOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_points: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within_tolerance_points: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_option_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_option_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_points: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventShowcase {
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub cover_asset_url: Option<String>,
    pub external_url: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub item_count: i64,
    pub answered_count: i64,
    pub is_historical: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipleChoiceScoreOutcome {
    Exact,
    Partial,
    Incorrect,
}

#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MultipleChoiceItemScoringConfig {
    pub pool_id: String,
    pub item_id: String,
    pub exact_points: i64,
    pub partial_points: i64,
    pub incorrect_points: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomPredictionValue {
    pub prediction_id: String,
    pub option_id: String,
}

#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NumericItemScoringConfig {
    pub pool_id: String,
    pub item_id: String,
    pub exact_points: i64,
    pub tolerance: String,
    pub within_tolerance_points: i64,
    pub incorrect_points: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomMemberPrediction {
    pub item_id: String,
    pub title: String,
    pub option_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomMemberPredictions {
    pub user_id: String,
    pub username: String,
    pub predictions: Vec<CustomMemberPrediction>,
}
