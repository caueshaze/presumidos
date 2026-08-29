use super::*;

#[tokio::test]
async fn contact_endpoint_returns_runtime_configured_email() {
    let base = test_server().await;
    let client = client();

    let response = client
        .get(format!("{base}/api/contact"))
        .send()
        .await
        .expect("contact request");

    assert!(response.status().is_success());
    let payload: serde_json::Value = response.json().await.expect("contact json");
    let expected = crate::config::settings()
        .contact_email
        .clone()
        .unwrap_or_default();
    assert_eq!(payload["email"], expected);
}
