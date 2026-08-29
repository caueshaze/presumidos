use super::*;

#[tokio::test]
async fn operational_health_endpoints_and_request_id_are_safe() {
    let base = test_server().await;
    let live = client()
        .get(format!("{base}/api/health/live"))
        .send()
        .await
        .expect("liveness request");
    assert_eq!(live.status(), reqwest::StatusCode::OK);
    let request_id = live
        .headers()
        .get("x-request-id")
        .expect("request id")
        .to_str()
        .expect("request id ascii")
        .to_string();
    assert_eq!(
        live.json::<serde_json::Value>().await.expect("live json")["status"],
        "ok"
    );
    assert!(!request_id.is_empty());

    let ready = client()
        .get(format!("{base}/api/health/ready"))
        .send()
        .await
        .expect("readiness request");
    assert_eq!(ready.status(), reqwest::StatusCode::OK);
    assert_eq!(
        ready.json::<serde_json::Value>().await.expect("ready json")["status"],
        "ok"
    );
}
