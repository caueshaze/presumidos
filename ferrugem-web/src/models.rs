use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserPublic {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
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
