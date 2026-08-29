use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PredictionItemKind {
    FootballMatch,
    /// Reservado para a fase de perguntas customizadas.
    SingleChoice,
    Numeric,
    MultipleChoice,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PredictionItemStatus {
    Draft,
    Open,
    Locked,
    Resolved,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionItem {
    pub id: String,
    pub event_id: String,
    pub kind: PredictionItemKind,
    pub title: String,
    pub description: Option<String>,
    pub lock_at: String,
    pub reveal_at: String,
    pub sort_order: i64,
    pub status: PredictionItemStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FootballScoringConfig {
    pub exact_score_points: i64,
    pub correct_result_exact_side_points: i64,
    pub correct_result_points: i64,
    pub incorrect_result_points: i64,
    pub knockout_bonus_points: i64,
}

#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomItemScoringConfig {
    pub pool_id: String,
    pub item_id: String,
    pub correct_points: i64,
    pub incorrect_points: i64,
}
