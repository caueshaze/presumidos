use serde::{Deserialize, Serialize};

/// Dados leves do evento necessários para contextualizar um bolão.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub kind: EventKind,
    pub status: EventStatus,
    pub ends_at: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub cover_asset_url: Option<String>,
    pub external_url: Option<String>,
    /// Estado de apresentação calculado pelo lifecycle do Event. Não cria uma
    /// cópia histórica do Pool: os mesmos registros continuam sendo lidos.
    pub is_historical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Football,
    /// Reservado para a futura Fase de eventos customizados.
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventOrigin {
    System,
    User,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Draft,
    Active,
    Finished,
}

/// Entidade de domínio que define o conteúdo previsto, independente dos pools.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub kind: EventKind,
    pub origin: EventOrigin,
    pub status: EventStatus,
    pub created_by: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub cover_asset_id: Option<String>,
    pub cover_asset_url: Option<String>,
    pub external_url: Option<String>,
    pub pool_creation_enabled: bool,
    pub current_published_version_id: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Projeção administrativa do catálogo de Events. Os contadores são
/// observacionais e não fazem parte da definição portátil do Event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminEventRecord {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub kind: EventKind,
    pub origin: EventOrigin,
    pub status: EventStatus,
    pub created_by: Option<String>,
    pub created_by_username: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub cover_asset_url: Option<String>,
    pub external_url: Option<String>,
    pub pool_creation_enabled: bool,
    pub current_published_version_id: Option<String>,
    pub archived_at: Option<String>,
    pub working_version_id: Option<String>,
    pub current_version_number: Option<i64>,
    pub item_count: i64,
    pub option_count: i64,
    pub pool_count: i64,
}
