use super::PoolPredictionRecord;
use serde::{Deserialize, Serialize};

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
    pub unread_reaction_count: i64,
    pub predictions: Vec<PoolPredictionRecord>,
}
