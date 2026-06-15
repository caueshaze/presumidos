#![cfg(all(test, feature = "server"))]

use serde::Deserialize;
use serde_json::json;

use crate::models::{AuthResult, SessionState};

#[derive(Debug, Deserialize)]
struct ErrorPayload {
    error: String,
}

fn seed_http_test_env() {
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
}

/// Sobe a API HTTP uma unica vez por binario de teste e devolve a URL base.
///
/// O servidor roda numa thread + runtime dedicada (não via `tokio::spawn` no
/// runtime do teste), senão ele morreria quando o primeiro `#[tokio::test]` que
/// o inicializou terminasse e derrubasse o próprio runtime — causando
/// "connection refused" nos testes seguintes.
async fn test_server() -> &'static str {
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

#[tokio::test]
async fn contact_endpoint_returns_runtime_configured_email() {
    let base = test_server().await;
    let client = client();

    let response = client
        .get(format!("{base}/api/contact"))
        .send()
        .await
        .expect("contact request");

    assert!(response.status().is_success());
    let payload: serde_json::Value = response.json().await.expect("contact json");
    let expected = crate::config::settings()
        .contact_email
        .clone()
        .unwrap_or_default();
    assert_eq!(payload["email"], expected);
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

async fn login(
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

async fn insert_pool(name: &str, created_by: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let code = uuid::Uuid::new_v4().simple().to_string();
    let code = code[..8].to_uppercase();
    sqlx::query("INSERT INTO pools (id, name, invite_code, created_by) VALUES (?1, ?2, ?3, ?4)")
        .bind(&id)
        .bind(name)
        .bind(&code)
        .bind(created_by)
        .execute(crate::db::pool())
        .await
        .expect("inserir bolao de teste");
    id
}

async fn add_membership(pool_id: &str, user_id: &str) {
    sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, user_id) VALUES (?1, ?2)")
        .bind(pool_id)
        .bind(user_id)
        .execute(crate::db::pool())
        .await
        .expect("inserir membro de teste");
}

/// Membership com `joined_at` explícito (para testar elegibilidade por data de entrada).
async fn add_membership_at(pool_id: &str, user_id: &str, joined_at: &str) {
    sqlx::query(
        "INSERT OR IGNORE INTO pool_members (pool_id, user_id, joined_at) VALUES (?1, ?2, ?3)",
    )
    .bind(pool_id)
    .bind(user_id)
    .bind(joined_at)
    .execute(crate::db::pool())
    .await
    .expect("inserir membro com joined_at");
}

/// Partida com resultado oficial já lançado (entra no cálculo do ranking).
async fn insert_finished_match(
    home: &str,
    away: &str,
    kickoff: &str,
    home_score: i64,
    away_score: i64,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO matches (id, home_team, away_team, kickoff, group_name, phase,
                              home_score, away_score, finished)
         VALUES (?1, ?2, ?3, ?4, 'A', 'Fase de grupos', ?5, ?6, 1)",
    )
    .bind(&id)
    .bind(home)
    .bind(away)
    .bind(kickoff)
    .bind(home_score)
    .bind(away_score)
    .execute(crate::db::pool())
    .await
    .expect("inserir partida finalizada");
    id
}

async fn insert_match(home: &str, away: &str, kickoff: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO matches (id, home_team, away_team, kickoff, group_name, phase)
         VALUES (?1, ?2, ?3, ?4, 'A', 'Fase de grupos')",
    )
    .bind(&id)
    .bind(home)
    .bind(away)
    .bind(kickoff)
    .execute(crate::db::pool())
    .await
    .expect("inserir partida de teste");
    id
}

