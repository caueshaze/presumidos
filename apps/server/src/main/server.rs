//! Construção e execução do servidor HTTP.
use crate::{api, db, invite_page, operability, push, security, shutdown, startup};
/// Diretório dos arquivos estáticos da SPA. Em produção (Docker) é `/app/public`;
/// em desenvolvimento, normalmente `apps/web/dist`. Configurável via `STATIC_DIR`.
pub(crate) fn static_dir() -> String {
    std::env::var("STATIC_DIR").unwrap_or_else(|_| "public".to_string())
}

pub(crate) fn bind_address() -> std::net::SocketAddr {
    crate::config::settings()
        .listen_address
        .parse()
        .expect("LISTEN_ADDRESS inválido para bind do servidor")
}

pub async fn serve_application() {
    use axum::extract::DefaultBodyLimit;
    use axum::response::Html;
    use axum::routing::{get, get_service};
    use axum::Router;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tower_http::services::{ServeDir, ServeFile};

    let removed_staging = operability::cleanup_known_staging();
    if removed_staging > 0 {
        security::log_event(
            "startup_staging_cleanup",
            serde_json::json!({ "entries_removed": removed_staging }),
        );
    }
    db::init().await;
    let readiness = operability::readiness_report().await;
    if readiness.state != operability::ReadinessState::Ready {
        panic!("readiness inicial falhou; dependência operacional indisponível");
    }
    let migration_status = db::migration_status().await.unwrap_or((0, 0));
    security::log_event(
        "startup",
        serde_json::json!({
            "application_version": env!("CARGO_PKG_VERSION"),
            "environment": crate::config::settings().app_env,
            "database_backend": "sqlite",
            "database_path_configured": true,
            "asset_store": "filesystem_hash_addressed",
            "backup_configured": !crate::config::settings().backup_dir.is_empty(),
            "public_base_url_configured": crate::config::settings().public_base_url.is_some(),
            "migration_applied": migration_status.0,
            "migration_expected": migration_status.1,
            "listen_address": crate::config::settings().listen_address,
        }),
    );
    if let Err(error) = startup::run_housekeeping().await {
        security::log_event(
            "startup_housekeeping_failed",
            serde_json::json!({
                "error": error.to_string(),
            }),
        );
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
    let spa_index_html = index_html.clone();
    let spa_fallback = move || {
        let index_html = spa_index_html.clone();
        async move { Html(index_html.to_string()) }
    };
    let invite_page = {
        let index_html = index_html.clone();
        move |axum::extract::Path(token): axum::extract::Path<String>| {
            let index_html = index_html.clone();
            async move { invite_page::render(token, index_html).await }
        }
    };

    let app = Router::new()
        .nest("/api", api::router())
        .route("/pools/join/{token}", get(invite_page))
        .route("/health/live", axum::routing::get(api::health_live))
        .route("/health/ready", axum::routing::get(api::health_ready))
        .route(
            "/internal/metrics",
            axum::routing::get(api::internal_metrics),
        )
        .route(
            "/media/assets/{asset_id}/{variant}",
            axum::routing::get(api::media_asset),
        )
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
        .layer(DefaultBodyLimit::max(
            crate::config::settings().max_body_bytes,
        ))
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
    .with_graceful_shutdown(shutdown::shutdown_signal())
    .await
    .expect("falha ao servir aplicacao");
}
