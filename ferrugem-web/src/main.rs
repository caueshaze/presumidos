//! Servidor Presumidos: API HTTP/JSON (Axum) + arquivos estáticos da SPA React.
//!
//! A lógica de negócio (auth, pools, matches, scoring) é exposta em [crate::api] sob `/api`.
//! Qualquer outra rota serve o build da SPA (`index.html` como fallback de client-side routing).

mod api;
mod auth;
mod context;
mod error;
mod matches;
mod models;
mod pools;
mod scoring;

mod config;
mod db;
mod email;
mod security;

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

fn try_handle_server_command() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    let command = args.next()?;
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
        .route_service("/favicon.ico", get_service(ServeFile::new(format!("{dir}/favicon.ico"))))
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
