use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionRecord {
    pub item_id: String,
    pub match_id: String,
    pub home_score: i64,
    pub away_score: i64,
    /// 'home' ou 'away' — palpite de quem se classifica (apenas mata-mata).
    pub qualifier: Option<String>,
    pub went_to_penalties: bool,
    pub penalty_home_score: Option<i64>,
    pub penalty_away_score: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionReactionGroup {
    pub emoji: String,
    pub count: i64,
    pub reacted_by_viewer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PoolPredictionRecord {
    pub match_id: String,
    pub home_score: i64,
    pub away_score: i64,
    pub qualifier: Option<String>,
    pub went_to_penalties: bool,
    pub penalty_home_score: Option<i64>,
    pub penalty_away_score: Option<i64>,
    pub reactions: Vec<PredictionReactionGroup>,
    pub viewer_reaction: Option<String>,
    pub unread_reaction_count: i64,
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
