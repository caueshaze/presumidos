use crate::models::KnockoutEntry;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchResultBody {
    pub(crate) home_score: i64,
    pub(crate) away_score: i64,
    #[serde(default)]
    pub(crate) knockout: KnockoutEntry,
}
#[derive(Deserialize)]
pub(crate) struct KnockoutReleasedBody {
    pub(crate) released: bool,
}
#[derive(Deserialize)]
pub(crate) struct MatchFinishedBody {
    pub(crate) finished: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateTeamsBody {
    pub(crate) home_team: String,
    pub(crate) away_team: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchScheduleBody {
    pub(crate) home_team: String,
    pub(crate) away_team: String,
    pub(crate) phase: String,
    pub(crate) kickoff: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoolMemberBody {
    pub(crate) user_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoolReportBody {
    pub(crate) category: String,
    pub(crate) details: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoolReportStatusBody {
    pub(crate) status: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdjustmentBody {
    pub(crate) user_id: String,
    pub(crate) delta: i64,
    #[serde(default)]
    pub(crate) reason: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoveAdjustmentBody {
    pub(crate) adjustment_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdminMatchListQuery {
    pub(crate) phase: Option<String>,
    pub(crate) group_name: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) origin: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdminPredictionsQuery {
    pub(crate) match_id: Option<String>,
    pub(crate) user_id: Option<String>,
    pub(crate) pool_id: Option<String>,
    pub(crate) missing_only: Option<bool>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdminAuditQuery {
    pub(crate) action: Option<String>,
    pub(crate) actor_user_id: Option<String>,
    pub(crate) target_type: Option<String>,
    pub(crate) target_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoolReportQuery {
    pub(crate) status: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReopenPredictionBody {
    pub(crate) match_id: String,
    pub(crate) user_id: String,
    pub(crate) reason: String,
    pub(crate) expires_at: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RevokePredictionOverrideBody {
    pub(crate) override_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecalculateMatchBody {
    pub(crate) match_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlockUserBody {
    pub(crate) reason: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdminPushBody {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) url: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoolIdQuery {
    pub(crate) pool_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationPreferenceBody {
    pub(crate) enabled: bool,
    pub(crate) lead_time_minutes: i64,
    pub(crate) reaction_enabled: bool,
}
#[derive(Deserialize)]
pub(crate) struct SubscriptionRemoveBody {
    pub(crate) endpoint: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PredictionReactionBody {
    pub(crate) target_user_id: String,
    pub(crate) prediction_id: Option<String>,
    pub(crate) match_id: Option<String>,
    pub(crate) emoji: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptionMediaProgressBody {
    pub(crate) pool_id: String,
    pub(crate) option_id: String,
    pub(crate) seen: bool,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContactInfoResponse {
    pub(crate) email: String,
}
