//! Renderização da página pública de convite com metadados seguros.

use axum::response::IntoResponse;
use std::sync::Arc;

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub async fn render(token: String, index_html: Arc<String>) -> axum::response::Response {
    let preview = crate::pools::public_invite_preview(token).await.ok();
    let (title, description, image) = match preview {
        Some(preview) if preview.join_status != "invalid" => {
            let pool = preview.pool_name.unwrap_or_else(|| "Bolão".to_string());
            let event = preview
                .event_name
                .unwrap_or_else(|| "Presumidos".to_string());
            (
                format!("{pool} — Presumidos"),
                preview
                    .event_description
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| format!("Entre no bolão do {event}")),
                preview
                    .cover_asset_url
                    .or(preview.cover_url)
                    .unwrap_or_else(|| "/android-chrome-512x512.png".to_string()),
            )
        }
        _ => (
            "Convite de bolão — Presumidos".to_string(),
            "Confira este convite para participar de um bolão no Presumidos.".to_string(),
            "/android-chrome-512x512.png".to_string(),
        ),
    };
    let image = if image.starts_with('/') {
        crate::config::settings()
            .public_base_url
            .as_deref()
            .map(|base| format!("{}{}", base.trim_end_matches('/'), image))
            .unwrap_or(image)
    } else {
        image
    };
    let meta = format!("<meta property=\"og:title\" content=\"{}\"><meta property=\"og:description\" content=\"{}\"><meta property=\"og:type\" content=\"website\"><meta property=\"og:image\" content=\"{}\"><meta name=\"description\" content=\"{}\">", escape_html(&title), escape_html(&description), escape_html(&image), escape_html(&description));
    let mut html = index_html.replace("</head>", &format!("{meta}</head>"));
    let title_markup = format!("<title>{}</title>", escape_html(&title));
    if let Some(start) = html.find("<title>") {
        if let Some(end) = html[start..].find("</title>") {
            html.replace_range(start..start + end + "</title>".len(), &title_markup);
        }
    } else {
        html = html.replace("</head>", &format!("{title_markup}</head>"));
    }
    let mut response = axum::response::Html(html).into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    response
}