/// Cria uma sessão diretamente no banco (sem passar pelo endpoint de login, que
/// é limitado por IP). Devolve (token de sessão, csrf token).
async fn seed_session(user_id: &str) -> (String, String) {
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
fn client_with_session(base: &str, token: &str) -> reqwest::Client {
    use std::sync::Arc;
    let jar = Arc::new(reqwest::cookie::Jar::default());
    let url = base.parse::<reqwest::Url>().expect("url base");
    jar.add_cookie_str(&format!("presumidos_session={token}"), &url);
    reqwest::Client::builder()
        .cookie_provider(jar)
        .build()
        .expect("cliente http com sessao")
}

/// Pontos de um usuário no ranking de um bolão (0 se ausente).
async fn leaderboard_points(
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

async fn insert_prediction(user_id: &str, match_id: &str, home: i64, away: i64) {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO predictions (id, user_id, match_id, home_score, away_score)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(match_id)
    .bind(home)
    .bind(away)
    .execute(crate::db::pool())
    .await
    .expect("inserir palpite de teste");
}

#[tokio::test]
async fn login_sets_session_cookie_and_current_user_works() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("login-{suffix}@teste.com");
    seed_user(
        &format!("login-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;

    let client = client();

    let login_response = login(&client, base, &email, "senha-correta-123").await;
    assert!(
        login_response.status().is_success(),
        "login deveria ter sucesso"
    );

    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");
    assert_eq!(auth_result.user.email, email);
    assert!(!auth_result.csrf_token.is_empty());

    let current_response = client
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("requisicao current_user");
    assert!(current_response.status().is_success());

    let session: SessionState = current_response
        .json()
        .await
        .expect("corpo de current_user");
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

    // Senha errada nao deve alterar o hash armazenado.
    let wrong_password = login(&client, base, &email, "senha-incorreta").await;
    assert!(!wrong_password.status().is_success());

    let hash_after_wrong_password: (String,) =
        sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("hash do usuario");
    assert_eq!(hash_after_wrong_password.0, weak_hash);

    // Senha correta com hash desatualizado deve disparar rehash transparente.
    let login_response = login(&client, base, &email, "senha-correta-123").await;
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
    seed_user(
        &format!("logout-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;

    let client = client();

    let login_response = login(&client, base, &email, "senha-correta-123").await;
    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");

    let bad_logout = client
        .post(format!("{base}/api/auth/logout"))
        .header("X-CSRF-Token", "token-errado")
        .send()
        .await
        .expect("requisicao de logout com csrf invalido");
    assert!(!bad_logout.status().is_success());
    let error: ErrorPayload = bad_logout.json().await.expect("corpo de erro");
    assert!(error.error.to_lowercase().contains("seguranca"));

    let still_logged_in = client
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("requisicao current_user");
    let session: SessionState = still_logged_in.json().await.expect("corpo de current_user");
    assert!(session.user.is_some(), "csrf invalido nao deveria deslogar");

    let good_logout = client
        .post(format!("{base}/api/auth/logout"))
        .header("X-CSRF-Token", auth_result.csrf_token)
        .send()
        .await
        .expect("requisicao de logout com csrf valido");
    assert!(good_logout.status().is_success());

    let logged_out = client
        .get(format!("{base}/api/auth/current-user"))
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
    let user_id = seed_user(
        &format!("admin-{suffix}"),
        &email,
        "senha-correta-123",
        true,
    )
    .await;

    let client = client();

    let login_response = login(&client, base, &email, "senha-correta-123").await;
    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");
    let csrf = auth_result.csrf_token;
    let reauth_url = format!("{base}/api/auth/reauth");

    // Senha errada nao altera o estado da sessao.
    let wrong_password = client
        .post(&reauth_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "password": "senha-errada" }))
        .send()
        .await
        .expect("reauth com senha errada");
    assert!(!wrong_password.status().is_success());
    let error: ErrorPayload = wrong_password.json().await.expect("corpo de erro");
    assert!(error
        .error
        .to_lowercase()
        .contains("senha de administrador"));

    let admin_reauthed_after_failure: (Option<String>,) =
        sqlx::query_as("SELECT admin_reauthed_at FROM sessions WHERE user_id = ?1")
            .bind(&user_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("sessao do admin");
    assert!(admin_reauthed_after_failure.0.is_none());

    // Senha correta confirma a reautenticacao recente.
    let right_password = client
        .post(&reauth_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "password": "senha-correta-123" }))
        .send()
        .await
        .expect("reauth com senha correta");
    assert!(right_password.status().is_success());

    let admin_reauthed_after_success: (Option<String>,) =
        sqlx::query_as("SELECT admin_reauthed_at FROM sessions WHERE user_id = ?1")
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
            .post(&reauth_url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({ "password": "senha-errada" }))
            .send()
            .await
            .expect("reauth repetido");
        assert!(!response.status().is_success());
        let error: ErrorPayload = response.json().await.expect("corpo de erro");
        last_message = error.error;
    }
    assert!(
        last_message.to_lowercase().contains("muitas tentativas"),
        "esperava erro de rate limit, recebeu: {last_message}"
    );
}

/// Apagar bolão: o criador consegue; um membro comum não. Os registros filhos
/// (membros, ajustes) somem junto, e os palpites globais do usuário permanecem.
#[tokio::test]
async fn pool_creator_can_delete_pool() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let creator_email = format!("del-creator-{suffix}@teste.com");
    let member_email = format!("del-member-{suffix}@teste.com");
    let creator_id = seed_user(
        &format!("delcreator-{suffix}"),
        &creator_email,
        "senha-correta-123",
        false,
    )
    .await;
    let member_id = seed_user(
        &format!("delmember-{suffix}"),
        &member_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &creator_id).await;
    add_membership(&pool_id, &creator_id).await;
    add_membership(&pool_id, &member_id).await;

    let del_url = format!("{base}/api/pools/{pool_id}/delete");

    // Membro comum NÃO pode apagar.
    let (member_token, member_csrf) = seed_session(&member_id).await;
    let member_c = client_with_session(base, &member_token);
    let denied = member_c
        .post(&del_url)
        .header("X-CSRF-Token", &member_csrf)
        .send()
        .await
        .expect("delete por membro comum");
    assert!(
        !denied.status().is_success(),
        "membro comum nao deveria apagar"
    );

    // Pool e membros continuam existindo após a tentativa barrada.
    let still_there: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE id = ?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("contar pool");
    assert_eq!(still_there.0, 1);

    // Criador apaga.
    let (creator_token, creator_csrf) = seed_session(&creator_id).await;
    let creator_c = client_with_session(base, &creator_token);
    let deleted = creator_c
        .post(&del_url)
        .header("X-CSRF-Token", &creator_csrf)
        .send()
        .await
        .expect("delete pelo criador");
    assert!(
        deleted.status().is_success(),
        "criador deveria poder apagar"
    );

    // Pool e pool_members somem; nenhum órfão.
    let pools_left: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE id = ?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("contar pool apos delete");
    assert_eq!(pools_left.0, 0, "bolao deveria ter sido apagado");

    let members_left: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE pool_id = ?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar membros apos delete");
    assert_eq!(
        members_left.0, 0,
        "membros do bolao deveriam ter sido removidos"
    );
}

