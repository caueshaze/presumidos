//! Servidor Presumidos: API HTTP/JSON (Axum) + arquivos estáticos da SPA React.
//!
//! A lógica de negócio (auth, pools, matches, scoring) é exposta em [crate::api] sob `/api`.
//! Qualquer outra rota serve o build da SPA (`index.html` como fallback de client-side routing).

mod api;
mod admin;
mod auth;
mod context;
mod error;
mod football;
mod matches;
mod models;
mod pools;
mod scoring;

mod config;
mod db;
mod email;
mod security;

#[cfg(feature = "web-push")]
mod push;
#[cfg(not(feature = "web-push"))]
#[path = "push_stub.rs"]
mod push;

#[cfg(all(test, feature = "server"))]
mod http_tests;

#[derive(Debug)]
struct BootstrapAdminArgs {
    username: String,
    email: String,
    password: String,
}

fn parse_bootstrap_admin_args<I>(mut args: I) -> Result<BootstrapAdminArgs, String>
where
    I: Iterator<Item = String>,
{
    let mut username = None;
    let mut email = None;

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--username" => username = args.next(),
            "--email" => email = args.next(),
            unknown => {
                return Err(format!(
                    "argumento desconhecido: {unknown}. Use --username e --email."
                ));
            }
        }
    }

    let password = if let Ok(value) = std::env::var("BOOTSTRAP_ADMIN_PASSWORD") {
        value
    } else {
        let first =
            rpassword::prompt_password("Senha do admin inicial: ").map_err(|e| e.to_string())?;
        let second = rpassword::prompt_password("Confirme a senha: ").map_err(|e| e.to_string())?;
        if first != second {
            return Err("as senhas digitadas nao conferem".to_string());
        }
        first
    };

    Ok(BootstrapAdminArgs {
        username: username
            .ok_or_else(|| "faltou --username para o bootstrap inicial".to_string())?,
        email: email.ok_or_else(|| "faltou --email para o bootstrap inicial".to_string())?,
        password,
    })
}

fn parse_sync_fixtures_args<I>(mut args: I) -> Result<football::SyncMode, String>
where
    I: Iterator<Item = String>,
{
    let mut mode: Option<football::SyncMode> = None;

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--dry-run" => mode = Some(football::SyncMode::DryRun),
            "--apply" => mode = Some(football::SyncMode::Apply),
            "--fixture" => {
                let pair = args
                    .next()
                    .ok_or_else(|| "--fixture exige jogo-XXX=ID".to_string())?;
                let (match_id, fixture_id) = pair.split_once('=').ok_or_else(|| {
                    format!("formato inválido para --fixture: {pair} (use jogo-XXX=ID)")
                })?;
                let fixture_id = fixture_id
                    .parse::<i64>()
                    .map_err(|_| format!("ID de fixture inválido: {fixture_id}"))?;
                mode = Some(football::SyncMode::Override {
                    match_id: match_id.to_string(),
                    fixture_id,
                });
            }
            unknown => {
                return Err(format!(
                    "argumento desconhecido: {unknown}. Use --dry-run, --apply ou --fixture jogo-XXX=ID."
                ));
            }
        }
    }

    // Sem flag explícita, o padrão é dry-run (não grava nada por acidente).
    Ok(mode.unwrap_or(football::SyncMode::DryRun))
}

fn run_sync_fixtures_command<I>(args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let mode = match parse_sync_fixtures_args(args) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("{error}");
            return 2;
        }
    };

    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    let result = runtime.block_on(async {
        db::init().await;
        football::sync_fixtures(mode).await
    });

    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("falha no sync-fixtures: {error:?}");
            1
        }
    }
}

async fn run_housekeeping() -> Result<(), error::ServerFnError> {
    let db = db::pool();
    let auth_summary = auth::cleanup_expired_auth_data(db).await?;
    let push_summary = push::cleanup_stale_push_data(db).await?;

    security::log_event(
        "startup_housekeeping_completed",
        serde_json::json!({
            "expired_sessions_deleted": auth_summary.expired_sessions_deleted,
            "expired_pending_registrations_deleted": auth_summary.expired_pending_registrations_deleted,
            "expired_password_reset_codes_deleted": auth_summary.expired_password_reset_codes_deleted,
            "inactive_push_subscriptions_deleted": push_summary.inactive_subscriptions_deleted,
            "old_push_deliveries_deleted": push_summary.old_deliveries_deleted,
        }),
    );

    Ok(())
}

fn run_cleanup_expired_command() -> i32 {
    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    let result = runtime.block_on(async {
        db::init().await;
        run_housekeeping().await
    });

    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("falha no cleanup-expired: {error}");
            1
        }
    }
}

