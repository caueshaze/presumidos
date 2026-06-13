#![cfg(all(test, feature = "server"))]

use dioxus::prelude::*;
use dioxus::server::{http::Method, FullstackState, ServerFunction};
use serde::Deserialize;
use serde_json::json;

use crate::models::{AuthResult, SessionState};

#[derive(Debug, Deserialize)]
struct ErrorPayload {
    message: String,
}

fn seed_http_test_env() {
    let db_path = std::env::temp_dir().join(format!("presumidos-http-test-{}.db", uuid::Uuid::new_v4()));
    std::env::set_var("APP_ENV", "test");
    std::env::set_var("DATABASE_PATH", db_path.to_string_lossy().to_string());
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
}

/// Sobe o servidor de server functions uma unica vez por binario de teste e
/// devolve a URL base (`http://127.0.0.1:PORT`).
async fn test_server() -> &'static str {
    static SERVER: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();
    SERVER
        .get_or_init(|| async {
            seed_http_test_env();
            crate::db::init().await;

            let app = dioxus::server::axum::Router::new()
                .register_server_functions()
                .with_state(FullstackState::headless());

            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind do listener de teste");
            let addr = listener.local_addr().expect("endereco local");
            tokio::spawn(async move {
                dioxus::server::axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                )
                .await
                .expect("servidor de teste falhou");
            });

            format!("http://{addr}")
        })
        .await
}

/// Descobre a rota registrada para uma server function pelo nome, ja que o
/// macro `#[server]` aplica um sufixo hash ao path (`/api/<fn><hash>`).
fn route_for(fn_name: &str) -> (Method, String) {
    let prefix = format!("/api/{fn_name}");
    ServerFunction::collect()
        .into_iter()
        .find(|f| {
            f.path().starts_with(&prefix)
                && f.path()[prefix.len()..].chars().all(|c| c.is_ascii_digit())
        })
        .map(|f| (f.method(), f.path().to_string()))
        .unwrap_or_else(|| panic!("rota nao encontrada para {fn_name}"))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("cliente http")
}

async fn seed_user(username: &str, email: &str, password: &str, is_admin: bool) -> String {
    let hash = crate::auth::hash_password(password).expect("hash de senha");
    crate::auth::insert_user_account(crate::db::pool(), username, email, &hash, is_admin)
        .await
        .expect("inserir usuario de teste")
}

/// Gera um hash Argon2id valido, mas com parametros mais fracos que a
/// politica atual, para simular uma conta criada antes de um reforco.
fn weak_password_hash(password: &str) -> String {
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

#[tokio::test]
async fn login_sets_session_cookie_and_current_user_works() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("login-{suffix}@teste.com");
    seed_user(&format!("login-{suffix}"), &email, "senha-correta-123", false).await;

    let client = client();

    let (method, path) = route_for("login");
    let login_response = client
        .request(method, format!("{base}{path}"))
        .json(&json!({ "username": email, "password": "senha-correta-123" }))
        .send()
        .await
        .expect("requisicao de login");
    assert!(login_response.status().is_success(), "login deveria ter sucesso");

    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");
    assert_eq!(auth_result.user.email, email);
    assert!(!auth_result.csrf_token.is_empty());

    let (method, path) = route_for("current_user");
    let current_response = client
        .request(method, format!("{base}{path}"))
        .json(&json!({ "token": "" }))
        .send()
        .await
        .expect("requisicao current_user");
    assert!(current_response.status().is_success());

    let session: SessionState = current_response.json().await.expect("corpo de current_user");
    let user = session.user.expect("sessao deveria ter usuario");
    assert_eq!(user.email, email);
    assert_eq!(session.csrf_token, auth_result.csrf_token);
}

#[tokio::test]
async fn login_rehashes_password_with_outdated_parameters() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("rehash-{suffix}@teste.com");
    let weak_hash = weak_password_hash("senha-correta-123");
    let user_id = crate::auth::insert_user_account(
        crate::db::pool(),
        &format!("rehash-{suffix}"),
        &email,
        &weak_hash,
        false,
    )
    .await
    .expect("inserir usuario com hash fraco");

    let client = client();
    let (method, path) = route_for("login");

    // Senha errada nao deve alterar o hash armazenado.
    let wrong_password = client
        .request(method.clone(), format!("{base}{path}"))
        .json(&json!({ "username": email, "password": "senha-incorreta" }))
        .send()
        .await
        .expect("requisicao de login com senha errada");
    assert!(!wrong_password.status().is_success());

    let hash_after_wrong_password: (String,) =
        sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("hash do usuario");
    assert_eq!(hash_after_wrong_password.0, weak_hash);

    // Senha correta com hash desatualizado deve disparar rehash transparente.
    let login_response = client
        .request(method, format!("{base}{path}"))
        .json(&json!({ "username": email, "password": "senha-correta-123" }))
        .send()
        .await
        .expect("requisicao de login com senha correta");
    assert!(login_response.status().is_success());

    let hash_after_login: (String,) =
        sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("hash do usuario");
    assert_ne!(hash_after_login.0, weak_hash);

    let parsed = argon2::password_hash::PasswordHash::new(&hash_after_login.0)
        .expect("hash novo deve ser valido");
    let cfg = crate::config::settings();
    let params = argon2::Params::try_from(&parsed).expect("params do hash novo");
    assert_eq!(params.m_cost(), cfg.argon2_memory_kib);
    assert_eq!(params.t_cost(), cfg.argon2_time_cost);
    assert_eq!(params.p_cost(), cfg.argon2_parallelism);
}

