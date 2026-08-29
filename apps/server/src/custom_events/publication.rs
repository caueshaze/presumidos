use crate::error::ServerFnError;

use super::core::owner;

pub async fn publish(token: String, event_id: String, csrf: String) -> Result<(), ServerFnError> {
    let (s, _db, version_id) = owner(token, &event_id, Some(csrf)).await?;
    crate::custom_event_manifest::publish_working_revision(&event_id, Some(&version_id), &s.user_id)
        .await
}
