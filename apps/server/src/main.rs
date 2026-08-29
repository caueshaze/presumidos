//! Servidor Presumidos: API HTTP/JSON (Axum) + arquivos estáticos da SPA React.
//!
//! A lógica de negócio (auth, pools, matches, scoring) é exposta em [crate::api] sob `/api`.
//! Qualquer outra rota serve o build da SPA (`index.html` como fallback de client-side routing).

mod admin;
mod api;
mod auth;
mod context;
mod error;
mod matches;
mod models;
mod pool_scoring;
mod pools;
mod prediction_access;
mod prediction_items;
mod prediction_reuse;
mod scoring;

mod assets;
#[path = "main/cli.rs"]
mod cli;
mod config;
mod custom_event_manifest;
mod custom_events;
mod custom_questions;
mod db;
mod email;
mod event_package;
mod events;
#[path = "main/invite_page.rs"]
mod invite_page;
mod multiple_choice;
mod numeric;
mod operability;
mod security;
#[path = "main/server.rs"]
mod server;
#[path = "main/shutdown.rs"]
mod shutdown;
#[path = "main/startup.rs"]
mod startup;

#[cfg(feature = "web-push")]
mod push;
#[cfg(not(feature = "web-push"))]
#[path = "push_stub.rs"]
mod push;

#[cfg(all(test, feature = "server"))]
mod http_tests;
#[cfg(test)]
mod line_limit_tests;

#[cfg(test)]
async fn render_invite_page(
    token: String,
    index_html: std::sync::Arc<String>,
) -> axum::response::Response {
    invite_page::render(token, index_html).await
}

fn main() {
    if let Some(exit_code) = cli::try_handle_server_command() {
        std::process::exit(exit_code);
    }

    if let Err(error) = config::check_config() {
        eprintln!("configuração inválida: {error}");
        std::process::exit(78);
    }
    let rt = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    rt.block_on(server::serve_application());
}
