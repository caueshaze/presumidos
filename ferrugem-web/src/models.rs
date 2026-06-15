use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserPublic {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
    pub blocked_at: Option<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthResult {
    pub user: UserPublic,
    pub token: String,
    pub csrf_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub user: Option<UserPublic>,
    pub csrf_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PoolSummary {
    pub id: String,
    pub name: String,
    pub invite_code: String,
    pub member_count: i64,
    /// Id do usuário que criou o bolão (organizador).
    pub created_by: String,
    pub description: String,
    pub visible_rules: String,
    pub join_closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchRecord {
    pub id: String,
    pub home_team: String,
    pub away_team: String,
    pub kickoff: String,
    pub group_name: Option<String>,
    pub phase: Option<String>,
    pub home_score: Option<i64>,
    pub away_score: Option<i64>,
    /// 'home' ou 'away' — quem se classificou (apenas mata-mata).
    pub qualifier: Option<String>,
    pub went_to_penalties: bool,
    pub penalty_home_score: Option<i64>,
    pub penalty_away_score: Option<i64>,
    /// Rótulo oficial de "jogo finalizado". Não afeta a pontuação (o placar já
    /// conta quando preenchido); é só o indicador de partida encerrada.
    pub finished: bool,
    /// Placar ao vivo (parcial), preenchido pelo poller da API-Football enquanto
    /// o jogo acontece. Apenas exibição — não conta no ranking.
    pub live_home_score: Option<i64>,
    pub live_away_score: Option<i64>,
    /// Status curto da API (ex.: "1H", "HT", "2H", "ET", "P") quando ao vivo.
    pub live_status: Option<String>,
    /// Minuto corrido do jogo, quando disponível.
    pub live_elapsed: Option<i64>,
    pub result_source: Option<String>,
    pub result_synced_at: Option<String>,
    pub result_external_raw_status: Option<String>,
    pub live_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionRecord {
    pub match_id: String,
    pub home_score: i64,
    pub away_score: i64,
    /// 'home' ou 'away' — palpite de quem se classifica (apenas mata-mata).
    pub qualifier: Option<String>,
    pub went_to_penalties: bool,
    pub penalty_home_score: Option<i64>,
    pub penalty_away_score: Option<i64>,
}

/// Campos de mata-mata de um palpite ou resultado oficial, transportados juntos.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct KnockoutEntry {
    /// 'home' ou 'away' — quem se classifica.
    pub qualifier: Option<String>,
    pub went_to_penalties: bool,
    pub penalty_home: Option<i64>,
    pub penalty_away: Option<i64>,
}

/// Decide se uma fase é de mata-mata (tudo que não é "fase de grupos").
/// Normaliza o texto para tolerar variações de origem do dado.
pub fn is_knockout(phase: Option<&str>) -> bool {
    match phase {
        None => false,
        Some(p) => {
            let p = p.trim().to_lowercase();
            !(p.starts_with("fase de grupos") || p == "group" || p == "group stage")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub points: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminOverview {
    pub scheduled_matches: i64,
    pub live_matches: i64,
    pub finalized_matches: i64,
    pub manually_corrected_matches: i64,
    pub overdue_matches: i64,
    pub api_conflicts: i64,
    pub users_without_predictions_soon: i64,
    pub pool_count: i64,
    pub user_count: i64,
    pub blocked_user_count: i64,
    pub last_sync: Option<SyncStatus>,
    pub sync_enabled: bool,
    pub activity_feed: Vec<AdminActivityItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminActivityItem {
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
pub struct SyncStatus {
    pub id: String,
    pub status: String,
    pub trigger_source: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub summary_json: String,
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
    pub auto_sync_enabled: bool,
    pub sync_interval_minutes: i64,
    pub prediction_lock_minutes: i64,
    pub global_banner_enabled: bool,
    pub global_banner_text: String,
}

/// Ajuste manual de pontos aplicado a um membro de um bolão.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointAdjustment {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub delta: i64,
    pub reason: String,
    pub created_at: String,
}

/// Um membro do bolão com os palpites já visíveis (apenas de partidas que já
/// começaram). Membros sem palpite visível têm `predictions` vazio.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemberPredictions {
    pub user_id: String,
    pub username: String,
    pub predictions: Vec<PredictionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreference {
    pub enabled: bool,
    pub lead_time_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebPushSubscriptionKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebPushSubscriptionInput {
    pub endpoint: String,
    pub expiration_time: Option<i64>,
    pub keys: WebPushSubscriptionKeys,
    pub user_agent: Option<String>,
    pub device_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub web_push_enabled: bool,
    pub vapid_public_key: Option<String>,
    pub preference: NotificationPreference,
    pub active_subscription_count: i64,
}
