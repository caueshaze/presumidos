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