/// Elegibilidade por data de entrada: palpites de jogos que começaram ANTES de
/// o usuário entrar no bolão não pontuam (sem retroatividade). Palpites de jogos
/// que começaram depois da entrada contam normalmente.
#[tokio::test]
async fn leaderboard_ignores_predictions_from_before_join() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("joiner-{suffix}@teste.com");
    let user_id = seed_user(
        &format!("joiner-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_id).await;
    // Entrou no bolão em 2022.
    add_membership_at(&pool_id, &user_id, "2022-01-01 00:00:00").await;

    // Jogo anterior à entrada (2020): palpite exato valeria 7, mas NÃO deve contar.
    let old_match =
        insert_finished_match("Brasil", "Argentina", "2020-01-01T00:00:00Z", 2, 1).await;
    insert_prediction(&user_id, &old_match, 2, 1).await;

    // Jogo posterior à entrada (2023): palpite exato vale 7 e DEVE contar.
    let new_match = insert_finished_match("Franca", "Espanha", "2023-01-01T00:00:00Z", 1, 0).await;
    insert_prediction(&user_id, &new_match, 1, 0).await;

    let (token, _csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    // Só o jogo posterior à entrada pontua: 7 (e não 14).
    assert_eq!(
        leaderboard_points(&client, base, &pool_id, &user_id).await,
        7,
        "apenas o palpite do jogo posterior a entrada deve pontuar"
    );
}

/// Ajuste manual de pontos: criador e admin podem lançar/remover, o total reflete
/// no ranking, membro comum é barrado para lançar mas vê os ajustes (transparência).
#[tokio::test]
async fn pool_creator_and_admin_can_adjust_points() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let creator_email = format!("creator-{suffix}@teste.com");
    let target_email = format!("target-adj-{suffix}@teste.com");
    let admin_email = format!("admin-adj-{suffix}@teste.com");
    let outsider_email = format!("outsider-adj-{suffix}@teste.com");
    let creator_id = seed_user(
        &format!("creator-{suffix}"),
        &creator_email,
        "senha-correta-123",
        false,
    )
    .await;
    let target_id = seed_user(
        &format!("targetadj-{suffix}"),
        &target_email,
        "senha-correta-123",
        false,
    )
    .await;
    let admin_id = seed_user(
        &format!("adminadj-{suffix}"),
        &admin_email,
        "senha-correta-123",
        true,
    )
    .await;
    let outsider_id = seed_user(
        &format!("outadj-{suffix}"),
        &outsider_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &creator_id).await;
    add_membership(&pool_id, &creator_id).await;
    add_membership(&pool_id, &target_id).await;

    let adj_url = format!("{base}/api/pools/{pool_id}/adjustments");

    // Criador lança +5 para o alvo (sessão semeada, sem usar o endpoint de login).
    let (creator_token, creator_csrf) = seed_session(&creator_id).await;
    let creator_c = client_with_session(base, &creator_token);

    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        0
    );

    let added = creator_c
        .post(&adj_url)
        .header("X-CSRF-Token", &creator_csrf)
        .json(&json!({ "userId": target_id, "delta": 5, "reason": "erro de placar" }))
        .send()
        .await
        .expect("lancar ajuste");
    assert!(added.status().is_success(), "criador deveria poder ajustar");
    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        5
    );

    // Lista de ajustes (criador, membro) tem 1 item.
    let list: Vec<crate::models::PointAdjustment> = creator_c
        .get(&adj_url)
        .send()
        .await
        .expect("listar ajustes")
        .json()
        .await
        .expect("corpo ajustes");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].delta, 5);
    let adjustment_id = list[0].id.clone();

    // Membro comum não pode lançar, mas enxerga os ajustes (transparência).
    let (target_token, target_csrf) = seed_session(&target_id).await;
    let target_c = client_with_session(base, &target_token);
    let denied = target_c
        .post(&adj_url)
        .header("X-CSRF-Token", &target_csrf)
        .json(&json!({ "userId": target_id, "delta": 99, "reason": "trapaca" }))
        .send()
        .await
        .expect("ajuste por membro comum");
    assert!(
        !denied.status().is_success(),
        "membro comum nao deveria ajustar"
    );
    let seen: Vec<crate::models::PointAdjustment> = target_c
        .get(&adj_url)
        .send()
        .await
        .expect("membro lista ajustes")
        .json()
        .await
        .expect("corpo ajustes membro");
    assert_eq!(seen.len(), 1, "membro deveria ver o ajuste (transparencia)");

    // Admin global ajusta um bolão que não criou: +2.
    let (admin_token, admin_csrf) = seed_session(&admin_id).await;
    let admin_c = client_with_session(base, &admin_token);
    let admin_added = admin_c
        .post(&adj_url)
        .header("X-CSRF-Token", &admin_csrf)
        .json(&json!({ "userId": target_id, "delta": 2, "reason": "bonus admin" }))
        .send()
        .await
        .expect("ajuste do admin");
    assert!(
        admin_added.status().is_success(),
        "admin deveria poder ajustar"
    );
    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        7
    );

    // Criador remove o ajuste de +5: total volta a 2.
    let removed = creator_c
        .post(format!("{base}/api/pools/{pool_id}/adjustments/remove"))
        .header("X-CSRF-Token", &creator_csrf)
        .json(&json!({ "adjustmentId": adjustment_id }))
        .send()
        .await
        .expect("remover ajuste");
    assert!(
        removed.status().is_success(),
        "criador deveria poder remover"
    );
    assert_eq!(
        leaderboard_points(&creator_c, base, &pool_id, &target_id).await,
        2
    );

    // Não-membro é barrado ao listar ajustes.
    let (outsider_token, _) = seed_session(&outsider_id).await;
    let outsider_c = client_with_session(base, &outsider_token);
    let outsider_list = outsider_c
        .get(&adj_url)
        .send()
        .await
        .expect("nao-membro lista ajustes");
    assert!(
        !outsider_list.status().is_success(),
        "nao-membro nao deveria listar"
    );
}

