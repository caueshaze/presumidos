#![cfg(all(test, feature = "server"))]

pub(crate) use serde::Deserialize;
pub(crate) use serde_json::json;
pub(crate) use sha2::Digest;
pub(crate) use std::collections::HashSet;
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::Command;

pub(crate) use crate::models::{AdminEventRecord, AdminSettings, AuthResult, SessionState};

#[derive(Debug, Deserialize)]
pub(crate) struct ErrorPayload {
    pub(crate) error: String,
}

pub(crate) fn seed_http_test_env() {
    let db_path =
        std::env::temp_dir().join(format!("presumidos-http-test-{}.db", uuid::Uuid::new_v4()));
    std::env::set_var("APP_ENV", "test");
    std::env::set_var("DATABASE_PATH", db_path.to_string_lossy().to_string());
    std::env::set_var("CONTACT_EMAIL", "contato@example.com");
    std::env::set_var(
        "SESSION_SECRET",
        "0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    std::env::set_var(
        "ADMIN_BOOTSTRAP_SECRET",
        "bootstrap-secret-super-seguro-0123456789abcdef",
    );
    std::env::set_var("SESSION_TTL_HOURS", "12");
    std::env::set_var("COOKIE_SECURE", "false");
    std::env::set_var("ADMIN_REAUTH_TTL_MINUTES", "10");
    std::env::set_var("TRUSTED_PROXY_CIDRS", "");
    std::env::set_var("REQUIRE_TRUSTED_PROXY", "false");
    std::env::set_var("RESEND_API_KEY", "test-key");
    std::env::set_var("RESEND_FROM_EMAIL", "teste@presumidos.dev");
    std::env::set_var("RATE_LIMIT_BACKEND", "memory");
    std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
    std::env::set_var(
        "PRESUMIDOS_ASSET_DIR",
        std::env::temp_dir()
            .join(format!("presumidos-http-assets-{}", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .to_string(),
    );
}

/// Sobe a API HTTP uma unica vez por binario de teste e devolve a URL base.
///
/// O servidor roda numa thread + runtime dedicada (não via `tokio::spawn` no
/// runtime do teste), senão ele morreria quando o primeiro `#[tokio::test]` que
/// o inicializou terminasse e derrubasse o próprio runtime — causando
/// "connection refused" nos testes seguintes.
pub(crate) async fn test_server() -> &'static str {
    static SERVER: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();
    SERVER
        .get_or_init(|| async {
            seed_http_test_env();
            crate::db::init().await;

            let std_listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("bind do listener de teste");
            std_listener
                .set_nonblocking(true)
                .expect("listener nao-bloqueante");
            let addr = std_listener.local_addr().expect("endereco local");

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime do servidor de teste");
                rt.block_on(async move {
                    let app = axum::Router::new()
                        .nest("/api", crate::api::router())
                        .route(
                            "/media/assets/{asset_id}/{variant}",
                            axum::routing::get(crate::api::media_asset),
                        )
                        .layer(axum::middleware::from_fn(crate::api::context_middleware));
                    let listener = tokio::net::TcpListener::from_std(std_listener)
                        .expect("converter listener para tokio");
                    axum::serve(
                        listener,
                        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                    )
                    .await
                    .expect("servidor de teste falhou");
                });
            });

            format!("http://{addr}")
        })
        .await
}

pub(crate) fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("cliente http")
}

pub(crate) async fn seed_user(
    username: &str,
    email: &str,
    password: &str,
    is_admin: bool,
) -> String {
    let hash = crate::auth::hash_password(password).expect("hash de senha");
    crate::auth::insert_user_account(crate::db::pool(), username, email, &hash, is_admin)
        .await
        .expect("inserir usuario de teste")
}

/// Gera um hash Argon2id valido, mas com parametros mais fracos que a
/// politica atual, para simular uma conta criada antes de um reforco.
pub(crate) fn weak_password_hash(password: &str) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Algorithm, Argon2, Params, Version};
    use rand_core::OsRng;

    let weak_params = Params::new(19456, 1, 1, None).expect("parametros fracos");
    let weak_hasher = Argon2::new(Algorithm::Argon2id, Version::V0x13, weak_params);
    let salt = SaltString::generate(&mut OsRng);
    weak_hasher
        .hash_password(password.as_bytes(), &salt)
        .expect("hash com parametros fracos")
        .to_string()
}

pub(crate) async fn login(
    client: &reqwest::Client,
    base: &str,
    email: &str,
    password: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/api/auth/login"))
        .json(&json!({ "username": email, "password": password }))
        .send()
        .await
        .expect("requisicao de login")
}

pub(crate) async fn seed_session(user_id: &str) -> (String, String) {
    let token = uuid::Uuid::new_v4().to_string();
    let csrf = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sessions (token, user_id, expires_at, csrf_token, last_seen_at)
         VALUES (?1, ?2, datetime('now', '+12 hours'), ?3, datetime('now'))",
    )
    .bind(&token)
    .bind(user_id)
    .bind(&csrf)
    .execute(crate::db::pool())
    .await
    .expect("inserir sessao de teste");
    (token, csrf)
}

/// Cliente HTTP autenticado com o cookie de sessão pré-preenchido.
pub(crate) fn client_with_session(base: &str, token: &str) -> reqwest::Client {
    use std::sync::Arc;
    let jar = Arc::new(reqwest::cookie::Jar::default());
    let url = base.parse::<reqwest::Url>().expect("url base");
    jar.add_cookie_str(&format!("presumidos_session={token}"), &url);
    reqwest::Client::builder()
        .cookie_provider(jar)
        .build()
        .expect("cliente http com sessao")
}

pub(crate) async fn leaderboard_points(
    client: &reqwest::Client,
    base: &str,
    pool_id: &str,
    user_id: &str,
) -> i64 {
    let entries: Vec<crate::models::LeaderboardEntry> = client
        .get(format!("{base}/api/leaderboard?poolId={pool_id}"))
        .send()
        .await
        .expect("requisicao leaderboard")
        .json()
        .await
        .expect("corpo leaderboard");
    entries
        .iter()
        .find(|e| e.user_id == user_id)
        .map(|e| e.points)
        .unwrap_or(0)
}

pub(crate) async fn insert_prediction(user_id: &str, match_id: &str, home: i64, away: i64) {
    let id = uuid::Uuid::new_v4().to_string();
    let pool_id: (String,) = sqlx::query_as(
        "SELECT pm.pool_id FROM pool_members pm JOIN pools p ON p.id = pm.pool_id
         JOIN matches m ON m.id = ?2 JOIN prediction_items pi ON pi.id = m.prediction_item_id
         WHERE pm.user_id = ?1 AND p.event_id = pi.event_id LIMIT 1",
    )
    .bind(user_id)
    .bind(match_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("pool compativel para palpite de teste");
    let item_id: (String,) = sqlx::query_as("SELECT prediction_item_id FROM matches WHERE id = ?1")
        .bind(match_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("item do match de teste");
    sqlx::query(
        "INSERT INTO predictions (id, pool_id, user_id, item_id, match_id, home_score, away_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&id)
    .bind(&pool_id.0)
    .bind(user_id)
    .bind(&item_id.0)
    .bind(match_id)
    .bind(home)
    .bind(away)
    .execute(crate::db::pool())
    .await
    .expect("inserir palpite de teste");
}