#[tokio::test]
async fn logout_requires_valid_csrf_token() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("logout-{suffix}@teste.com");
    seed_user(&format!("logout-{suffix}"), &email, "senha-correta-123", false).await;

    let client = client();

    let (method, path) = route_for("login");
    let login_response = client
        .request(method, format!("{base}{path}"))
        .json(&json!({ "username": email, "password": "senha-correta-123" }))
        .send()
        .await
        .expect("requisicao de login");
    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");

    let (method, path) = route_for("logout");

    let bad_logout = client
        .request(method.clone(), format!("{base}{path}"))
        .json(&json!({ "token": "", "csrf_token": "token-errado" }))
        .send()
        .await
        .expect("requisicao de logout com csrf invalido");
    assert!(!bad_logout.status().is_success());
    let error: ErrorPayload = bad_logout.json().await.expect("corpo de erro");
    assert!(error.message.to_lowercase().contains("seguranca"));

    let (method2, path2) = route_for("current_user");
    let still_logged_in = client
        .request(method2.clone(), format!("{base}{path2}"))
        .json(&json!({ "token": "" }))
        .send()
        .await
        .expect("requisicao current_user");
    let session: SessionState = still_logged_in.json().await.expect("corpo de current_user");
    assert!(session.user.is_some(), "csrf invalido nao deveria deslogar");

    let good_logout = client
        .request(method, format!("{base}{path}"))
        .json(&json!({ "token": "", "csrf_token": auth_result.csrf_token }))
        .send()
        .await
        .expect("requisicao de logout com csrf valido");
    assert!(good_logout.status().is_success());

    let logged_out = client
        .request(method2, format!("{base}{path2}"))
        .json(&json!({ "token": "" }))
        .send()
        .await
        .expect("requisicao current_user apos logout");
    let session: SessionState = logged_out.json().await.expect("corpo de current_user");
    assert!(session.user.is_none(), "sessao deveria estar encerrada");
}

#[tokio::test]
async fn admin_reauth_flow_and_rate_limit() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("admin-{suffix}@teste.com");
    let user_id = seed_user(&format!("admin-{suffix}"), &email, "senha-correta-123", true).await;

    let client = client();

    let (method, path) = route_for("login");
    let login_response = client
        .request(method, format!("{base}{path}"))
        .json(&json!({ "username": email, "password": "senha-correta-123" }))
        .send()
        .await
        .expect("requisicao de login");
    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");

    let (method, path) = route_for("confirm_admin_password");

    // Senha errada nao altera o estado da sessao.
    let wrong_password = client
        .request(method.clone(), format!("{base}{path}"))
        .json(&json!({ "password": "senha-errada", "csrf_token": auth_result.csrf_token }))
        .send()
        .await
        .expect("confirm_admin_password com senha errada");
    assert!(!wrong_password.status().is_success());
    let error: ErrorPayload = wrong_password.json().await.expect("corpo de erro");
    assert!(error.message.to_lowercase().contains("senha de administrador"));

    let admin_reauthed_after_failure: (Option<String>,) = sqlx::query_as(
        "SELECT admin_reauthed_at FROM sessions WHERE user_id = ?1",
    )
    .bind(&user_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("sessao do admin");
    assert!(admin_reauthed_after_failure.0.is_none());

    // Senha correta confirma a reautenticacao recente.
    let right_password = client
        .request(method.clone(), format!("{base}{path}"))
        .json(&json!({ "password": "senha-correta-123", "csrf_token": auth_result.csrf_token }))
        .send()
        .await
        .expect("confirm_admin_password com senha correta");
    assert!(right_password.status().is_success());

    let admin_reauthed_after_success: (Option<String>,) = sqlx::query_as(
        "SELECT admin_reauthed_at FROM sessions WHERE user_id = ?1",
    )
    .bind(&user_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("sessao do admin");
    assert!(admin_reauthed_after_success.0.is_some());

    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'admin_reauthenticated' AND actor_user_id = ?1",
    )
    .bind(&user_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("audit log");
    assert_eq!(audit_count.0, 1);

    // Ja foram feitas 2 chamadas (errada + correta) nesta janela. O limite por
    // IP e de 8 tentativas/min, entao mais 6 chamadas com senha errada devem
    // estourar o limite na setima.
    let mut last_message = String::new();
    for _ in 0..6 {
        let response = client
            .request(method.clone(), format!("{base}{path}"))
            .json(&json!({ "password": "senha-errada", "csrf_token": auth_result.csrf_token }))
            .send()
            .await
            .expect("confirm_admin_password repetido");
        assert!(!response.status().is_success());
        let error: ErrorPayload = response.json().await.expect("corpo de erro");
        last_message = error.message;
    }
    assert!(
        last_message.to_lowercase().contains("muitas tentativas"),
        "esperava erro de rate limit, recebeu: {last_message}"
    );
}