/// Troca de nome de usuário: aplica o novo nome, mas rejeita um nome já em uso
/// por outra conta (case-insensitive).
#[tokio::test]
async fn change_username_updates_and_rejects_duplicates() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    // Sufixo curto: o endpoint limita o nome a 32 caracteres (um UUID já tem 36).
    let short = suffix.simple().to_string();
    let short = &short[..8];
    let email = format!("rename-{suffix}@teste.com");
    let other_email = format!("other-{suffix}@teste.com");
    let user_id = seed_user(
        &format!("rename-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;
    let taken_name = format!("taken{short}");
    seed_user(&taken_name, &other_email, "senha-correta-123", false).await;

    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);
    let url = format!("{base}/api/auth/username");

    // Nome novo e livre: sucesso, e a sessão passa a refletir o novo nome.
    let new_name = format!("novo{short}");
    let ok = client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "username": new_name }))
        .send()
        .await
        .expect("trocar nome");
    assert!(
        ok.status().is_success(),
        "troca de nome deveria ter sucesso"
    );
    let updated: crate::models::UserPublic = ok.json().await.expect("usuario atualizado");
    assert_eq!(updated.username, new_name);

    let stored: (String,) = sqlx::query_as("SELECT username FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("nome no banco");
    assert_eq!(stored.0, new_name);

    // Nome já usado por outra conta (variando maiúsc./minúsc.): rejeitado.
    let dup = client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "username": taken_name.to_uppercase() }))
        .send()
        .await
        .expect("trocar para nome ocupado");
    assert!(
        !dup.status().is_success(),
        "nome em uso deveria ser rejeitado"
    );
    let err: ErrorPayload = dup.json().await.expect("corpo de erro");
    assert!(err.error.to_lowercase().contains("ja esta em uso"));

    // O nome no banco não mudou após a tentativa rejeitada.
    let unchanged: (String,) = sqlx::query_as("SELECT username FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("nome no banco apos rejeicao");
    assert_eq!(unchanged.0, new_name);
}

