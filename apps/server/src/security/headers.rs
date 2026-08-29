use std::net::IpAddr;

use axum::http::HeaderMap;

use crate::{
    config::settings,
    error::ServerFnError,
    security::{log_event, public_error, set_response_header},
};

#[cfg(feature = "server")]
fn parse_ip_token(raw: &str) -> Option<IpAddr> {
    let trimmed = raw
        .trim()
        .trim_matches('"')
        .trim_matches('[')
        .trim_matches(']');
    let candidate = trimmed
        .split(':')
        .next()
        .filter(|_| trimmed.matches(':').count() < 2)
        .unwrap_or(trimmed);

    candidate
        .parse::<IpAddr>()
        .ok()
        .or_else(|| trimmed.parse::<IpAddr>().ok())
}

#[cfg(feature = "server")]
fn parse_forwarded_for_ip(raw: &str) -> Option<IpAddr> {
    raw.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix("for=")
            .and_then(parse_ip_token)
            .or_else(|| part.strip_prefix("For=").and_then(parse_ip_token))
    })
}

#[cfg(feature = "server")]
fn header_ip(headers: &HeaderMap, key: &str) -> Option<IpAddr> {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_token)
}

#[cfg(feature = "server")]
fn forwarded_chain(headers: &HeaderMap) -> Vec<IpAddr> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        let parsed: Vec<IpAddr> = value.split(',').filter_map(parse_ip_token).collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .filter_map(parse_forwarded_for_ip)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(feature = "server")]
fn is_trusted_proxy(ip: IpAddr, trusted_proxy_cidrs: &[ipnet::IpNet]) -> bool {
    trusted_proxy_cidrs.iter().any(|cidr| cidr.contains(&ip))
}

#[cfg(feature = "server")]
fn resolve_client_ip_from_peer_and_headers(
    peer_ip: Option<IpAddr>,
    headers: &HeaderMap,
    trusted_proxy_cidrs: &[ipnet::IpNet],
) -> Option<IpAddr> {
    let peer_ip = peer_ip?;
    if trusted_proxy_cidrs.is_empty() || !is_trusted_proxy(peer_ip, trusted_proxy_cidrs) {
        return Some(peer_ip);
    }

    let chain = forwarded_chain(headers);
    if !chain.is_empty() {
        for ip in chain.into_iter().rev() {
            if !is_trusted_proxy(ip, trusted_proxy_cidrs) {
                return Some(ip);
            }
        }
    }

    header_ip(headers, "cf-connecting-ip")
        .or_else(|| header_ip(headers, "x-real-ip"))
        .or(Some(peer_ip))
}

#[cfg(feature = "server")]
fn proxy_boundary_allowed(
    peer_ip: Option<IpAddr>,
    trusted_proxy_cidrs: &[ipnet::IpNet],
    require_trusted_proxy: bool,
) -> bool {
    if !require_trusted_proxy {
        return true;
    }

    peer_ip.is_some_and(|peer_ip| is_trusted_proxy(peer_ip, trusted_proxy_cidrs))
}

#[cfg(feature = "server")]
pub fn current_peer_ip() -> Option<IpAddr> {
    crate::context::peer_ip()
}

#[cfg(feature = "server")]
pub fn client_ip(headers: &HeaderMap) -> String {
    resolve_client_ip_from_peer_and_headers(
        current_peer_ip(),
        headers,
        &settings().trusted_proxy_cidrs,
    )
    .map(|ip| ip.to_string())
    .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(feature = "server")]
pub fn enforce_trusted_proxy(headers: &HeaderMap) -> Result<(), ServerFnError> {
    let peer_ip = current_peer_ip();
    if proxy_boundary_allowed(
        peer_ip,
        &settings().trusted_proxy_cidrs,
        settings().require_trusted_proxy,
    ) {
        return Ok(());
    }

    let Some(peer_ip) = peer_ip else {
        log_event(
            "proxy_boundary_blocked",
            serde_json::json!({
                "reason": "missing_connect_info",
            }),
        );
        return Err(public_error(
            "Este ambiente exige acesso pelo proxy configurado.",
        ));
    };

    if !is_trusted_proxy(peer_ip, &settings().trusted_proxy_cidrs) {
        log_event(
            "proxy_boundary_blocked",
            serde_json::json!({
                "reason": "untrusted_peer",
                "peer_ip": peer_ip.to_string(),
                "client_ip": resolve_client_ip_from_peer_and_headers(
                    Some(peer_ip),
                    headers,
                    &settings().trusted_proxy_cidrs,
                )
                .map(|ip| ip.to_string()),
            }),
        );
        return Err(public_error(
            "Este ambiente exige acesso pelo proxy configurado.",
        ));
    }

    Ok(())
}

#[cfg(feature = "server")]
#[cfg(feature = "server")]
pub fn apply_security_headers() {
    set_response_header(
        "content-security-policy",
        // SPA React servida estaticamente: scripts e estilos próprios, fontes do Google,
        // e fetch da API no mesmo host. Sem 'unsafe-inline'/'wasm-unsafe-eval' (não há mais
        // SSR/WASM do Dioxus). 'style-src' mantém 'unsafe-inline' para estilos utilitários
        // injetados em runtime (Tailwind/shadcn) e variáveis de tema.
        // O hash em 'script-src' libera o único script inline (anti-FOUC de tema em
        // apps/web/index.html, executado antes do React montar). Se aquele script mudar, o
        // hash precisa ser recalculado (sha256 do conteúdo entre as tags <script>).
        "default-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob: https:; object-src 'none'; script-src 'self' 'sha256-sXw+kzZjEDOTCprbeOhrRSIW0La32ltxhXRk+DncIVU='; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".to_string(),
    );
    set_response_header(
        "referrer-policy",
        "strict-origin-when-cross-origin".to_string(),
    );
    set_response_header("x-content-type-options", "nosniff".to_string());
    set_response_header("x-frame-options", "DENY".to_string());
    set_response_header(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()".to_string(),
    );
    if settings().app_env == "production" && settings().cookie_secure {
        set_response_header(
            "strict-transport-security",
            "max-age=31536000; includeSubDomains".to_string(),
        );
    }
}
