use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PoolTieBreakMode {
    Inherit,
    Custom,
    Disabled,
}

impl PoolTieBreakMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Custom => "custom",
            Self::Disabled => "disabled",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "inherit" => Some(Self::Inherit),
            "custom" => Some(Self::Custom),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TieBreakPriority {
    pub item_id: String,
    pub title: String,
    pub kind: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolTieBreakConfig {
    pub mode: PoolTieBreakMode,
    pub effective_priorities: Vec<TieBreakPriority>,
    pub custom_priorities: Vec<TieBreakPriority>,
    pub can_edit: bool,
}