#[tokio::test]
async fn delete_account_removes_user_data_and_logs_out() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner_email = format!("owner-{suffix}@teste.com");
    let member_email = format!("member-{suffix}@teste.com");
    let owner_id = seed_user(
        &format!("owner-{suffix}"),
        &owner_email,
        "senha-correta-123",
        false,
    )
    .await;
    let member_id = seed_user(
        &format!("member-{suffix}"),
        &member_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao do owner {suffix}"), &owner_id).await;
    add_membership(&pool_id, &owner_id).await;
    add_membership(&pool_id, &member_id).await;

    let match_id = insert_match("Brasil", "Japao", "2999-01-01T00:00:00Z").await;
    insert_prediction(&member_id, &match_id, 2, 1).await;

    sqlx::query(
        "INSERT INTO notification_preferences (user_id, enabled, lead_time_minutes) VALUES (?1, 1, 20)",
    )
    .bind(&member_id)
    .execute(crate::db::pool())
    .await
    .expect("preferencia de notificacao");

    sqlx::query(
        "INSERT INTO push_subscriptions
            (id, user_id, endpoint, p256dh, auth, active)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&member_id)
    .bind(format!("https://push.example/{suffix}"))
    .bind("p256dh-teste")
    .bind("auth-teste")
    .execute(crate::db::pool())
    .await
    .expect("subscription de push");

    let (token, csrf) = seed_session(&member_id).await;
    let client = client_with_session(base, &token);
    let delete_url = format!("{base}/api/auth/delete");

    let deleted = client
        .post(&delete_url)
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .expect("excluir conta");
    assert!(
        deleted.status().is_success(),
        "exclusao deveria ter sucesso"
    );

    let current = client
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("current_user apos exclusao");
    let session: SessionState = current.json().await.expect("sessao apos exclusao");
    assert!(session.user.is_none(), "sessao deveria estar encerrada");

    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?1")
        .bind(&member_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("contar usuario");
    assert_eq!(user_count.0, 0);

    let prediction_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar palpites");
    assert_eq!(prediction_count.0, 0);

    let membership_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pool_members WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar memberships");
    assert_eq!(membership_count.0, 0);

    let pref_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notification_preferences WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar preferencias");
    assert_eq!(pref_count.0, 0);

    let push_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM push_subscriptions WHERE user_id = ?1")
            .bind(&member_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("contar subscriptions");
    assert_eq!(push_count.0, 0);

    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'account_deleted' AND target_id = ?1",
    )
    .bind(&member_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("contar auditoria de exclusao");
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn delete_account_blocks_pool_owner() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email = format!("pool-owner-{suffix}@teste.com");
    let user_id = seed_user(
        &format!("pool-owner-{suffix}"),
        &email,
        "senha-correta-123",
        false,
    )
    .await;
    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_id).await;
    add_membership(&pool_id, &user_id).await;

    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    let blocked = client
        .post(format!("{base}/api/auth/delete"))
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .expect("exclusao bloqueada");
    assert!(
        !blocked.status().is_success(),
        "criador de bolao nao deveria excluir a conta"
    );
    let err: ErrorPayload = blocked.json().await.expect("erro da exclusao bloqueada");
    assert!(err.error.to_lowercase().contains("criou bol"));

    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("usuario ainda existe");
    assert_eq!(user_count.0, 1);
}

#[tokio::test]
async fn load_active_subscriptions_includes_admin_accounts() {
    let _base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin_email = format!("admin-push-{suffix}@teste.com");
    let user_email = format!("user-push-{suffix}@teste.com");
    let admin_id =
        seed_user(&format!("admin-push-{suffix}"), &admin_email, "senha-correta-123", true).await;
    let user_id =
        seed_user(&format!("user-push-{suffix}"), &user_email, "senha-correta-123", false).await;

    for (user_id, endpoint) in [
        (&admin_id, format!("https://push.example/admin-{suffix}")),
        (&user_id, format!("https://push.example/user-{suffix}")),
    ] {
        sqlx::query(
            "INSERT INTO notification_preferences (user_id, enabled, lead_time_minutes)
             VALUES (?1, 1, 20)",
        )
        .bind(user_id)
        .execute(crate::db::pool())
        .await
        .expect("preferencia ativa");

        sqlx::query(
            "INSERT INTO push_subscriptions
                (id, user_id, endpoint, p256dh, auth, active)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(endpoint)
        .bind("p256dh-teste")
        .bind("auth-teste")
        .execute(crate::db::pool())
        .await
        .expect("subscription ativa");
    }

    let grouped = crate::push::test_active_subscription_user_ids(crate::db::pool())
        .await
        .expect("subscriptions ativas");

    assert!(grouped.contains(&admin_id), "admin deveria receber notificacoes se ativar push");
    assert!(grouped.contains(&user_id), "usuario comum deveria seguir recebendo notificacoes");
}

/// Regra de privacidade: os palpites de um membro só ficam visíveis depois que
/// a partida começa (kickoff <= agora). Jogos no futuro não podem vazar.
#[tokio::test]
async fn pool_member_predictions_hides_matches_before_kickoff() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email_a = format!("memberA-{suffix}@teste.com");
    let email_b = format!("memberB-{suffix}@teste.com");
    let email_c = format!("outsider-{suffix}@teste.com");
    let user_a = seed_user(
        &format!("memberA-{suffix}"),
        &email_a,
        "senha-correta-123",
        false,
    )
    .await;
    let user_b = seed_user(
        &format!("memberB-{suffix}"),
        &email_b,
        "senha-correta-123",
        false,
    )
    .await;
    let user_c = seed_user(
        &format!("outsider-{suffix}"),
        &email_c,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_a).await;
    // Entraram no bolão antes do jogo "passado", para isolar o teste da regra de
    // elegibilidade por data de entrada (coberta em outro teste).
    add_membership_at(&pool_id, &user_a, "2019-01-01 00:00:00").await;
    add_membership_at(&pool_id, &user_b, "2019-01-01 00:00:00").await;

    let past_match = insert_match("Brasil", "Argentina", "2020-01-01T00:00:00Z").await;
    let future_match = insert_match("Franca", "Espanha", "2999-01-01T00:00:00Z").await;

    // O membro B palpitou nos dois jogos (um já iniciado, um no futuro).
    insert_prediction(&user_b, &past_match, 2, 1).await;
    insert_prediction(&user_b, &future_match, 0, 0).await;

    // Membro A consulta os palpites do bolão (sessão semeada, sem login).
    let (token_a, _) = seed_session(&user_a).await;
    let viewer = client_with_session(base, &token_a);
    let response = viewer
        .get(format!("{base}/api/pools/{pool_id}/member-predictions"))
        .send()
        .await
        .expect("requisicao member-predictions");
    assert!(
        response.status().is_success(),
        "membro deveria poder consultar"
    );

    let members: Vec<crate::models::MemberPredictions> =
        response.json().await.expect("corpo member-predictions");
    let b = members
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("membro B presente na resposta");

    // Apenas o palpite do jogo já iniciado deve aparecer.
    assert_eq!(
        b.predictions.len(),
        1,
        "apenas o palpite do jogo iniciado deve ser visivel, e nao o do futuro"
    );
    assert_eq!(b.predictions[0].match_id, past_match);

    // Quem não é membro do bolão é barrado.
    let (token_c, _) = seed_session(&user_c).await;
    let outsider = client_with_session(base, &token_c);
    let denied = outsider
        .get(format!("{base}/api/pools/{pool_id}/member-predictions"))
        .send()
        .await
        .expect("requisicao de nao-membro");
    assert!(
        !denied.status().is_success(),
        "nao-membro nao deveria acessar"
    );
}

/// Gestão de membros (admin): adicionar/remover exige admin + reautenticação
/// recente + CSRF; usuário comum é barrado.
#[tokio::test]
async fn admin_can_add_and_remove_pool_members() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin_email = format!("admin-mgmt-{suffix}@teste.com");
    let target_email = format!("target-{suffix}@teste.com");
    let admin_id = seed_user(
        &format!("admin-mgmt-{suffix}"),
        &admin_email,
        "senha-correta-123",
        true,
    )
    .await;
    let target_id = seed_user(
        &format!("target-{suffix}"),
        &target_email,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao Admin {suffix}"), &admin_id).await;

    let (admin_token, csrf) = seed_session(&admin_id).await;
    let admin = client_with_session(base, &admin_token);
    let add_url = format!("{base}/api/admin/pools/{pool_id}/members");
    let members_url = add_url.clone();

    // Sem reautenticação recente, a ação é bloqueada.
    let needs_reauth = admin
        .post(&add_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "userId": target_id }))
        .send()
        .await
        .expect("add sem reauth");
    assert_eq!(needs_reauth.status().as_u16(), 403);
    let err: ErrorPayload = needs_reauth.json().await.expect("corpo de erro");
    assert!(
        err.error.contains("ADMIN_REAUTH_REQUIRED"),
        "esperava exigencia de reauth, recebeu: {}",
        err.error
    );

    // Marca a sessão como reautenticada recentemente (sem passar pelo endpoint
    // de reauth, para não interferir no rate limit compartilhado dos testes).
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE user_id = ?1")
        .bind(&admin_id)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth recente");

    // Adiciona o usuário ao bolão.
    let added = admin
        .post(&add_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "userId": target_id }))
        .send()
        .await
        .expect("add membro");
    assert!(added.status().is_success(), "add deveria ter sucesso");

    // A listagem de membros passa a conter o alvo.
    let listed: Vec<crate::models::UserPublic> = admin
        .get(&members_url)
        .send()
        .await
        .expect("listar membros")
        .json()
        .await
        .expect("corpo de membros");
    assert!(
        listed.iter().any(|u| u.id == target_id),
        "alvo deveria estar nos membros"
    );

    // Remove o usuário do bolão.
    let removed = admin
        .post(format!("{base}/api/admin/pools/{pool_id}/members/remove"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "userId": target_id }))
        .send()
        .await
        .expect("remover membro");
    assert!(removed.status().is_success(), "remove deveria ter sucesso");

    let after: Vec<crate::models::UserPublic> = admin
        .get(&members_url)
        .send()
        .await
        .expect("listar membros apos remocao")
        .json()
        .await
        .expect("corpo de membros 2");
    assert!(
        !after.iter().any(|u| u.id == target_id),
        "alvo deveria ter sido removido"
    );

    // Usuário comum não pode gerenciar membros.
    let (normal_token, normal_csrf) = seed_session(&target_id).await;
    let normal = client_with_session(base, &normal_token);
    let denied = normal
        .post(&add_url)
        .header("X-CSRF-Token", &normal_csrf)
        .json(&json!({ "userId": admin_id }))
        .send()
        .await
        .expect("add por nao-admin");
    assert!(
        !denied.status().is_success(),
        "usuario comum nao deveria poder gerenciar membros"
    );
}
