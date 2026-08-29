use axum::http::HeaderMap;

use crate::config::settings;

#[cfg(feature = "server")]
pub fn session_cookie_name() -> &'static str {
    "presumidos_session"
}

#[cfg(feature = "server")]
pub fn parse_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| {
            raw.split(';').find_map(|part| {
                let mut pieces = part.trim().splitn(2, '=');
                let key = pieces.next()?.trim();
                let value = pieces.next()?.trim();
                (key == name && !value.is_empty()).then(|| value.to_string())
            })
        })
}

#[cfg(feature = "server")]
pub fn set_response_header(name: &'static str, value: String) {
    crate::context::push_response_header(name, value);
}

#[cfg(feature = "server")]
pub fn current_headers() -> HeaderMap {
    crate::context::request_headers()
}

#[cfg(feature = "server")]
pub fn set_session_cookie(token: &str) {
    let max_age = settings().session_ttl_hours * 60 * 60;
    let secure = if settings().cookie_secure {
        "; Secure"
    } else {
        ""
    };
    set_response_header(
        "Set-Cookie",
        format!(
            "{}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}",
            session_cookie_name()
        ),
    );
}

#[cfg(feature = "server")]
pub fn clear_session_cookie() {
    let secure = if settings().cookie_secure {
        "; Secure"
    } else {
        ""
    };
    set_response_header(
        "Set-Cookie",
        format!(
            "{}=deleted; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
            session_cookie_name(),
            secure,
        ),
    );
}
