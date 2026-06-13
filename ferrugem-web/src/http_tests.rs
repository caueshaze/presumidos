#![cfg(all(test, feature = "server"))]

use serde::Deserialize;
use serde_json::json;

use crate::models::{AuthResult, SessionState};

#[derive(Debug, Deserialize)]
struct ErrorPayload {
    error: String,
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

/// Sobe a API HTTP uma unica vez por binario de teste e devolve a URL base.
async fn test_server() -> &'static str {
    static SERVER: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();
    SERVER
        .get_or_init(|| async {
            seed_http_test_env();
            crate::db::init().await;

            let app = axum::Router::new()
                .nest("/api", crate::api::router())
                .layer(axum::middleware::from_fn(crate::api::context_middleware));

            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind do listener de teste");
            let addr = listener.local_addr().expect("endereco local");
            tokio::spawn(async move {
                axum::serve(
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

async fn login(client: &reqwest::Client, base: &str, email: &str, password: &str) -> reqwest::Response {
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
    seed_user(&format!("login-{suffix}"), &email, "senha-correta-123", false).await;

    let client = client();

    let login_response = login(&client, base, &email, "senha-correta-123").await;
    assert!(login_response.status().is_success(), "login deveria ter sucesso");

    let auth_result: AuthResult = login_response.json().await.expect("corpo de login");
    assert_eq!(auth_result.user.email, email);
    assert!(!auth_result.csrf_token.is_empty());

    let current_response = client
        .get(format!("{base}/api/auth/current-user"))
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
    seed_user(&format!("logout-{suffix}"), &email, "senha-correta-123", false).await;

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
    let user_id = seed_user(&format!("admin-{suffix}"), &email, "senha-correta-123", true).await;

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
    assert!(error.error.to_lowercase().contains("senha de administrador"));

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
        .post(&reauth_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "password": "senha-correta-123" }))
        .send()
        .await
        .expect("reauth com senha correta");
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
    let user_id = seed_user(&format!("rename-{suffix}"), &email, "senha-correta-123", false).await;
    let taken_name = format!("taken{short}");
    seed_user(&taken_name, &other_email, "senha-correta-123", false).await;

    let client = client();
    let login_response = login(&client, base, &email, "senha-correta-123").await;
    let auth: AuthResult = login_response.json().await.expect("login");
    let csrf = auth.csrf_token;
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
    assert!(ok.status().is_success(), "troca de nome deveria ter sucesso");
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
    assert!(!dup.status().is_success(), "nome em uso deveria ser rejeitado");
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

/// Regra de privacidade: os palpites de um membro só ficam visíveis depois que
/// a partida começa (kickoff <= agora). Jogos no futuro não podem vazar.
#[tokio::test]
async fn pool_member_predictions_hides_matches_before_kickoff() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email_a = format!("memberA-{suffix}@teste.com");
    let email_b = format!("memberB-{suffix}@teste.com");
    let email_c = format!("outsider-{suffix}@teste.com");
    let user_a = seed_user(&format!("memberA-{suffix}"), &email_a, "senha-correta-123", false).await;
    let user_b = seed_user(&format!("memberB-{suffix}"), &email_b, "senha-correta-123", false).await;
    seed_user(&format!("outsider-{suffix}"), &email_c, "senha-correta-123", false).await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_a).await;
    add_membership(&pool_id, &user_a).await;
    add_membership(&pool_id, &user_b).await;

    let past_match = insert_match("Brasil", "Argentina", "2020-01-01T00:00:00Z").await;
    let future_match = insert_match("Franca", "Espanha", "2999-01-01T00:00:00Z").await;

    // O membro B palpitou nos dois jogos (um já iniciado, um no futuro).
    insert_prediction(&user_b, &past_match, 2, 1).await;
    insert_prediction(&user_b, &future_match, 0, 0).await;

    // Membro A consulta os palpites do bolão.
    let viewer = client();
    login(&viewer, base, &email_a, "senha-correta-123").await;
    let response = viewer
        .get(format!("{base}/api/pools/{pool_id}/member-predictions"))
        .send()
        .await
        .expect("requisicao member-predictions");
    assert!(response.status().is_success(), "membro deveria poder consultar");

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
    let outsider = client();
    login(&outsider, base, &email_c, "senha-correta-123").await;
    let denied = outsider
        .get(format!("{base}/api/pools/{pool_id}/member-predictions"))
        .send()
        .await
        .expect("requisicao de nao-membro");
    assert!(!denied.status().is_success(), "nao-membro nao deveria acessar");
}

/// Gestão de membros (admin): adicionar/remover exige admin + reautenticação
/// recente + CSRF; usuário comum é barrado.
#[tokio::test]
async fn admin_can_add_and_remove_pool_members() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin_email = format!("admin-mgmt-{suffix}@teste.com");
    let target_email = format!("target-{suffix}@teste.com");
    let admin_id = seed_user(&format!("admin-mgmt-{suffix}"), &admin_email, "senha-correta-123", true).await;
    let target_id = seed_user(&format!("target-{suffix}"), &target_email, "senha-correta-123", false).await;

    let pool_id = insert_pool(&format!("Bolao Admin {suffix}"), &admin_id).await;

    let admin = client();
    let login_response = login(&admin, base, &admin_email, "senha-correta-123").await;
    let auth: AuthResult = login_response.json().await.expect("login admin");
    let csrf = auth.csrf_token;
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
    let normal = client();
    login(&normal, base, &target_email, "senha-correta-123").await;
    let session: SessionState = normal
        .get(format!("{base}/api/auth/current-user"))
        .send()
        .await
        .expect("current-user do usuario comum")
        .json()
        .await
        .expect("sessao do usuario comum");
    let denied = normal
        .post(&add_url)
        .header("X-CSRF-Token", &session.csrf_token)
        .json(&json!({ "userId": admin_id }))
        .send()
        .await
        .expect("add por nao-admin");
    assert!(
        !denied.status().is_success(),
        "usuario comum nao deveria poder gerenciar membros"
    );
}
