use super::*;

pub(super) fn json_request(
    token: &str,
    csrf: &str,
    path: String,
    payload: serde_json::Value,
) -> axum::http::Request<axum::body::Body> {
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::Request;
    use std::net::SocketAddr;

    Request::builder()
        .method("POST")
        .uri(path)
        .header("Content-Type", "application/json")
        .header(
            "Cookie",
            format!("{}={token}", crate::security::session_cookie_name()),
        )
        .header("X-CSRF-Token", csrf)
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
        .body(Body::from(payload.to_string()))
        .expect("request JSON builder")
}

pub(super) async fn response_json(response: axum::response::Response) -> serde_json::Value {
    use axum::body::to_bytes;

    let status = response.status();
    let body = to_bytes(response.into_body(), 2_000_000)
        .await
        .expect("body JSON builder");
    assert!(
        status.is_success(),
        "resposta builder: {status} {}",
        String::from_utf8_lossy(&body)
    );
    serde_json::from_slice::<serde_json::Value>(&body).expect("JSON builder")
}

pub(super) async fn create_builder_event(
    app: axum::Router,
    token: &str,
    csrf: &str,
) -> (String, String, String, String) {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::Request;
    use std::net::SocketAddr;
    use tower::ServiceExt;

    let json_request = |path: String, payload: serde_json::Value| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/json")
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(payload.to_string()))
            .expect("request JSON builder")
    };
    let response_json = |response: axum::response::Response| async move {
        let status = response.status();
        let body = to_bytes(response.into_body(), 2_000_000)
            .await
            .expect("body JSON builder");
        assert!(
            status.is_success(),
            "resposta builder: {status} {}",
            String::from_utf8_lossy(&body)
        );
        serde_json::from_slice::<serde_json::Value>(&body).expect("JSON builder")
    };
    let created = response_json(
        app.clone()
            .oneshot(json_request(
                "/api/custom/events".into(),
                json!({"name":"Evento com imagens","startsAt":null,"endsAt":null}),
            ))
            .await
            .expect("criar event builder"),
    )
    .await;
    let event_id = created["id"]
        .as_str()
        .expect("id event builder")
        .to_string();
    let lock = "2099-01-01T00:00:00Z";
    let reveal = "2099-01-02T00:00:00Z";
    let add_item = |path: String, payload: serde_json::Value| async {
        response_json(
            app.clone()
                .oneshot(json_request(path, payload))
                .await
                .expect("add item builder"),
        )
        .await["id"]
            .as_str()
            .expect("id item builder")
            .to_string()
    };
    let single_id = add_item(
        format!("/api/custom/events/{event_id}/items"),
        json!({"title":"Escolha","lockAt":lock,"revealAt":reveal}),
    )
    .await;
    let numeric_id = add_item(
        format!("/api/custom/events/{event_id}/items/numeric"),
        json!({"title":"Número","lockAt":lock,"revealAt":reveal,"decimalPlaces":1,"unitLabel":"pontos","minValue":"0.0","maxValue":"10.0"}),
    )
    .await;
    let multiple_id = add_item(
        format!("/api/custom/events/{event_id}/items/multiple-choice"),
        json!({"title":"Múltipla","lockAt":lock,"revealAt":reveal,"minSelections":1,"maxSelections":2}),
    )
    .await;
    let add_option = |item_id: String, label: String| {
        let app = app.clone();
        let request = json_request(
            format!("/api/custom/events/{event_id}/items/{item_id}/options"),
            json!({"label": label}),
        );
        async move {
            let response = app.oneshot(request).await.expect("add option builder");
            let status = response.status();
            let body = to_bytes(response.into_body(), 2_000_000)
                .await
                .expect("option body builder");
            assert!(
                status.is_success(),
                "resposta option builder: {status} {}",
                String::from_utf8_lossy(&body)
            );
            let value: serde_json::Value =
                serde_json::from_slice(&body).expect("option JSON builder");
            value["id"].as_str().expect("id option builder").to_string()
        }
    };
    let single_option = add_option(single_id.clone(), "A".into()).await;
    add_option(single_id.clone(), "B".into()).await;
    add_option(multiple_id.clone(), "X".into()).await;
    add_option(multiple_id, "Y".into()).await;
    (event_id, single_id, single_option, numeric_id)
}
