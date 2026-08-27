//! Servidor Presumidos: API HTTP/JSON (Axum) + arquivos estáticos da SPA React.
//!
//! A lógica de negócio (auth, pools, matches, scoring) é exposta em [crate::api] sob `/api`.
//! Qualquer outra rota serve o build da SPA (`index.html` como fallback de client-side routing).

mod admin;
mod api;
mod auth;
mod context;
mod error;
mod football;
mod matches;
mod models;
mod pool_scoring;
mod pools;
mod prediction_access;
mod prediction_items;
mod prediction_reuse;
mod scoring;

mod assets;
mod config;
mod custom_event_manifest;
mod custom_events;
mod custom_questions;
mod db;
mod email;
mod event_package;
mod events;
mod multiple_choice;
mod numeric;
mod operability;
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

fn run_import_custom_event_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let mut file = None;
    let mut apply = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" => file = args.next(),
            "--apply" => apply = true,
            "--dry-run" => apply = false,
            _ => {
                eprintln!("uso: import-custom-event --file <arquivo> [--dry-run|--apply]");
                return 2;
            }
        }
    }
    let Some(file) = file else {
        eprintln!("--file é obrigatório");
        return 2;
    };
    let bytes = match std::fs::read_to_string(&file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("falha ao ler manifesto: {e}");
            return 1;
        }
    };
    let manifest = match custom_event_manifest::parse_and_validate(&bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("manifesto inválido: {e}");
            return 2;
        }
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        db::init().await;
        custom_event_manifest::import(manifest, apply).await
    });
    match result {
        Ok((items, options)) => {
            println!(
                "{}: {items} itens, {options} opções",
                if apply { "importado" } else { "dry-run" }
            );
            0
        }
        Err(e) => {
            eprintln!("falha na importação: {e}");
            1
        }
    }
}

async fn run_housekeeping() -> Result<(), error::ServerFnError> {
    let db = db::pool();
    let auth_summary = auth::cleanup_expired_auth_data(db).await?;
    let push_summary = push::cleanup_stale_push_data(db).await?;
    let matches_force_finished = matches::force_finish_matches_for_ended_events().await?;

    security::log_event(
        "startup_housekeeping_completed",
        serde_json::json!({
            "expired_sessions_deleted": auth_summary.expired_sessions_deleted,
            "expired_pending_registrations_deleted": auth_summary.expired_pending_registrations_deleted,
            "expired_password_reset_codes_deleted": auth_summary.expired_password_reset_codes_deleted,
            "inactive_push_subscriptions_deleted": push_summary.inactive_subscriptions_deleted,
            "old_push_deliveries_deleted": push_summary.old_deliveries_deleted,
            "matches_force_finished": matches_force_finished,
        }),
    );

    Ok(())
}

fn run_backfill_results_command() -> i32 {
    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    let result = runtime.block_on(async {
        db::init().await;
        football::run_backfill().await
    });

    match result {
        Ok(summary) => {
            println!(
                "backfill concluído: {} jogo(s) elegível(is), {} finalizado(s), {} sugerido(s) (mata-mata, confirme no admin), {} ao vivo.",
                summary.candidates, summary.finalized, summary.suggested, summary.live
            );
            0
        }
        Err(error) => {
            eprintln!("falha no backfill-results: {error:?}");
            1
        }
    }
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

fn run_check_config_command() -> i32 {
    match config::check_config() {
        Ok(()) => {
            println!("configuração válida");
            0
        }
        Err(error) => {
            eprintln!("configuração inválida: {error}");
            78
        }
    }
}

fn run_migrate_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let mut check_only = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            _ => {
                eprintln!("uso: migrate [--check]");
                return 2;
            }
        }
    }
    if let Err(error) = config::check_config() {
        eprintln!("configuração inválida: {error}");
        return 78;
    }
    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    if check_only {
        return match runtime.block_on(db::migration_report()) {
            Ok(report) if !report.pending && !report.dirty && !report.checksum_mismatch => {
                println!("migrations em dia: {}/{}", report.applied, report.expected);
                0
            }
            Ok(report) => {
                eprintln!(
                    "migrations incompatíveis: aplicadas={}, esperadas={}, pendentes={}, dirty={}, checksum_mismatch={}",
                    report.applied, report.expected, report.pending, report.dirty, report.checksum_mismatch
                );
                1
            }
            Err(error) => {
                eprintln!("falha ao verificar migrations: {error}");
                1
            }
        };
    }
    match runtime.block_on(db::apply_migrations()) {
        Ok(()) => {
            println!("migrations aplicadas");
            0
        }
        Err(error) => {
            eprintln!("falha ao aplicar migrations: {error}");
            1
        }
    }
}

fn run_db_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some("check") if args.next().is_none() => {}
        _ => {
            eprintln!("uso: db check");
            return 2;
        }
    }
    if let Err(error) = config::check_config() {
        eprintln!("configuração inválida: {error}");
        return 78;
    }
    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    match runtime.block_on(db::integrity_check_without_migration()) {
        Ok(result) if result == "ok" => {
            println!("integrity_check: ok");
            0
        }
        Ok(result) => {
            eprintln!("integrity_check: {result}");
            1
        }
        Err(error) => {
            eprintln!("falha no db check: {error}");
            1
        }
    }
}

