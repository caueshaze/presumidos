use crate::models::KnockoutEntry;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JoinPoolBody {
    pub(crate) invite_code: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PredictionBody {
    pub(crate) pool_id: String,
    pub(crate) match_id: String,
    pub(crate) home_score: i64,
    pub(crate) away_score: i64,
    #[serde(default)]
    pub(crate) knockout: KnockoutEntry,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SingleChoicePredictionBody {
    pub(crate) pool_id: String,
    pub(crate) item_id: String,
    pub(crate) option_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NumericPredictionBody {
    pub(crate) pool_id: String,
    pub(crate) item_id: String,
    pub(crate) value: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultipleChoicePredictionBody {
    pub(crate) pool_id: String,
    pub(crate) item_id: String,
    pub(crate) option_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultipleChoiceResultBody {
    pub(crate) option_ids: Vec<String>,
    #[serde(default)]
    pub(crate) pool_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomQuestionsQuery {
    pub(crate) pool_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FootballScoringBody {
    pub(crate) exact_score_points: i64,
    pub(crate) correct_result_exact_side_points: i64,
    pub(crate) correct_result_points: i64,
    pub(crate) incorrect_result_points: i64,
    pub(crate) knockout_bonus_points: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomScoringBody {
    pub(crate) correct_points: i64,
    pub(crate) incorrect_points: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomResultBody {
    pub(crate) option_id: String,
    #[serde(default)]
    pub(crate) pool_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResultNotRepresentableBody {
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) pool_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NumericScoringBody {
    pub(crate) exact_points: i64,
    pub(crate) tolerance: String,
    pub(crate) within_tolerance_points: i64,
    pub(crate) incorrect_points: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultipleChoiceScoringBody {
    pub(crate) exact_points: i64,
    pub(crate) partial_points: i64,
    pub(crate) incorrect_points: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NumericResultBody {
    pub(crate) value: String,
    #[serde(default)]
    pub(crate) pool_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoolTieBreakBody {
    pub(crate) mode: crate::models::PoolTieBreakMode,
    #[serde(default)]
    pub(crate) item_ids: Vec<String>,
}
