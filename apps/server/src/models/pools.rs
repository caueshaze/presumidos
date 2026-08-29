use super::EventSummary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PoolSummary {
    pub id: String,
    /// Evento sobre o qual este bolão compete. A UI ainda não o seleciona.
    pub event_id: String,
    pub event: EventSummary,
    pub name: String,
    pub invite_code: String,
    pub member_count: i64,
    /// Id do usuário que criou o bolão (organizador).
    pub created_by: String,
    pub description: String,
    pub visible_rules: String,
    pub join_closed_at: Option<String>,
    pub predictions_closed_at: Option<String>,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PoolReport {
    pub id: String,
    pub pool_id: String,
    pub pool_name: String,
    pub invite_code: String,
    pub reporter_user_id: Option<String>,
    pub reporter_username: Option<String>,
    pub category: String,
    pub details: String,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_by_username: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Projeção mínima e pública de um convite. Nunca serializa o Pool completo,
/// predictions, ranking, emails ou metadados de membership.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicPoolInvitePreview {
    pub pool_name: Option<String>,
    pub event_name: Option<String>,
    pub event_description: Option<String>,
    pub cover_asset_url: Option<String>,
    pub cover_url: Option<String>,
    pub creator_display_name: Option<String>,
    pub member_count: Option<i64>,
    pub lock_deadline: Option<String>,
    pub join_status: String,
    pub predictions_closed_at: Option<String>,
    pub closed_at: Option<String>,
    /// Só é preenchido para um usuário autenticado que já é membro. Assim o
    /// preview anônimo não revela identificadores internos do Pool.
    pub pool_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PoolLifecycleState {
    pub predictions_closed_at: Option<String>,
    pub closed_at: Option<String>,
}

/// Leitura compacta da página inicial. Mantém o Pool como fonte de verdade e
/// agrega somente contadores pessoais que evitam uma requisição por card.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PoolDashboardSummary {
    pub pool: PoolSummary,
    pub answered_count: i64,
    pub item_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionReuseSource {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionReuseSuggestion {
    pub available: bool,
    pub source_pool: Option<PredictionReuseSource>,
    pub answered: i64,
    pub copyable: i64,
    pub total: i64,
    pub locked: i64,
}
impl PredictionReuseSuggestion {
    pub fn unavailable() -> Self {
        Self {
            available: false,
            source_pool: None,
            answered: 0,
            copyable: 0,
            total: 0,
            locked: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionReuseResult {
    pub copied_count: i64,
    pub already_initialized: bool,
}