fn run_backup_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let Some(action) = args.next() else {
        eprintln!("uso: backup create --output <diretório> | backup verify <diretório> | backup restore ...");
        return 2;
    };
    match action.as_str() {
        "create" => {
            let mut output = None;
            while let Some(arg) = args.next() {
                if arg == "--output" {
                    output = args.next();
                } else {
                    eprintln!("uso: backup create --output <diretório>");
                    return 2;
                }
            }
            let Some(output) = output else {
                eprintln!("--output é obrigatório");
                return 2;
            };
            if let Err(error) = config::check_config() {
                eprintln!("configuração inválida: {error}");
                return 78;
            }
            let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
            match runtime.block_on(async {
                db::init_for_backup().await;
                operability::create_backup(std::path::Path::new(&output)).await
            }) {
                Ok(path) => {
                    println!("backup criado: {}", path.display());
                    0
                }
                Err(error) => {
                    eprintln!("backup falhou: {error}");
                    1
                }
            }
        }
        "verify" => {
            let Some(path) = args.next() else {
                eprintln!("uso: backup verify <diretório>");
                return 2;
            };
            if args.next().is_some() {
                eprintln!("uso: backup verify <diretório>");
                return 2;
            }
            let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
            match runtime.block_on(operability::verify_backup(std::path::Path::new(&path))) {
                Ok(()) => {
                    println!("backup válido");
                    0
                }
                Err(error) => {
                    eprintln!("backup inválido: {error}");
                    1
                }
            }
        }
        "restore" => {
            let mut input = None;
            let mut database = None;
            let mut assets = None;
            let mut replace = false;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--input" => input = args.next(),
                    "--database" => database = args.next(),
                    "--assets" => assets = args.next(),
                    "--replace" => replace = true,
                    _ => {
                        eprintln!("uso: backup restore --input <dir> --database <path> --assets <dir> [--replace]");
                        return 2;
                    }
                }
            }
            let (Some(input), Some(database), Some(assets)) = (input, database, assets) else {
                eprintln!("--input, --database e --assets são obrigatórios");
                return 2;
            };
            let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
            let result = runtime.block_on(operability::verify_backup(std::path::Path::new(&input)));
            if let Err(error) = result {
                eprintln!("restore recusado: {error}");
                return 1;
            }
            match operability::restore_backup(
                std::path::Path::new(&input),
                std::path::Path::new(&database),
                std::path::Path::new(&assets),
                replace,
            ) {
                Ok(()) => {
                    println!("restore concluído; inicialize a aplicação e valide readiness");
                    0
                }
                Err(error) => {
                    eprintln!("restore falhou: {error}");
                    1
                }
            }
        }
        _ => {
            eprintln!("ação de backup desconhecida");
            2
        }
    }
}

fn try_handle_server_command() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    let command = args.next()?;
    if command == "sync-fixtures" {
        return Some(run_sync_fixtures_command(args));
    }
    if command == "import-custom-event" {
        return Some(run_import_custom_event_command(args));
    }
    if command == "cleanup-expired" {
        return Some(run_cleanup_expired_command());
    }
    if command == "backfill-results" {
        return Some(run_backfill_results_command());
    }
    if command == "check-config" {
        return Some(run_check_config_command());
    }
    if command == "migrate" {
        return Some(run_migrate_command(args));
    }
    if command == "db" {
        return Some(run_db_command(args));
    }
    if command == "backup" {
        return Some(run_backup_command(args));
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
    crate::config::settings()
        .listen_address
        .parse()
        .expect("LISTEN_ADDRESS inválido para bind do servidor")
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("falha ao registrar SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = term.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    crate::operability::runtime_state().stop_accepting();
    let drained = crate::operability::runtime_state()
        .drain(std::time::Duration::from_secs(
            crate::config::settings().shutdown_timeout_secs,
        ))
        .await;
    if !drained {
        crate::security::log_event(
            "graceful_shutdown_timeout",
            serde_json::json!({
                "in_flight": crate::operability::runtime_state().in_flight(),
            }),
        );
    } else {
        crate::security::log_event("graceful_shutdown_completed", serde_json::json!({}));
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

async fn render_invite_page(
    token: String,
    index_html: std::sync::Arc<String>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let preview = crate::pools::public_invite_preview(token).await.ok();
    let (title, description, image) = match preview {
        Some(preview) if preview.join_status != "invalid" => {
            let pool_name = preview.pool_name.unwrap_or_else(|| "Bolão".to_string());
            let event_name = preview
                .event_name
                .unwrap_or_else(|| "Presumidos".to_string());
            (
                format!("{pool_name} — Presumidos"),
                preview
                    .event_description
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| format!("Entre no bolão do {event_name}")),
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
    let meta = format!(
        "<meta property=\"og:title\" content=\"{}\"><meta property=\"og:description\" content=\"{}\"><meta property=\"og:type\" content=\"website\"><meta property=\"og:image\" content=\"{}\"><meta name=\"description\" content=\"{}\">",
        escape_html(&title),
        escape_html(&description),
        escape_html(&image),
        escape_html(&description),
    );
    let mut html = index_html.replace("</head>", &format!("{meta}</head>"));
    let title_markup = format!("<title>{}</title>", escape_html(&title));
    if let Some(start) = html.find("<title>") {
        if let Some(end) = html[start..].find("</title>") {
            let end = start + end + "</title>".len();
            html.replace_range(start..end, &title_markup);
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

async fn serve_application() {
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
    let spa_index_html = index_html.clone();
    let spa_fallback = move || {
        let index_html = spa_index_html.clone();
        async move { Html(index_html.to_string()) }
    };
    let invite_page = {
        let index_html = index_html.clone();
        move |axum::extract::Path(token): axum::extract::Path<String>| {
            let index_html = index_html.clone();
            async move { render_invite_page(token, index_html).await }
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
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("falha ao servir aplicacao");
}

fn main() {
    if let Some(exit_code) = try_handle_server_command() {
        std::process::exit(exit_code);
    }

    if let Err(error) = config::check_config() {
        eprintln!("configuração inválida: {error}");
        std::process::exit(78);
    }
    let rt = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    rt.block_on(serve_application());
}
