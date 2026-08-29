use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct RegisterBody {
    pub(crate) username: String,
    pub(crate) email: String,
    pub(crate) password: String,
}
#[derive(Deserialize)]
pub(crate) struct RegisterConfirmBody {
    pub(crate) email: String,
    pub(crate) code: String,
}
#[derive(Deserialize)]
pub(crate) struct PasswordResetBody {
    pub(crate) email: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PasswordResetConfirmBody {
    pub(crate) email: String,
    pub(crate) code: String,
    pub(crate) new_password: String,
}
#[derive(Deserialize)]
pub(crate) struct LoginBody {
    pub(crate) username: String,
    pub(crate) password: String,
}
#[derive(Deserialize)]
pub(crate) struct ChangeUsernameBody {
    pub(crate) username: String,
}
#[derive(Deserialize)]
pub(crate) struct ReauthBody {
    pub(crate) password: String,
}
