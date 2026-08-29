use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    pub position: i64,
    pub user_id: String,
    pub username: String,
    pub points: i64,
    /// Critérios de desempate (não somam ajustes manuais; refletem só acertos).
    /// 1º desempate: número de placares exatos cravados.
    pub exact_scores: i64,
    /// 2º desempate: número de palpites com resultado correto (inclui exatos).
    pub correct_results: i64,
    /// 3º desempate: soma dos bônus de precisão (gols + classificado + pênaltis).
    pub bonus_points: i64,
}