fn try_handle_server_command() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    let command = args.next()?;
    if command == "sync-fixtures" {
        return Some(run_sync_fixtures_command(args));
    }
    if command == "cleanup-expired" {
        return Some(run_cleanup_expired_command());
    }
    if command != "bootstrap-admin" {
        return None;
    }

    let parsed = match parse_bootstrap_admin_args(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{error}");
            eprintln!(
                "uso: cargo run -p ferrugem-web --features server -- bootstrap-admin --username <usuario> --email <email>"
            );
            return Some(2);
        }
    };

    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    let result = runtime.block_on(async {
        db::init().await;
        auth::run_bootstrap_admin(
            parsed.username,
            parsed.email,
            parsed.password,
            crate::config::settings().admin_bootstrap_secret.clone(),
        )
        .await
    });

    match result {
        Ok(user) => {
            println!(
                "admin inicial criado com sucesso: {} <{}>",
                user.username, user.email
            );
            Some(0)
        }
        Err(error) => {
            eprintln!("falha no bootstrap do admin inicial: {error}");
            Some(1)
        }
    }
}

/// Diretório dos arquivos estáticos da SPA. Em produção (Docker) é `/app/public`;
/// em desenvolvimento, normalmente `web/dist`. Configurável via `STATIC_DIR`.
fn static_dir() -> String {
    std::env::var("STATIC_DIR").unwrap_or_else(|_| "public".to_string())
}

fn bind_address() -> std::net::SocketAddr {
    let ip = std::env::var("IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    format!("{ip}:{port}")
        .parse()
        .expect("IP/PORT inválidos para bind do servidor")
}

async fn serve_application() {
    use axum::response::Html;
    use axum::routing::get_service;
    use axum::Router;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tower_http::services::{ServeDir, ServeFile};

    db::init().await;
    if let Err(error) = run_housekeeping().await {
        security::log_event(
            "startup_housekeeping_failed",
            serde_json::json!({
                "error": error.to_string(),
            }),
        );
    }

    // Poller de resultados ao vivo (API-Football). Sobe apenas se a integração e
    // o poller estiverem habilitados — mantenha o poller ligado em uma única
    // instância para não duplicar o consumo de cota.
    let football = &crate::config::settings().football;
    if football.enabled && football.poller_enabled {
        football::spawn_poller();
    }
    if crate::config::settings().web_push.enabled {
        push::spawn_reminder_worker();
    }

    let dir = static_dir();
    // index.html é lido uma vez e devolvido (200) como fallback de client-side routing:
    // qualquer rota não-/api e não-asset (ex.: refresh em /dashboard) carrega a SPA.
    let index_html = Arc::new(
        std::fs::read_to_string(format!("{dir}/index.html")).unwrap_or_else(|_| {
            eprintln!("aviso: {dir}/index.html não encontrado — SPA não será servida");
            String::new()
        }),
    );
    let spa_fallback = move || {
        let index_html = index_html.clone();
        async move { Html(index_html.to_string()) }
    };

    let app = Router::new()
        .nest("/api", api::router())
        .nest_service("/assets", ServeDir::new(format!("{dir}/assets")))
        .route_service(
            "/favicon.ico",
            get_service(ServeFile::new(format!("{dir}/favicon.ico"))),
        )
        .route_service(
            "/favicon-16x16.png",
            get_service(ServeFile::new(format!("{dir}/favicon-16x16.png"))),
        )
        .route_service(
            "/favicon-32x32.png",
            get_service(ServeFile::new(format!("{dir}/favicon-32x32.png"))),
        )
        .route_service(
            "/apple-touch-icon.png",
            get_service(ServeFile::new(format!("{dir}/apple-touch-icon.png"))),
        )
        .route_service(
            "/android-chrome-192x192.png",
            get_service(ServeFile::new(format!("{dir}/android-chrome-192x192.png"))),
        )
        .route_service(
            "/android-chrome-512x512.png",
            get_service(ServeFile::new(format!("{dir}/android-chrome-512x512.png"))),
        )
        .route_service(
            "/site.webmanifest",
            get_service(ServeFile::new(format!("{dir}/site.webmanifest"))),
        )
        .route_service(
            "/sw.js",
            get_service(ServeFile::new(format!("{dir}/sw.js"))),
        )
        .fallback(spa_fallback)
        .layer(axum::middleware::from_fn(api::context_middleware));

    let addr = bind_address();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("falha ao abrir listener HTTP");

    eprintln!("Presumidos ouvindo em http://{addr} (estáticos em {dir}/)");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("falha ao servir aplicacao");
}

fn main() {
    if let Some(exit_code) = try_handle_server_command() {
        std::process::exit(exit_code);
    }

    let rt = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    rt.block_on(serve_application());
}
