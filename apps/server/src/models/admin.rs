use super::{EventKind, MatchRecord, PredictionRecord, UserPublic};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminOverview {
    pub scheduled_matches: i64,
    pub finalized_matches: i64,
    pub overdue_matches: i64,
    pub users_without_predictions_soon: i64,
    pub pool_count: i64,
    pub user_count: i64,
    pub blocked_user_count: i64,
    pub activity_feed: Vec<AdminActivityItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminActivityItem {
    /// Id do registro de auditoria — chave única e estável para a lista.
    pub id: String,
    pub action: String,
    pub label: String,
    pub at: String,
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminMatchRecord {
    #[serde(flatten)]
    pub match_record: MatchRecord,
    pub admin_status: String,
    pub last_audit_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminPredictionRow {
    pub user_id: String,
    pub username: String,
    pub pool_id: Option<String>,
    pub pool_name: Option<String>,
    pub match_id: String,
    pub home_team: String,
    pub away_team: String,
    pub kickoff: String,
    pub phase: Option<String>,
    pub prediction: Option<PredictionRecord>,
    pub locked: bool,
    pub missing: bool,
    pub override_info: Option<PredictionReopenOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionReopenOverride {
    pub id: String,
    pub match_id: String,
    pub user_id: String,
    pub reason: String,
    pub reopened_by: String,
    pub expires_at: String,
    pub used_at: Option<String>,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[serde(rename_all = "camelCase")]
pub struct PredictionScoreBreakdown {
    pub pool_id: String,
    pub pool_name: String,
    pub user_id: String,
    pub username: String,
    pub match_id: String,
    pub home_team: String,
    pub away_team: String,
    pub exact_score_points: i64,
    pub outcome_points: i64,
    pub goal_bonus_points: i64,
    pub qualifier_points: i64,
    pub penalties_points: i64,
    pub total_points: i64,
    pub eligible: bool,
    pub eligibility_reason: String,
    pub official_source: Option<String>,
    pub computed_at: String,
}

/// Pontos que o usuário logado fez em um jogo, colapsados entre bolões (os
/// componentes só dependem do palpite vs resultado, então são iguais em todos os
/// bolões; `eligible` = elegível em ao menos um bolão).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[serde(rename_all = "camelCase")]
pub struct MatchPointsSummary {
    pub match_id: String,
    pub exact_score_points: i64,
    pub outcome_points: i64,
    pub goal_bonus_points: i64,
    pub qualifier_points: i64,
    pub penalties_points: i64,
    pub total_points: i64,
    pub eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScoringJob {
    pub id: String,
    pub scope_type: String,
    pub scope_id: Option<String>,
    pub triggered_by: Option<String>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub summary_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserRecord {
    #[serde(flatten)]
    pub user: UserPublic,
    pub pool_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: String,
    pub actor_user_id: Option<String>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub ip_address: Option<String>,
    pub details_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminSettings {
    pub knockout_released: bool,
    pub prediction_lock_minutes: i64,
    pub global_banner_enabled: bool,
    pub global_banner_text: String,
    /// Edição visual global "A Grande Final: Espanha x Argentina".
    pub final_theme_enabled: bool,
    /// Tela de encerramento da Copa para participantes.
    pub closing_screen_enabled: bool,
    /// Pool explicitamente autorizado pelo admin para aparecer no destaque global.
    pub featured_pool_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_pool: Option<FeaturedPool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedPool {
    pub pool_id: String,
    pub pool_name: String,
    pub event_name: String,
    pub event_kind: EventKind,
    pub is_historical: bool,
    pub member_count: i64,
    pub can_join: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_code: Option<String>,
}
