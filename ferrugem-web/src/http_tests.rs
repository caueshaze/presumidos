#![cfg(all(test, feature = "server"))]

use serde::Deserialize;
use serde_json::json;
use sha2::Digest;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::{AdminSettings, AuthResult, SessionState};

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

#[tokio::test]
async fn operational_health_endpoints_and_request_id_are_safe() {
    let base = test_server().await;
    let live = client()
        .get(format!("{base}/api/health/live"))
        .send()
        .await
        .expect("liveness request");
    assert_eq!(live.status(), reqwest::StatusCode::OK);
    let request_id = live
        .headers()
        .get("x-request-id")
        .expect("request id")
        .to_str()
        .expect("request id ascii")
        .to_string();
    assert_eq!(
        live.json::<serde_json::Value>().await.expect("live json")["status"],
        "ok"
    );
    assert!(!request_id.is_empty());

    let ready = client()
        .get(format!("{base}/api/health/ready"))
        .send()
        .await
        .expect("readiness request");
    assert_eq!(ready.status(), reqwest::StatusCode::OK);
    assert_eq!(
        ready.json::<serde_json::Value>().await.expect("ready json")["status"],
        "ok"
    );
}

#[tokio::test]
async fn final_theme_setting_is_admin_controlled_public_and_persisted() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    sqlx::query(
        "UPDATE app_settings SET value='0' WHERE key IN ('final_theme_enabled','closing_screen_enabled')",
    )
    .execute(crate::db::pool())
    .await
    .expect("resetar flags do teste de tema");
    let admin_id = seed_user(
        &format!("tema-final-admin-{suffix}"),
        &format!("tema-final-admin-{suffix}@example.com"),
        "Senha-forte-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin_id).await;
    let admin = client_with_session(base, &token);

    // A configuração continua sendo protegida por reautenticação recente.
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE token = ?1")
        .bind(&token)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth recente");

    let mut settings: AdminSettings = admin
        .get(format!("{base}/api/admin/settings"))
        .send()
        .await
        .expect("ler configuracoes admin")
        .json()
        .await
        .expect("decodificar configuracoes admin");
    assert!(
        !settings.final_theme_enabled,
        "a migracao deve iniciar o tema desligado"
    );
    assert!(
        !settings.closing_screen_enabled,
        "a migracao deve iniciar a tela de encerramento desligada"
    );

    settings.final_theme_enabled = true;
    settings.closing_screen_enabled = true;
    let enabled: AdminSettings = admin
        .post(format!("{base}/api/admin/settings"))
        .header("X-CSRF-Token", &csrf)
        .json(&settings)
        .send()
        .await
        .expect("ativar tema da final")
        .json()
        .await
        .expect("decodificar resposta da ativacao");
    assert!(enabled.final_theme_enabled);
    assert!(enabled.closing_screen_enabled);

    let public_settings: AdminSettings = client()
        .get(format!("{base}/api/settings/public"))
        .send()
        .await
        .expect("ler configuracoes publicas")
        .json()
        .await
        .expect("decodificar configuracoes publicas");
    assert!(public_settings.final_theme_enabled);
    assert!(public_settings.closing_screen_enabled);

    let stored: (String,) =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = 'final_theme_enabled'")
            .fetch_one(crate::db::pool())
            .await
            .expect("ler flag persistida");
    assert_eq!(stored.0, "1");

    let closing_stored: (String,) =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = 'closing_screen_enabled'")
            .fetch_one(crate::db::pool())
            .await
            .expect("ler flag de encerramento persistida");
    assert_eq!(closing_stored.0, "1");

    let audit: (String,) = sqlx::query_as(
        "SELECT details_json FROM audit_logs
         WHERE action = 'admin_settings_updated' AND actor_user_id = ?1
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&admin_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("ler auditoria de configuracoes");
    assert!(audit.0.contains("final_theme_enabled"));
    assert!(audit.0.contains("closing_screen_enabled"));
}

#[tokio::test]
async fn ended_events_are_historical_and_admin_can_finish_legacy_edition_without_data_loss() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_user(
        &format!("legacy-admin-{suffix}"),
        &format!("legacy-admin-{suffix}@example.test"),
        "Senha-forte-123",
        true,
    )
    .await;
    let member_id = seed_user(
        &format!("legacy-member-{suffix}"),
        &format!("legacy-member-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let event_id = uuid::Uuid::new_v4().to_string();
    let pool_id = uuid::Uuid::new_v4().to_string();
    let item_id = uuid::Uuid::new_v4().to_string();
    let option_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO events (id,name,slug,kind,status,ends_at)
         VALUES (?1,'Edição legada',?2,'custom','active','2020-01-01T00:00:00Z')",
    )
    .bind(&event_id)
    .bind(format!("legacy-{suffix}"))
    .execute(crate::db::pool())
    .await
    .expect("inserir evento legado");
    sqlx::query(
        "INSERT INTO pools (id,event_id,name,invite_code,created_by) VALUES (?1,?2,'Bolão legado',?3,?4)",
    )
    .bind(&pool_id)
    .bind(&event_id)
    .bind(format!("L{suffix}")[..8].to_uppercase())
    .bind(&member_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir pool legado");
    add_membership(&pool_id, &member_id).await;
    sqlx::query(
        "INSERT INTO prediction_items (id,event_id,kind,title,lock_at,reveal_at,sort_order,status)
         VALUES (?1,?2,'single_choice','Pergunta preservada','2099-01-01T00:00:00Z','2099-01-02T00:00:00Z',0,'open')",
    )
    .bind(&item_id)
    .bind(&event_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir item legado");
    sqlx::query("INSERT INTO custom_questions (item_id,points) VALUES (?1,1)")
        .bind(&item_id)
        .execute(crate::db::pool())
        .await
        .expect("inserir pergunta legada");
    sqlx::query(
        "INSERT INTO custom_question_options (id,item_id,label,sort_order) VALUES (?1,?2,'A',0)",
    )
    .bind(&option_id)
    .bind(&item_id)
    .execute(crate::db::pool())
    .await
    .expect("inserir opção legada");

    let (member_token, member_csrf) = seed_session(&member_id).await;
    let member = client_with_session(base, &member_token);
    let before: serde_json::Value = member
        .get(format!("{base}/api/pools/dashboard"))
        .send()
        .await
        .expect("dashboard legado")
        .json()
        .await
        .expect("json dashboard legado");
    assert_eq!(before[0]["pool"]["id"], pool_id);
    assert_eq!(before[0]["pool"]["event"]["isHistorical"], true);

    let (admin_token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE token = ?1")
        .bind(&admin_token)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth");
    let admin = client_with_session(base, &admin_token);
    let finished: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/{event_id}/finish"))
        .header("X-CSRF-Token", csrf)
        .send()
        .await
        .expect("encerrar evento legado")
        .json()
        .await
        .expect("json evento encerrado");
    assert_eq!(finished["status"], "finished");

    let stored: (String,) = sqlx::query_as("SELECT status FROM events WHERE id=?1")
        .bind(&event_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("status armazenado");
    assert_eq!(stored.0, "finished");
    let pool_still_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE id=?1")
        .bind(&pool_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("pool preservado");
    assert_eq!(pool_still_exists.0, 1);
    let audit: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='event_finished' AND target_id=?1",
    )
    .bind(&event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("auditoria de encerramento");
    assert_eq!(audit.0, 1);

    let mutation = member
        .post(format!("{base}/api/custom/predictions"))
        .header("X-CSRF-Token", member_csrf)
        .json(&json!({"poolId": pool_id, "itemId": item_id, "optionId": option_id}))
        .send()
        .await
        .expect("tentar editar edição encerrada");
    assert!(!mutation.status().is_success());
    let prediction_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("palpites preservados sem criação");
    assert_eq!(prediction_count.0, 0);
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
    let event_id: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug = ?1")
        .bind(crate::events::WORLD_CUP_2026_SLUG)
        .fetch_one(crate::db::pool())
        .await
        .expect("evento da Copa seedado para fixture");
    sqlx::query("INSERT INTO pools (id, event_id, name, invite_code, created_by) VALUES (?1, ?2, ?3, ?4, ?5)")
        .bind(&id)
        .bind(&event_id.0)
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
    let prediction_item_id = insert_prediction_item(home, away, kickoff).await;
    sqlx::query(
        "INSERT INTO matches (id, prediction_item_id, home_team, away_team, kickoff, group_name, phase,
                              home_score, away_score, finished)
         VALUES (?1, ?2, ?3, ?4, ?5, 'A', 'Fase de grupos', ?6, ?7, 1)",
    )
    .bind(&id)
    .bind(&prediction_item_id)
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
    let prediction_item_id = insert_prediction_item(home, away, kickoff).await;
    sqlx::query(
        "INSERT INTO matches (id, prediction_item_id, home_team, away_team, kickoff, group_name, phase)
         VALUES (?1, ?2, ?3, ?4, ?5, 'A', 'Fase de grupos')",
    )
    .bind(&id)
    .bind(&prediction_item_id)
    .bind(home)
    .bind(away)
    .bind(kickoff)
    .execute(crate::db::pool())
    .await
    .expect("inserir partida de teste");
    id
}

async fn insert_custom_question(
    event_id: &str,
    title: &str,
    lock_at: &str,
    reveal_at: &str,
    labels: &[&str],
) -> (String, Vec<String>) {
    assert!(
        labels.len() >= 2,
        "pergunta single choice exige duas opções"
    );
    let item_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO prediction_items (id,event_id,kind,title,lock_at,reveal_at,sort_order,status) VALUES (?1,?2,'single_choice',?3,?4,?5,999,'open')")
        .bind(&item_id).bind(event_id).bind(title).bind(lock_at).bind(reveal_at).execute(crate::db::pool()).await.expect("item custom");
    sqlx::query("INSERT INTO custom_questions (item_id,points) VALUES (?1,1)")
        .bind(&item_id)
        .execute(crate::db::pool())
        .await
        .expect("pergunta custom");
    let mut ids = Vec::new();
    for (sort_order, label) in labels.iter().enumerate() {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO custom_question_options (id,item_id,label,sort_order) VALUES (?1,?2,?3,?4)").bind(&id).bind(&item_id).bind(label).bind(sort_order as i64).execute(crate::db::pool()).await.expect("opção custom");
        ids.push(id);
    }
    (item_id, ids)
}

async fn insert_custom_event_pool(owner: &str, name: &str) -> (String, String) {
    let event_id = uuid::Uuid::new_v4().to_string();
    let pool_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO events (id,name,slug,kind,status) VALUES (?1,?2,?3,'custom','active')",
    )
    .bind(&event_id)
    .bind(name)
    .bind(format!("event-{event_id}"))
    .execute(crate::db::pool())
    .await
    .expect("evento custom");
    sqlx::query(
        "INSERT INTO pools (id,event_id,name,invite_code,created_by) VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(&pool_id)
    .bind(&event_id)
    .bind(name)
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(owner)
    .execute(crate::db::pool())
    .await
    .expect("pool custom");
    add_membership(&pool_id, owner).await;
    (event_id, pool_id)
}

async fn insert_prediction_item(home: &str, away: &str, kickoff: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let event_id: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug = ?1")
        .bind(crate::events::WORLD_CUP_2026_SLUG)
        .fetch_one(crate::db::pool())
        .await
        .expect("evento Copa para fixture");
    sqlx::query(
        "INSERT INTO prediction_items
            (id, event_id, kind, title, lock_at, reveal_at, sort_order, status)
         VALUES (?1, ?2, 'football_match', ?3, ?4, ?4, 0, 'open')",
    )
    .bind(&id)
    .bind(&event_id.0)
    .bind(format!("{home} x {away}"))
    .bind(kickoff)
    .execute(crate::db::pool())
    .await
    .expect("prediction item para fixture");
    id
}

#[tokio::test]
async fn prediction_items_backfill_matches_with_world_cup_lock_and_reveal() {
    test_server().await;
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM matches m
             JOIN prediction_items pi ON pi.id = m.prediction_item_id
             JOIN events e ON e.id = pi.event_id
             WHERE e.slug = 'world-cup-2026' AND m.id LIKE 'jogo-%'),
            (SELECT COUNT(*) FROM matches m
             JOIN prediction_items pi ON pi.id = m.prediction_item_id
             JOIN events e ON e.id = pi.event_id
             WHERE m.id LIKE 'jogo-%'
               AND pi.kind = 'football_match'
               AND e.slug = 'world-cup-2026'
               AND pi.lock_at = m.kickoff
               AND pi.reveal_at = m.kickoff)",
    )
    .fetch_one(crate::db::pool())
    .await
    .expect("validar backfill de prediction items");
    assert!(counts.0 > 0, "seed atual deve conter partidas");
    assert_eq!(
        counts.0, counts.1,
        "cada match deve ter um item football da Copa"
    );
}

#[tokio::test]
async fn single_choice_is_a_real_prediction_without_match_and_respects_identity_lock_and_event() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user = seed_user(
        &format!("custom-{suffix}"),
        &format!("custom-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (event, pool) = insert_custom_event_pool(&user, "Premios").await;
    let (item, options) = insert_custom_question(
        &event,
        "Video of the Year",
        "2999-01-01T00:00:00Z",
        "2999-01-01T00:00:00Z",
        &["Artist A", "Artist B"],
    )
    .await;
    let (other_item, other_options) = insert_custom_question(
        &event,
        "Album",
        "2999-01-01T00:00:00Z",
        "2999-01-01T00:00:00Z",
        &["C", "D"],
    )
    .await;
    let (token, csrf) = seed_session(&user).await;
    let client = client_with_session(base, &token);
    let url = format!("{base}/api/custom/predictions");
    let submit = |option_id: &String| {
        client
            .post(&url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item,"optionId":option_id}))
    };
    assert_eq!(
        submit(&options[0])
            .send()
            .await
            .expect("responder")
            .status()
            .as_u16(),
        204
    );
    let stored: (String, Option<String>, Option<i64>,) = sqlx::query_as("SELECT id,match_id,home_score FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3").bind(&pool).bind(&user).bind(&item).fetch_one(crate::db::pool()).await.expect("prediction custom");
    assert!(
        stored.1.is_none() && stored.2.is_none(),
        "custom não possui Match nem placar"
    );
    let value: (String,) =
        sqlx::query_as("SELECT option_id FROM custom_prediction_values WHERE prediction_id=?1")
            .bind(&stored.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("valor custom");
    assert_eq!(value.0, options[0]);
    assert_eq!(
        crate::custom_questions::custom_prediction_value(&stored.0)
            .await
            .unwrap()
            .unwrap()
            .option_id,
        options[0]
    );
    assert_eq!(
        submit(&options[1])
            .send()
            .await
            .expect("editar")
            .status()
            .as_u16(),
        204
    );
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3",
    )
    .bind(&pool)
    .bind(&user)
    .bind(&item)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(count.0, 1);
    let changed: (String,) =
        sqlx::query_as("SELECT option_id FROM custom_prediction_values WHERE prediction_id=?1")
            .bind(&stored.0)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(changed.0, options[1]);
    assert!(!submit(&other_options[0])
        .send()
        .await
        .expect("opção errada")
        .status()
        .is_success());
    let questions: Vec<crate::models::CustomQuestion> = client
        .get(format!("{base}/api/custom/questions?poolId={pool}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(questions
        .iter()
        .any(|q| q.item_id == item && q.options.len() == 2));
    sqlx::query("UPDATE prediction_items SET lock_at='2020-01-01T00:00:00Z' WHERE id=?1")
        .bind(&item)
        .execute(crate::db::pool())
        .await
        .unwrap();
    assert!(!submit(&options[0])
        .send()
        .await
        .expect("lock")
        .status()
        .is_success());
    let football = insert_match("Brasil", "Japao", "2999-01-01T00:00:00Z").await;
    let football_item: (String,) =
        sqlx::query_as("SELECT prediction_item_id FROM matches WHERE id=?1")
            .bind(&football)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert!(!client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":pool,"itemId":football_item.0,"optionId":options[0]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    crate::custom_questions::set_correct_option(&item, &options[1])
        .await
        .expect("resultado correto da própria pergunta");
    assert!(
        crate::custom_questions::set_correct_option(&item, &other_options[0])
            .await
            .is_err()
    );
    let correct: (String,) =
        sqlx::query_as("SELECT correct_option_id FROM custom_questions WHERE item_id=?1")
            .bind(&item)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(correct.0, options[1]);
    let awarded: (i64,) = sqlx::query_as(
        "SELECT total_points FROM custom_prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND item_id=?3",
    )
    .bind(&pool)
    .bind(&user)
    .bind(&item)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(awarded.0, 1, "single choice resolvida participa do score");
    assert_ne!(item, other_item);
}

#[tokio::test]
async fn football_scoring_is_persisted_and_isolated_per_pool() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user = seed_user(
        &format!("scoring-{suffix}"),
        &format!("scoring-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let pool_a = insert_pool(&format!("A-{suffix}"), &user).await;
    let pool_b = insert_pool(&format!("B-{suffix}"), &user).await;
    add_membership(&pool_a, &user).await;
    add_membership(&pool_b, &user).await;
    let default:(i64,i64,i64,i64,i64)=sqlx::query_as("SELECT exact_score_points,correct_result_exact_side_points,correct_result_points,incorrect_result_points,knockout_bonus_points FROM football_pool_scoring WHERE pool_id=?1").bind(&pool_a).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!(default, (7, 4, 3, 0, 3));
    sqlx::query("UPDATE football_pool_scoring SET exact_score_points=15 WHERE pool_id=?1")
        .bind(&pool_b)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let match_id = insert_finished_match("Brasil", "Japao", "2020-01-01T00:00:00Z", 2, 1).await;
    let item: (String,) = sqlx::query_as("SELECT prediction_item_id FROM matches WHERE id=?1")
        .bind(&match_id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    for pool in [&pool_a, &pool_b] {
        sqlx::query("INSERT INTO predictions (id,pool_id,user_id,item_id,match_id,home_score,away_score) VALUES (?1,?2,?3,?4,?5,2,1)").bind(uuid::Uuid::new_v4().to_string()).bind(pool).bind(&user).bind(&item.0).bind(&match_id).execute(crate::db::pool()).await.unwrap();
    }
    crate::scoring::recalculate_all_breakdowns(None)
        .await
        .unwrap();
    let a:(i64,)=sqlx::query_as("SELECT total_points FROM prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND match_id=?3").bind(&pool_a).bind(&user).bind(&match_id).fetch_one(crate::db::pool()).await.unwrap();
    let b:(i64,)=sqlx::query_as("SELECT total_points FROM prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND match_id=?3").bind(&pool_b).bind(&user).bind(&match_id).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!((a.0, b.0), (7, 15));
    let (token, _csrf) = seed_session(&user).await;
    let client = client_with_session(base, &token);
    let read: crate::models::FootballScoringConfig = client
        .get(format!("{base}/api/pools/{pool_a}/scoring/football"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(read.exact_score_points, 7);
}

#[tokio::test]
async fn custom_scoring_owner_can_edit_before_lock_and_is_frozen_after() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let owner = seed_user(
        &format!("custom-score-{suffix}"),
        &format!("custom-score-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let (event, pool) = insert_custom_event_pool(&owner, "Custom score").await;
    let (item, options) = insert_custom_question(
        &event,
        "Questão",
        "2999-01-01T00:00:00Z",
        "2999-01-01T00:00:00Z",
        &["A", "B"],
    )
    .await;
    let (token, csrf) = seed_session(&owner).await;
    let client = client_with_session(base, &token);
    let url = format!("{base}/api/pools/{pool}/scoring/items/{item}");
    assert_eq!(
        client
            .post(&url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"correctPoints":5,"incorrectPoints":2}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        client
            .post(format!("{base}/api/custom/predictions"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item,"optionId":options[0]}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    crate::custom_questions::set_correct_option(&item, &options[0])
        .await
        .unwrap();
    let points:(i64,)=sqlx::query_as("SELECT total_points FROM custom_prediction_score_breakdowns WHERE pool_id=?1 AND user_id=?2 AND item_id=?3").bind(&pool).bind(&owner).bind(&item).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!(points.0, 5);
    sqlx::query("UPDATE prediction_items SET lock_at='2020-01-01T00:00:00Z' WHERE id=?1")
        .bind(&item)
        .execute(crate::db::pool())
        .await
        .unwrap();
    assert!(!client
        .post(&url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"correctPoints":9,"incorrectPoints":0}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
}

#[tokio::test]
async fn vma_manifest_imports_as_generic_custom_event_without_matches() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let _admin = seed_user(
        &format!("vma-admin-{suffix}"),
        &format!("vma-admin-{suffix}@test"),
        "senha-correta-123",
        true,
    )
    .await;
    let manifest = crate::custom_event_manifest::parse_and_validate(include_str!(
        "../data/events/vma-2026.json"
    ))
    .expect("manifesto VMA válido");
    let summary = crate::custom_event_manifest::import(manifest, true)
        .await
        .expect("importação VMA idempotente");
    assert_eq!(summary, (19, 121));
    let (event_id, actor, version_id): (String, String, String) = sqlx::query_as(
        "SELECT e.id,u.id,v.id FROM events e JOIN users u ON u.is_admin=1 JOIN event_versions v ON v.event_id=e.id AND v.state='working' WHERE e.slug='vma-2026' LIMIT 1",
    )
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_as::<_, (String,)>("SELECT status FROM events WHERE id=?1")
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap()
            .0,
        "draft"
    );
    crate::custom_event_manifest::publish_working_revision(&event_id, Some(&version_id), &actor)
        .await
        .unwrap();
    let second_manifest = crate::custom_event_manifest::parse_and_validate(include_str!(
        "../data/events/vma-2026.json"
    ))
    .unwrap();
    assert_eq!(
        crate::custom_event_manifest::import(second_manifest, true)
            .await
            .unwrap(),
        (19, 121),
        "reimportação idempotente contra a versão publicada"
    );
    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM prediction_items pi JOIN events e ON e.id=pi.event_id WHERE e.slug='vma-2026'),
           (SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id JOIN events e ON e.id=pi.event_id WHERE e.slug='vma-2026'),
           (SELECT COUNT(*) FROM matches m JOIN prediction_items pi ON pi.id=m.prediction_item_id JOIN events e ON e.id=pi.event_id WHERE e.slug='vma-2026')",
    )
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(counts, (19, 121, 0));
    let suffix = uuid::Uuid::new_v4();
    let user = seed_user(
        &format!("vma-{suffix}"),
        &format!("vma-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug='vma-2026'")
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    let pool_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO pools(id,event_id,name,invite_code,created_by) VALUES(?1,?2,?3,?4,?5)",
    )
    .bind(&pool_id)
    .bind(&event.0)
    .bind("VMA smoke")
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(&user)
    .execute(crate::db::pool())
    .await
    .unwrap();
    add_membership(&pool_id, &user).await;
    let configs: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM custom_pool_item_scoring WHERE pool_id=?1")
            .bind(&pool_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(configs.0, 19, "pool novo materializa scoring por item");

    let (token, csrf) = seed_session(&user).await;
    let client = client_with_session(base, &token);
    let pools: Vec<crate::models::PoolSummary> = client
        .get(format!("{base}/api/pools"))
        .send()
        .await
        .expect("listar pools VMA")
        .json()
        .await
        .expect("JSON de pools VMA");
    let vma_pool = pools
        .iter()
        .find(|pool| pool.id == pool_id)
        .expect("pool VMA");
    assert_eq!(vma_pool.event.kind, crate::models::EventKind::Custom);
    assert_eq!(vma_pool.event.slug, "vma-2026");

    let available: Vec<crate::models::Event> = client
        .get(format!("{base}/api/custom/events/available"))
        .send()
        .await
        .expect("listar eventos publicados")
        .json()
        .await
        .expect("JSON de eventos publicados");
    assert!(available.iter().any(|event| event.slug == "vma-2026"));

    let questions: Vec<crate::models::CustomQuestion> = client
        .get(format!("{base}/api/custom/questions?poolId={pool_id}"))
        .send()
        .await
        .expect("carregar perguntas VMA")
        .json()
        .await
        .expect("JSON de perguntas VMA");
    assert_eq!(questions.len(), 19);
    assert_eq!(
        questions
            .iter()
            .map(|question| question.options.len())
            .sum::<usize>(),
        121
    );
    assert!(questions
        .iter()
        .all(|question| question.kind == crate::models::PredictionItemKind::SingleChoice));
    assert!(questions
        .windows(2)
        .all(|pair| pair[0].sort_order <= pair[1].sort_order));
    assert_eq!(
        questions
            .iter()
            .flat_map(|question| question.options.iter())
            .filter(|option| !option.links.is_empty())
            .count(),
        4,
        "links editoriais do manifesto não duplicam nem exigem tipo especial",
    );
    let media_option = questions
        .iter()
        .flat_map(|question| question.options.iter())
        .find(|option| !option.links.is_empty())
        .unwrap();
    let media_response = client
        .post(format!("{base}/api/custom/media-progress"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":pool_id,"optionId":media_option.id,"seen":true}))
        .send()
        .await
        .unwrap();
    assert!(media_response.status().is_success());
    let progress: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM option_media_progress WHERE user_id=?1 AND option_id=?2",
    )
    .bind(&user)
    .bind(&media_option.id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(progress.0, 1);
    let predictions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&user)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_eq!(predictions.0, 0, "checklist de mídia não cria Prediction");
    let other = seed_user(
        &format!("vma-other-{suffix}"),
        &format!("vma-other-{suffix}@test"),
        "senha-correta-123",
        false,
    )
    .await;
    add_membership(&pool_id, &other).await;
    let (other_token, _) = seed_session(&other).await;
    let other_questions: Vec<crate::models::CustomQuestion> =
        client_with_session(base, &other_token)
            .get(format!("{base}/api/custom/questions?poolId={pool_id}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    assert!(
        !other_questions
            .iter()
            .flat_map(|question| question.options.iter())
            .any(|option| option.id == media_option.id && option.media_seen),
        "progresso de mídia é isolado por usuário"
    );
}

#[tokio::test]
async fn published_event_versions_preserve_old_pools_and_switch_new_ones() {
    test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin = seed_user(
        &format!("version-admin-{suffix}"),
        &format!("version-admin-{suffix}@test"),
        "senha-correta-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin).await;
    let event = crate::custom_events::create(
        token.clone(),
        "Versão 1".into(),
        Some("2099-01-01T00:00:00Z".into()),
        Some("2099-12-31T00:00:00Z".into()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let item = crate::custom_events::add_item(
        token.clone(),
        event.id.clone(),
        "Pergunta V1".into(),
        "2099-01-01T00:00:00Z".into(),
        "2099-01-02T00:00:00Z".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        item.clone(),
        "A".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    crate::custom_events::add_option(
        token.clone(),
        event.id.clone(),
        item,
        "B".into(),
        csrf.clone(),
    )
    .await
    .unwrap();
    crate::custom_events::publish(token.clone(), event.id.clone(), csrf.clone())
        .await
        .unwrap();
    let pool_one = crate::pools::create_pool_for_event(
        token.clone(),
        "Pool V1".into(),
        Some(event.id.clone()),
        csrf.clone(),
    )
    .await
    .unwrap();
    let v1: (String,) = sqlx::query_as("SELECT event_version_id FROM pools WHERE id=?1")
        .bind(&pool_one.id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();

    crate::custom_events::update_metadata(
        token.clone(),
        event.id.clone(),
        "Versão 2".into(),
        Some("2099-01-01T00:00:00Z".into()),
        Some("2099-12-31T00:00:00Z".into()),
        None,
        None,
        None,
        csrf.clone(),
    )
    .await
    .unwrap();
    let working: (String, String) =
        sqlx::query_as("SELECT id,name FROM event_versions WHERE event_id=?1 AND state='working'")
            .bind(&event.id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    assert_ne!(working.0, v1.0);
    assert_eq!(working.1, "Versão 2");
    let old_name: (String,) = sqlx::query_as(
        "SELECT v.name FROM pools p JOIN event_versions v ON v.id=p.event_version_id WHERE p.id=?1",
    )
    .bind(&pool_one.id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(old_name.0, "Versão 1");
    crate::custom_events::publish(token.clone(), event.id.clone(), csrf.clone())
        .await
        .unwrap();
    let pool_two =
        crate::pools::create_pool_for_event(token, "Pool V2".into(), Some(event.id), csrf)
            .await
            .unwrap();
    let v2: (String,) = sqlx::query_as("SELECT event_version_id FROM pools WHERE id=?1")
        .bind(&pool_two.id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert_eq!(v2.0, working.0);
    assert_ne!(v1.0, v2.0);
}

#[tokio::test]
async fn admin_manifest_preview_apply_is_idempotent_and_blocks_published_structure() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_user(
        &format!("manifest-admin-{suffix}"),
        &format!("manifest-admin-{suffix}@example.test"),
        "Senha-forte-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE token = ?1")
        .bind(&token)
        .execute(crate::db::pool())
        .await
        .expect("marcar reauth de manifesto");
    let admin = client_with_session(base, &token);
    let slug = format!("manifest-smoke-{suffix}");
    let content = serde_json::json!({
        "schemaVersion": 1,
        "name": "Manifest Smoke",
        "slug": slug,
        "kind": "custom",
        "description": "descrição inicial",
        "items": [{
            "externalKey": "best-picture",
            "kind": "single_choice",
            "title": "Best Picture",
            "lockAt": "2099-01-01T00:00:00Z",
            "revealAt": "2099-01-02T00:00:00Z",
            "options": [
                {"externalKey": "artist-a", "label": "Artist A"},
                {"externalKey": "artist-b", "label": "Artist B"}
            ]
        }]
    })
    .to_string();
    let preview: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": content.clone(), "filename": "smoke.json"}))
        .send()
        .await
        .expect("preview create")
        .json()
        .await
        .expect("json preview create");
    assert_eq!(preview["action"], "create");
    let applied_response = admin
        .post(format!("{base}/api/admin/events/import/apply"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": content.clone(), "baseFingerprint": preview["baseFingerprint"], "filename": "smoke.json"}))
        .send()
        .await
        .expect("apply create");
    let applied_status = applied_response.status();
    let applied_body = applied_response.text().await.expect("body apply create");
    assert!(
        applied_status.is_success(),
        "apply create returned {applied_status}: {applied_body}"
    );
    let applied: serde_json::Value =
        serde_json::from_str(&applied_body).expect("json apply create");
    assert_eq!(applied["action"], "create");
    let event_id = applied["eventId"].as_str().expect("event id").to_string();

    let second: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": content.clone()}))
        .send()
        .await
        .expect("preview no change")
        .json()
        .await
        .expect("json preview no change");
    assert_eq!(second["action"], "noChange");

    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .expect("publicar smoke");
    let editorial = content.replace("Manifest Smoke", "Manifest Smoke Renamed");
    let editorial_preview: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": editorial}))
        .send()
        .await
        .expect("preview editorial")
        .json()
        .await
        .expect("json preview editorial");
    assert_eq!(editorial_preview["action"], "safeUpdate");
    assert!(editorial_preview["safeChanges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"] == "Event.name"));
    let editorial_applied: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/apply"))
        .header("X-CSRF-Token", &csrf)
        .json(
            &json!({"content": editorial, "baseFingerprint": editorial_preview["baseFingerprint"]}),
        )
        .send()
        .await
        .expect("apply editorial")
        .json()
        .await
        .expect("json apply editorial");
    assert_eq!(editorial_applied["action"], "safeUpdate");
    let exported = admin
        .get(format!("{base}/api/admin/events/{event_id}/manifest"))
        .send()
        .await
        .expect("exportar manifesto");
    assert!(exported.status().is_success());
    assert!(exported
        .headers()
        .get("content-disposition")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains(&format!("{slug}.json"))));
    let exported_json: serde_json::Value = exported.json().await.expect("json exportado");
    assert_eq!(exported_json["schemaVersion"], 2);
    assert!(exported_json.get("eventId").is_none());
    let export_audit: (String,) = sqlx::query_as(
        "SELECT details_json FROM audit_logs WHERE action='event_manifest_exported' AND target_id=?1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("auditoria de export manifesto");
    assert!(export_audit.0.contains("manifestFingerprint"));
    assert!(!export_audit.0.contains("Best Picture"));
    assert!(!export_audit.0.contains("Artist A"));
    let import_audit: (String,) = sqlx::query_as(
        "SELECT details_json FROM audit_logs WHERE action='event_manifest_imported' AND target_id=?1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&event_id)
    .fetch_one(crate::db::pool())
    .await
    .expect("auditoria de import manifesto");
    assert!(import_audit.0.contains("manifestFingerprint"));
    assert!(!import_audit.0.contains("Manifest Smoke Renamed"));

    let structural = editorial.replace("Artist A", "Different Label");
    let structural_preview: serde_json::Value = admin
        .post(format!("{base}/api/admin/events/import/preview"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"content": structural}))
        .send()
        .await
        .expect("preview structural conflict")
        .json()
        .await
        .expect("json preview structural conflict");
    assert_eq!(structural_preview["action"], "safeUpdate");
    assert!(structural_preview["safeChanges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"].as_str().unwrap_or_default().contains("label")));
    let stored: (String,) =
        sqlx::query_as("SELECT name FROM event_versions WHERE event_id=?1 AND state='working'")
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("evento preservado após conflict preview");
    assert_eq!(stored.0, "Manifest Smoke Renamed");
}

#[tokio::test]
async fn manual_match_lifecycle_creates_syncs_and_deletes_prediction_item() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let admin_id = seed_user(
        &format!("item-admin-{suffix}"),
        &format!("item-admin-{suffix}@teste.com"),
        "senha-correta-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at = datetime('now') WHERE token = ?1")
        .bind(&token)
        .execute(crate::db::pool())
        .await
        .expect("reauth admin");
    let client = client_with_session(base, &token);
    let mut create_body = String::new();
    let mut create_status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
    for attempt in 0..8 {
        let response = client
            .post(format!("{base}/api/admin/matches"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({
                "homeTeam": "Brasil", "awayTeam": "Japao", "phase": "Final",
                "kickoff": "2030-07-01T20:00:00Z"
            }))
            .send()
            .await
            .expect("criar match manual");
        create_status = response.status();
        create_body = response.text().await.expect("corpo create");
        if create_status.is_success() {
            break;
        }
        if attempt < 7 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    assert!(create_status.is_success(), "criar match: {create_body}");
    let created: crate::models::MatchRecord =
        serde_json::from_str(&create_body).expect("match criado");
    let item_id: (String,) = sqlx::query_as("SELECT prediction_item_id FROM matches WHERE id = ?1")
        .bind(&created.id)
        .fetch_one(crate::db::pool())
        .await
        .expect("item do match");
    let initial: (String, String, String) =
        sqlx::query_as("SELECT title, lock_at, reveal_at FROM prediction_items WHERE id = ?1")
            .bind(&item_id.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("item criado");
    assert_eq!(
        initial,
        (
            "Brasil x Japao".into(),
            "2030-07-01T20:00:00+00:00".into(),
            "2030-07-01T20:00:00+00:00".into()
        )
    );

    let updated_kickoff = "2030-07-02T20:00:00Z";
    let updated: crate::models::MatchRecord = client
        .post(format!("{base}/api/admin/matches/{}/schedule", created.id))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "homeTeam": "Brasil", "awayTeam": "Coreia", "phase": "Final", "kickoff": updated_kickoff }))
        .send().await.expect("editar schedule").error_for_status().expect("status schedule")
        .json().await.expect("match atualizado");
    assert_eq!(updated.kickoff, "2030-07-02T20:00:00+00:00");
    let synced: (String, String, String) =
        sqlx::query_as("SELECT title, lock_at, reveal_at FROM prediction_items WHERE id = ?1")
            .bind(&item_id.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("item sincronizado");
    assert_eq!(
        synced,
        (
            "Brasil x Coreia".into(),
            updated.kickoff.clone(),
            updated.kickoff
        )
    );

    let deleted = client
        .post(format!("{base}/api/admin/matches/{}/delete", created.id))
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .expect("deletar match");
    assert!(deleted.status().is_success());
    let item_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prediction_items WHERE id = ?1")
        .bind(&item_id.0)
        .fetch_one(crate::db::pool())
        .await
        .expect("verificar item removido");
    assert_eq!(item_exists.0, 0);
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

#[tokio::test]
async fn event_builder_draft_lifecycle_enforces_owner_publish_and_pool_scoring() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = seed_user(
        &format!("builder-owner-{suffix}"),
        &format!("builder-owner-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let other_id = seed_user(
        &format!("builder-other-{suffix}"),
        &format!("builder-other-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (owner_token, owner_csrf) = seed_session(&owner_id).await;
    let (other_token, other_csrf) = seed_session(&other_id).await;
    let owner = client_with_session(base, &owner_token);
    let other = client_with_session(base, &other_token);
    let invalid = owner
        .post(format!("{base}/api/custom/events"))
        .json(&json!({"name":"Sem CSRF"}))
        .send()
        .await
        .expect("criar sem csrf");
    assert_eq!(invalid.status(), reqwest::StatusCode::FORBIDDEN);
    let event: serde_json::Value = owner.post(format!("{base}/api/custom/events")).header("X-CSRF-Token", &owner_csrf).json(&json!({"name":"Premiação Builder","startsAt":"2026-10-01T18:00:00Z","endsAt":"2026-10-01T22:00:00Z"})).send().await.expect("criar draft").json().await.expect("json do draft");
    let event_id = event["id"].as_str().expect("id").to_string();
    assert_eq!(event["status"], "draft");
    let forbidden = other.post(format!("{base}/api/custom/events/{event_id}/items")).header("X-CSRF-Token", &other_csrf).json(&json!({"title":"Invasão","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z"})).send().await.expect("tentativa de outro usuario");
    assert_eq!(forbidden.status(), reqwest::StatusCode::BAD_REQUEST);
    let item: serde_json::Value = owner.post(format!("{base}/api/custom/events/{event_id}/items")).header("X-CSRF-Token", &owner_csrf).json(&json!({"title":"Melhor café","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z"})).send().await.expect("criar pergunta").json().await.expect("json da pergunta");
    let item_id = item["id"].as_str().expect("item id");
    let mut option_ids = Vec::new();
    for label in ["Coado", "Máquina", "Solúvel"] {
        let response = owner
            .post(format!(
                "{base}/api/custom/events/{event_id}/items/{item_id}/options"
            ))
            .header("X-CSRF-Token", &owner_csrf)
            .json(&json!({"label":label}))
            .send()
            .await
            .expect("criar opcao");
        assert!(response.status().is_success());
        let value: serde_json::Value = response.json().await.expect("json da opcao");
        option_ids.push(value["id"].as_str().expect("id da opcao").to_string());
    }
    let moved = owner
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options/{}/move",
            option_ids[2]
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"direction":-1}))
        .send()
        .await
        .expect("mover opcao");
    assert!(moved.status().is_success());
    let removed = owner
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options/{}/delete",
            option_ids[1]
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("remover opcao");
    assert!(removed.status().is_success());
    let replacement = owner
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options"
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"label":"Prensa"}))
        .send()
        .await
        .expect("adicionar apos remover");
    assert!(
        replacement.status().is_success(),
        "a ordem não pode colidir após remover opção"
    );
    let published = owner
        .post(format!("{base}/api/custom/events/{event_id}/publish"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("publicar");
    assert!(published.status().is_success());
    let immutable = owner.post(format!("{base}/api/custom/events/{event_id}/items")).header("X-CSRF-Token", &owner_csrf).json(&json!({"title":"Tarde","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z"})).send().await.expect("editar publicado");
    assert_eq!(immutable.status(), reqwest::StatusCode::BAD_REQUEST);
    let active_delete = owner
        .post(format!("{base}/api/custom/events/{event_id}/delete"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("excluir ativo");
    assert_eq!(active_delete.status(), reqwest::StatusCode::BAD_REQUEST);
    let pool: serde_json::Value = owner
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"name":"Bolão do café","eventId":event_id}))
        .send()
        .await
        .expect("criar pool custom")
        .json()
        .await
        .expect("pool json");
    let scoring: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM custom_pool_item_scoring WHERE pool_id=?1")
            .bind(pool["id"].as_str().expect("pool id"))
            .fetch_one(crate::db::pool())
            .await
            .expect("scoring custom");
    assert_eq!(scoring.0, 1, "cada item publicado recebe scoring do Pool");
    let denied_result = other
        .post(format!(
            "{base}/api/admin/custom/questions/{item_id}/result"
        ))
        .header("X-CSRF-Token", &other_csrf)
        .json(&json!({"optionId":option_ids[0]}))
        .send()
        .await
        .expect("resultado por não-owner");
    assert_eq!(denied_result.status(), reqwest::StatusCode::BAD_REQUEST);
    let owner_result = owner
        .post(format!(
            "{base}/api/admin/custom/questions/{item_id}/result"
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"optionId":option_ids[0]}))
        .send()
        .await
        .expect("resultado pelo owner");
    assert!(owner_result.status().is_success());
    let audit: (String,) = sqlx::query_as("SELECT actor_user_id FROM audit_logs WHERE action='event_official_result_changed' AND target_id=?1 ORDER BY created_at DESC LIMIT 1")
        .bind(item_id).fetch_one(crate::db::pool()).await.expect("auditoria de resultado");
    assert_eq!(audit.0, owner_id);
    let disposable: serde_json::Value = owner
        .post(format!("{base}/api/custom/events"))
        .header("X-CSRF-Token", &owner_csrf)
        .json(&json!({"name":"Rascunho descartável"}))
        .send()
        .await
        .expect("criar descartável")
        .json()
        .await
        .expect("json descartável");
    let deleted = owner
        .post(format!(
            "{base}/api/custom/events/{}/delete",
            disposable["id"].as_str().expect("id descartável")
        ))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("excluir draft");
    assert!(deleted.status().is_success());
}

#[tokio::test]
async fn numeric_predictions_are_exact_per_pool_and_recalculate_without_duplicates() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = seed_user(
        &format!("numeric-owner-{suffix}"),
        &format!("numeric-owner-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&owner_id).await;
    let client = client_with_session(base, &token);
    let event: serde_json::Value = client
        .post(format!("{base}/api/custom/events"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":"Evento Numeric {suffix}"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let event_id = event["id"].as_str().unwrap();
    let invalid=client.post(format!("{base}/api/custom/events/{event_id}/items/numeric")).header("X-CSRF-Token",&csrf).json(&json!({"title":"Inválida","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z","decimalPlaces":7})).send().await.unwrap();
    assert_eq!(invalid.status(), reqwest::StatusCode::BAD_REQUEST);
    let item:serde_json::Value=client.post(format!("{base}/api/custom/events/{event_id}/items/numeric")).header("X-CSRF-Token",&csrf).json(&json!({"title":"Quantos prêmios?","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z","decimalPlaces":2,"unitLabel":"prêmios","minValue":"0","maxValue":"20"})).send().await.unwrap().json().await.unwrap();
    let item_id = item["id"].as_str().unwrap();
    let option = client
        .post(format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options"
        ))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"label":"indevida"}))
        .send()
        .await
        .unwrap();
    assert!(!option.status().is_success());
    assert!(client
        .post(format!("{base}/api/custom/events/{event_id}/publish"))
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let pool_a: serde_json::Value = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":"Numeric A {suffix}","eventId":event_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pool_a = pool_a["id"].as_str().unwrap();
    let pool_b: serde_json::Value = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":"Numeric B {suffix}","eventId":event_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pool_b = pool_b["id"].as_str().unwrap();
    for pool in [pool_a, pool_b] {
        let response = client
            .post(format!("{base}/api/custom/numeric-predictions"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item_id,"value":"7.00"}))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
    }
    let excessive = client
        .post(format!("{base}/api/custom/numeric-predictions"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":pool_a,"itemId":item_id,"value":"7.001"}))
        .send()
        .await
        .unwrap();
    assert_eq!(excessive.status(), reqwest::StatusCode::BAD_REQUEST);
    for (pool, exact, tolerance, within, incorrect) in
        [(pool_a, 5, "0", 0, 1), (pool_b, 12, "2.00", 3, 0)]
    {
        assert!(client.post(format!("{base}/api/pools/{pool}/scoring/numeric/{item_id}")).header("X-CSRF-Token",&csrf).json(&json!({"exactPoints":exact,"tolerance":tolerance,"withinTolerancePoints":within,"incorrectPoints":incorrect})).send().await.unwrap().status().is_success());
    }
    assert!(client
        .post(format!("{base}/api/admin/custom/numeric/{item_id}/result"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"value":"9"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let rows:Vec<(String,String,i64)>=sqlx::query_as("SELECT pool_id,outcome,total_points FROM numeric_prediction_score_breakdowns WHERE item_id=?1 ORDER BY pool_id").bind(item_id).fetch_all(crate::db::pool()).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_a && o == "incorrect" && *points == 1));
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_b && o == "within_tolerance" && *points == 3));
    assert!(client
        .post(format!("{base}/api/admin/custom/numeric/{item_id}/result"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"value":"7.00"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let rows:Vec<(String,String,i64)>=sqlx::query_as("SELECT pool_id,outcome,total_points FROM numeric_prediction_score_breakdowns WHERE item_id=?1 ORDER BY pool_id").bind(item_id).fetch_all(crate::db::pool()).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_a && o == "exact" && *points == 5));
    assert!(rows
        .iter()
        .any(|(p, o, points)| p == pool_b && o == "exact" && *points == 12));
    let reader_id = seed_user(
        &format!("numeric-reader-{suffix}"),
        &format!("numeric-reader-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (reader_token, reader_csrf) = seed_session(&reader_id).await;
    let reader = client_with_session(base, &reader_token);
    let invite: (String,) = sqlx::query_as("SELECT invite_code FROM pools WHERE id=?1")
        .bind(pool_a)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert!(reader
        .post(format!("{base}/api/pools/join"))
        .header("X-CSRF-Token", &reader_csrf)
        .json(&json!({"inviteCode":invite.0}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    sqlx::query("UPDATE prediction_items SET reveal_at='2000-01-01T00:00:00Z' WHERE id=?1")
        .bind(item_id)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let prediction: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3")
            .bind(pool_a)
            .bind(&owner_id)
            .bind(item_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    let reaction = reader
        .post(format!("{base}/api/pools/{pool_a}/prediction-reactions"))
        .header("X-CSRF-Token", &reader_csrf)
        .json(&json!({"targetUserId":owner_id,"predictionId":prediction.0,"emoji":"🔥"}))
        .send()
        .await
        .unwrap();
    assert!(
        reaction.status().is_success(),
        "numeric revelado aceita reação por prediction_id"
    );
    let stored: (String,) = sqlx::query_as(
        "SELECT prediction_id FROM prediction_reactions WHERE pool_id=?1 AND target_user_id=?2",
    )
    .bind(pool_a)
    .bind(&owner_id)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(stored.0, prediction.0);
}

#[tokio::test]
async fn multiple_choice_predictions_are_sets_and_score_per_pool() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner = seed_user(
        &format!("multiple-owner-{suffix}"),
        &format!("multiple-owner-{suffix}@example.com"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&owner).await;
    let client = client_with_session(base, &token);
    let event: serde_json::Value = client
        .post(format!("{base}/api/custom/events"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":format!("Multiple {suffix}")}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let event_id = event["id"].as_str().unwrap();
    let item:serde_json::Value=client.post(format!("{base}/api/custom/events/{event_id}/items/multiple-choice")).header("X-CSRF-Token",&csrf).json(&json!({"title":"Artistas","lockAt":"2026-10-01T18:00:00Z","revealAt":"2026-10-01T19:00:00Z","minSelections":1,"maxSelections":3})).send().await.unwrap().json().await.unwrap();
    let item_id = item["id"].as_str().unwrap();
    let mut options = Vec::new();
    for label in ["A", "B", "C", "D"] {
        let option: serde_json::Value = client
            .post(format!(
                "{base}/api/custom/events/{event_id}/items/{item_id}/options"
            ))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"label":label}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        options.push(option["id"].as_str().unwrap().to_string());
    }
    assert!(!client
        .post(format!("{base}/api/custom/multiple-choice-predictions"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"poolId":"nope","itemId":item_id,"optionIds":[options[0],options[0]]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert!(client
        .post(format!("{base}/api/custom/events/{event_id}/publish"))
        .header("X-CSRF-Token", &csrf)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let pool_a: serde_json::Value = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":format!("Multiple A {suffix}"),"eventId":event_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pool_a = pool_a["id"].as_str().unwrap();
    let pool_b: serde_json::Value = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"name":format!("Multiple B {suffix}"),"eventId":event_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pool_b = pool_b["id"].as_str().unwrap();
    for (pool, partial) in [(pool_a, 2), (pool_b, 6)] {
        assert!(client
            .post(format!(
                "{base}/api/pools/{pool}/scoring/multiple-choice/{item_id}"
            ))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"exactPoints":8,"partialPoints":partial,"incorrectPoints":1}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
    }
    for pool in [pool_a, pool_b] {
        assert!(client
            .post(format!("{base}/api/custom/multiple-choice-predictions"))
            .header("X-CSRF-Token", &csrf)
            .json(&json!({"poolId":pool,"itemId":item_id,"optionIds":[options[0],options[2]]}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
    }
    assert!(client
        .post(format!(
            "{base}/api/admin/custom/multiple-choice/{item_id}/result"
        ))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({"optionIds":[options[0],options[2],options[3]]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let partial:Vec<(String,String,i64)>=sqlx::query_as("SELECT pool_id,outcome,total_points FROM multiple_choice_prediction_score_breakdowns WHERE item_id=?1 ORDER BY pool_id").bind(item_id).fetch_all(crate::db::pool()).await.unwrap();
    assert!(partial
        .iter()
        .any(|(pool, outcome, points)| pool == pool_a && outcome == "partial" && *points == 2));
    assert!(partial
        .iter()
        .any(|(pool, outcome, points)| pool == pool_b && outcome == "partial" && *points == 6));
    assert!(client.post(format!("{base}/api/custom/multiple-choice-predictions")).header("X-CSRF-Token",&csrf).json(&json!({"poolId":pool_a,"itemId":item_id,"optionIds":[options[3],options[0],options[2]]})).send().await.unwrap().status().is_success());
    let stored: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3")
            .bind(pool_a)
            .bind(&owner)
            .bind(item_id)
            .fetch_one(crate::db::pool())
            .await
            .unwrap();
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM multiple_choice_prediction_options WHERE prediction_id=?1",
    )
    .bind(stored.0)
    .fetch_one(crate::db::pool())
    .await
    .unwrap();
    assert_eq!(count.0, 3);
    crate::scoring::recalculate_custom_breakdowns()
        .await
        .unwrap();
    let exact:(String,i64)=sqlx::query_as("SELECT outcome,total_points FROM multiple_choice_prediction_score_breakdowns WHERE pool_id=?1 AND item_id=?2").bind(pool_a).bind(item_id).fetch_one(crate::db::pool()).await.unwrap();
    assert_eq!(exact, ("exact".into(), 8));
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

#[tokio::test]
async fn creating_a_pool_requires_an_explicit_active_event() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user_id = seed_user(
        &format!("event-owner-{suffix}"),
        &format!("event-owner-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    let response = client
        .post(format!("{base}/api/pools"))
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "name": format!("Bolao Evento {suffix}") }))
        .send()
        .await
        .expect("tentar criar sem evento");
    assert!(!response.status().is_success());
    let payload: ErrorPayload = response.json().await.expect("erro de evento obrigatório");
    assert!(payload.error.contains("Escolha um evento publicado"));
}

/// O lock é aplicado pelo domínio ao gravar (e não apenas ocultado pela UI):
/// antes dele o palpite pode ser criado e atualizado; depois, nem a criação
/// nem a alteração passam sem uma reabertura administrativa explícita.
#[tokio::test]
async fn predictions_can_change_before_lock_and_are_rejected_after_lock() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let user_id = seed_user(
        &format!("prediction-lock-{suffix}"),
        &format!("prediction-lock-{suffix}@teste.com"),
        "senha-correta-123",
        false,
    )
    .await;
    let pool_id = insert_pool(&format!("prediction-lock-pool-{suffix}"), &user_id).await;
    add_membership(&pool_id, &user_id).await;
    let (token, csrf) = seed_session(&user_id).await;
    let client = client_with_session(base, &token);

    let future_match = insert_match("Brasil", "Japao", "2999-01-01T00:00:00Z").await;
    let prediction_url = format!("{base}/api/predictions");
    for (home_score, away_score) in [(2, 1), (3, 1)] {
        let response = client
            .post(&prediction_url)
            .header("X-CSRF-Token", &csrf)
            .json(&json!({
                "matchId": future_match,
                "homeScore": home_score,
                "awayScore": away_score,
            }))
            .send()
            .await
            .expect("enviar palpite antes do lock");
        assert_eq!(response.status().as_u16(), 204);
    }
    let future_stored: (i64, i64) = sqlx::query_as(
        "SELECT home_score, away_score FROM predictions WHERE user_id = ?1 AND match_id = ?2",
    )
    .bind(&user_id)
    .bind(&future_match)
    .fetch_one(crate::db::pool())
    .await
    .expect("ler palpite atualizado antes do lock");
    assert_eq!(future_stored, (3, 1));

    // Fixture arquitetural: football de produção mantém lock_at == kickoff,
    // mas esta divergência prova que a regra consulta o item genérico.
    sqlx::query(
        "UPDATE prediction_items SET lock_at = '2020-01-01T00:00:00Z'
         WHERE id = (SELECT prediction_item_id FROM matches WHERE id = ?1)",
    )
    .bind(&future_match)
    .execute(crate::db::pool())
    .await
    .expect("forçar lock arquitetural");
    let item_locked = client
        .post(&prediction_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "matchId": future_match, "homeScore": 4, "awayScore": 1 }))
        .send()
        .await
        .expect("palpite contra lock do item");
    assert!(
        !item_locked.status().is_success(),
        "lock_at passado deve bloquear mesmo com kickoff futuro"
    );

    let locked_match = insert_match("Franca", "Espanha", "2020-01-01T00:00:00Z").await;
    let rejected_new = client
        .post(&prediction_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "matchId": locked_match, "homeScore": 1, "awayScore": 0 }))
        .send()
        .await
        .expect("tentar criar palpite travado");
    assert!(!rejected_new.status().is_success());

    insert_prediction(&user_id, &locked_match, 0, 0).await;
    let rejected_update = client
        .post(&prediction_url)
        .header("X-CSRF-Token", &csrf)
        .json(&json!({ "matchId": locked_match, "homeScore": 4, "awayScore": 0 }))
        .send()
        .await
        .expect("tentar alterar palpite travado");
    assert!(!rejected_update.status().is_success());
    let locked_stored: (i64, i64) = sqlx::query_as(
        "SELECT home_score, away_score FROM predictions WHERE user_id = ?1 AND match_id = ?2",
    )
    .bind(&user_id)
    .bind(&locked_match)
    .fetch_one(crate::db::pool())
    .await
    .expect("ler palpite preservado apos lock");
    assert_eq!(locked_stored, (0, 0));
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

    // A suíte inteira pode deixar breakdowns materializados de testes anteriores.
    // Recalcula explicitamente para garantir isolamento deste caso.
    crate::scoring::recalculate_all_breakdowns(None)
        .await
        .expect("recalcular breakdowns");

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
    let admin_id = seed_user(
        &format!("admin-push-{suffix}"),
        &admin_email,
        "senha-correta-123",
        true,
    )
    .await;
    let user_id = seed_user(
        &format!("user-push-{suffix}"),
        &user_email,
        "senha-correta-123",
        false,
    )
    .await;

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

    if cfg!(feature = "web-push") {
        assert!(
            grouped.contains(&admin_id),
            "admin deveria receber notificacoes se ativar push"
        );
        assert!(
            grouped.contains(&user_id),
            "usuario comum deveria seguir recebendo notificacoes"
        );
    } else {
        assert!(
            grouped.is_empty(),
            "sem a feature web-push, o stub nao deve registrar subscriptions"
        );
    }
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
    let email_d = format!("late-member-{suffix}@teste.com");
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
    let user_d = seed_user(
        &format!("late-member-{suffix}"),
        &email_d,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao {suffix}"), &user_a).await;
    // Entraram no bolão antes do jogo "passado", para isolar o teste da regra de
    // elegibilidade por data de entrada (coberta em outro teste).
    add_membership_at(&pool_id, &user_a, "2019-01-01 00:00:00").await;
    add_membership_at(&pool_id, &user_b, "2019-01-01 00:00:00").await;
    // Entrou depois do jogo passado: sua participação não é retroativa, nem
    // pode vazar para os demais membros na lista de palpites.
    add_membership_at(&pool_id, &user_d, "2021-01-01 00:00:00").await;

    let past_match = insert_match("Brasil", "Argentina", "2020-01-01T00:00:00Z").await;
    let future_match = insert_match("Franca", "Espanha", "2999-01-01T00:00:00Z").await;

    // O membro B palpitou nos dois jogos (um já iniciado, um no futuro).
    insert_prediction(&user_b, &past_match, 2, 1).await;
    insert_prediction(&user_b, &future_match, 0, 0).await;
    insert_prediction(&user_d, &past_match, 1, 0).await;
    // Fixture arquitetural: reveal vem do item genérico, não do kickoff.
    sqlx::query(
        "UPDATE prediction_items SET reveal_at = '2020-01-01T00:00:00Z'
         WHERE id = (SELECT prediction_item_id FROM matches WHERE id = ?1)",
    )
    .bind(&future_match)
    .execute(crate::db::pool())
    .await
    .expect("forçar reveal arquitetural");

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
        2,
        "reveal_at do item deve liberar o palpite mesmo se o kickoff for futuro"
    );
    assert!(b
        .predictions
        .iter()
        .any(|prediction| prediction.match_id == past_match));
    assert!(b
        .predictions
        .iter()
        .any(|prediction| prediction.match_id == future_match));

    let late_member = members
        .iter()
        .find(|m| m.user_id == user_d)
        .expect("membro que entrou depois presente na resposta");
    assert!(
        late_member.predictions.is_empty(),
        "palpite de jogo anterior a entrada não deve ser exposto retroativamente"
    );

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
async fn prediction_reactions_can_be_created_changed_removed_and_seen() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email_a = format!("reactor-{suffix}@teste.com");
    let email_b = format!("target-{suffix}@teste.com");
    let user_a = seed_user(
        &format!("reactor-{suffix}"),
        &email_a,
        "senha-correta-123",
        false,
    )
    .await;
    let user_b = seed_user(
        &format!("target-{suffix}"),
        &email_b,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao React {suffix}"), &user_a).await;
    add_membership_at(&pool_id, &user_a, "2019-01-01 00:00:00").await;
    add_membership_at(&pool_id, &user_b, "2019-01-01 00:00:00").await;

    let match_id = insert_match("Brasil", "Argentina", "2020-01-01T00:00:00Z").await;
    insert_prediction(&user_b, &match_id, 7, 0).await;

    let (token_a, csrf_a) = seed_session(&user_a).await;
    let reactor = client_with_session(base, &token_a);
    let react_url = format!("{base}/api/pools/{pool_id}/prediction-reactions");

    let created = reactor
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_b, "matchId": match_id, "emoji": "😂" }))
        .send()
        .await
        .expect("criar reacao");
    assert!(created.status().is_success(), "criacao deveria ter sucesso");

    let (token_b, csrf_b) = seed_session(&user_b).await;
    let target = client_with_session(base, &token_b);
    let list_url = format!("{base}/api/pools/{pool_id}/member-predictions");
    let members: Vec<crate::models::MemberPredictions> = target
        .get(&list_url)
        .send()
        .await
        .expect("listar palpites apos reacao")
        .json()
        .await
        .expect("corpo member-predictions");
    let target_entry = members
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("membro alvo presente");
    let prediction = target_entry
        .predictions
        .iter()
        .find(|p| p.match_id == match_id)
        .expect("palpite do alvo presente");
    assert_eq!(target_entry.unread_reaction_count, 1);
    assert_eq!(prediction.unread_reaction_count, 1);
    assert_eq!(prediction.reactions.len(), 1);
    assert_eq!(prediction.reactions[0].emoji, "😂");
    assert_eq!(prediction.reactions[0].count, 1);

    let seen = target
        .post(format!(
            "{base}/api/pools/{pool_id}/prediction-reactions/mark-seen"
        ))
        .header("X-CSRF-Token", &csrf_b)
        .send()
        .await
        .expect("marcar reacoes como vistas");
    assert!(seen.status().is_success(), "mark-seen deveria ter sucesso");

    let after_seen: Vec<crate::models::MemberPredictions> = target
        .get(&list_url)
        .send()
        .await
        .expect("listar apos mark-seen")
        .json()
        .await
        .expect("corpo apos mark-seen");
    let target_after_seen = after_seen
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("alvo apos mark-seen");
    assert_eq!(target_after_seen.unread_reaction_count, 0);

    let changed = reactor
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_b, "matchId": match_id, "emoji": "🔥" }))
        .send()
        .await
        .expect("trocar reacao");
    assert!(changed.status().is_success(), "troca deveria ter sucesso");

    let removed = reactor
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_b, "matchId": match_id, "emoji": "🔥" }))
        .send()
        .await
        .expect("remover reacao");
    assert!(removed.status().is_success(), "remocao deveria ter sucesso");

    let after_remove: Vec<crate::models::MemberPredictions> = target
        .get(&list_url)
        .send()
        .await
        .expect("listar apos remover")
        .json()
        .await
        .expect("corpo apos remover");
    let target_after_remove = after_remove
        .iter()
        .find(|m| m.user_id == user_b)
        .expect("alvo apos remover");
    let prediction_after_remove = target_after_remove
        .predictions
        .iter()
        .find(|p| p.match_id == match_id)
        .expect("palpite apos remover");
    assert!(prediction_after_remove.reactions.is_empty());
}

#[tokio::test]
async fn prediction_reactions_reject_self_reaction_and_non_member() {
    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4();
    let email_a = format!("self-react-{suffix}@teste.com");
    let email_b = format!("other-react-{suffix}@teste.com");
    let email_c = format!("outsider-react-{suffix}@teste.com");
    let user_a = seed_user(
        &format!("self-react-{suffix}"),
        &email_a,
        "senha-correta-123",
        false,
    )
    .await;
    let user_b = seed_user(
        &format!("other-react-{suffix}"),
        &email_b,
        "senha-correta-123",
        false,
    )
    .await;
    let user_c = seed_user(
        &format!("outsider-react-{suffix}"),
        &email_c,
        "senha-correta-123",
        false,
    )
    .await;

    let pool_id = insert_pool(&format!("Bolao React Guard {suffix}"), &user_a).await;
    add_membership_at(&pool_id, &user_a, "2019-01-01 00:00:00").await;
    add_membership_at(&pool_id, &user_b, "2019-01-01 00:00:00").await;
    let match_id = insert_match("Italia", "Alemanha", "2020-01-01T00:00:00Z").await;
    insert_prediction(&user_a, &match_id, 0, 0).await;

    let react_url = format!("{base}/api/pools/{pool_id}/prediction-reactions");

    let (token_a, csrf_a) = seed_session(&user_a).await;
    let self_client = client_with_session(base, &token_a);
    let self_reaction = self_client
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_a)
        .json(&json!({ "targetUserId": user_a, "matchId": match_id, "emoji": "😂" }))
        .send()
        .await
        .expect("auto-reacao");
    assert!(!self_reaction.status().is_success());

    let (token_c, csrf_c) = seed_session(&user_c).await;
    let outsider = client_with_session(base, &token_c);
    let outsider_reaction = outsider
        .post(&react_url)
        .header("X-CSRF-Token", &csrf_c)
        .json(&json!({ "targetUserId": user_a, "matchId": match_id, "emoji": "😂" }))
        .send()
        .await
        .expect("reacao por nao-membro");
    assert!(!outsider_reaction.status().is_success());
}

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

#[tokio::test]
async fn contextual_asset_upload_deduplicates_and_enforces_draft_privacy() {
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;

    let base = test_server().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner = seed_user(
        &format!("asset-owner-{suffix}"),
        &format!("asset-owner-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let other = seed_user(
        &format!("asset-other-{suffix}"),
        &format!("asset-other-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let event_id = uuid::Uuid::new_v4().to_string();
    let slug = format!("asset-smoke-{suffix}");
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by) VALUES(?1,?2,?3,'custom','draft',?4)")
        .bind(&event_id)
        .bind("Asset smoke")
        .bind(&slug)
        .bind(&owner)
        .execute(crate::db::pool())
        .await
        .expect("evento draft de asset");
    let (item_id, options) = insert_custom_question(
        &event_id,
        "Imagem editorial",
        "2099-01-01T00:00:00Z",
        "2099-01-02T00:00:00Z",
        &["A", "B"],
    )
    .await;
    let (owner_token, owner_csrf) = seed_session(&owner).await;
    let (other_token, other_csrf) = seed_session(&other).await;
    let owner_client = client_with_session(base, &owner_token);
    let other_client = client_with_session(base, &other_token);

    let fixture_digest = sha2::Sha256::digest(suffix.as_bytes());
    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
        24,
        12,
        Rgb([fixture_digest[0], fixture_digest[1], fixture_digest[2]]),
    ));
    let mut image_bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut image_bytes, ImageOutputFormat::Png)
        .expect("fixture png");
    let png = image_bytes.into_inner();

    let upload = |path: String, csrf: String, bytes: Vec<u8>| async {
        owner_client
            .post(path)
            .header("X-CSRF-Token", csrf)
            .multipart(
                reqwest::multipart::Form::new().part(
                    "file",
                    reqwest::multipart::Part::bytes(bytes)
                        .file_name("editorial.png")
                        .mime_str("image/png")
                        .expect("mime png"),
                ),
            )
            .send()
            .await
            .expect("upload de asset")
    };
    let cover = upload(
        format!("{base}/api/custom/events/{event_id}/cover"),
        owner_csrf.clone(),
        png.clone(),
    )
    .await;
    assert!(cover.status().is_success(), "upload cover");
    let cover_asset: crate::assets::AssetResponse = cover.json().await.expect("asset cover");

    let denied = other_client
        .post(format!("{base}/api/custom/events/{event_id}/cover"))
        .header("X-CSRF-Token", &other_csrf)
        .multipart(
            reqwest::multipart::Form::new().part(
                "file",
                reqwest::multipart::Part::bytes(png.clone())
                    .file_name("editorial.png")
                    .mime_str("image/png")
                    .unwrap(),
            ),
        )
        .send()
        .await
        .expect("upload negado");
    assert!(!denied.status().is_success(), "outro dono não edita draft");

    let option_upload = upload(
        format!(
            "{base}/api/custom/events/{event_id}/items/{item_id}/options/{}/image",
            options[0]
        ),
        owner_csrf.clone(),
        png.clone(),
    )
    .await;
    assert!(option_upload.status().is_success(), "upload option");
    let option_asset: crate::assets::AssetResponse = option_upload.json().await.unwrap();
    assert_eq!(
        cover_asset.asset_id, option_asset.asset_id,
        "dedup por hash"
    );
    let assets_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets WHERE sha256=?1")
        .bind(&cover_asset.sha256)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert_eq!(assets_count.0, 1);

    let private = other_client
        .get(format!("{base}{}", cover_asset.variants["cover"]))
        .send()
        .await
        .expect("serving draft privado");
    assert!(!private.status().is_success());
    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .unwrap();
    let public = other_client
        .get(format!("{base}{}", cover_asset.variants["cover"]))
        .send()
        .await
        .expect("serving publicado");
    assert_eq!(public.status(), reqwest::StatusCode::OK);
    assert_eq!(public.headers()["content-type"], "image/webp");
    assert!(public.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("immutable"));

    let removed = owner_client
        .post(format!("{base}/api/custom/events/{event_id}/cover/remove"))
        .header("X-CSRF-Token", &owner_csrf)
        .send()
        .await
        .expect("remover cover");
    assert!(removed.status().is_success());
    let still_stored: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets WHERE id=?1")
        .bind(&cover_asset.asset_id)
        .fetch_one(crate::db::pool())
        .await
        .unwrap();
    assert_eq!(still_stored.0, 1, "remove desassocia, não apaga asset");
}

#[tokio::test]
async fn contextual_asset_upload_http_smoke_without_tcp() {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;
    use std::net::SocketAddr;
    use tower::ServiceExt;

    test_server().await;
    let app = axum::Router::new()
        .nest("/api", crate::api::router())
        .route(
            "/media/assets/{asset_id}/{variant}",
            axum::routing::get(crate::api::media_asset),
        )
        .layer(axum::middleware::from_fn(crate::api::context_middleware));
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner = seed_user(
        &format!("in-process-owner-{suffix}"),
        &format!("in-process-owner-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let other = seed_user(
        &format!("in-process-other-{suffix}"),
        &format!("in-process-other-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let event_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO events(id,name,slug,kind,status,created_by) VALUES(?1,?2,?3,'custom','draft',?4)")
        .bind(&event_id)
        .bind("In-process asset smoke")
        .bind(format!("in-process-asset-{suffix}"))
        .bind(&owner)
        .execute(crate::db::pool())
        .await
        .expect("evento in-process");
    let (item_id, options) = insert_custom_question(
        &event_id,
        "Imagem editorial",
        "2099-01-01T00:00:00Z",
        "2099-01-02T00:00:00Z",
        &["A", "B"],
    )
    .await;
    let (owner_token, owner_csrf) = seed_session(&owner).await;
    let (other_token, _) = seed_session(&other).await;
    let fixture_digest = sha2::Sha256::digest(suffix.as_bytes());
    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
        24,
        12,
        Rgb([fixture_digest[0], fixture_digest[1], fixture_digest[2]]),
    ));
    let mut encoded = Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, ImageOutputFormat::Png)
        .expect("png fixture");
    let png = encoded.into_inner();
    let boundary = "asset-smoke-boundary";
    let multipart = |bytes: &[u8]| {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"editorial.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        body
    };
    let request = |path: String, token: &str, csrf: &str, body: Vec<u8>| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(body))
            .expect("request multipart")
    };
    let cover = app
        .clone()
        .oneshot(request(
            format!("/api/custom/events/{event_id}/cover"),
            &owner_token,
            &owner_csrf,
            multipart(&png),
        ))
        .await
        .expect("upload cover in-process");
    assert_eq!(cover.status(), StatusCode::OK);
    let cover_body = to_bytes(cover.into_body(), 1_000_000)
        .await
        .expect("cover response body");
    let cover_asset: crate::assets::AssetResponse =
        serde_json::from_slice(&cover_body).expect("cover asset json");

    let option = app
        .clone()
        .oneshot(request(
            format!(
                "/api/custom/events/{event_id}/items/{item_id}/options/{}/image",
                options[0]
            ),
            &owner_token,
            &owner_csrf,
            multipart(&png),
        ))
        .await
        .expect("upload option in-process");
    assert_eq!(option.status(), StatusCode::OK);
    let option_body = to_bytes(option.into_body(), 1_000_000)
        .await
        .expect("option response body");
    let option_asset: crate::assets::AssetResponse =
        serde_json::from_slice(&option_body).expect("option asset json");
    assert_eq!(cover_asset.asset_id, option_asset.asset_id);

    let invalid = app
        .clone()
        .oneshot(request(
            format!("/api/custom/events/{event_id}/cover"),
            &owner_token,
            &owner_csrf,
            multipart(b"not-an-image"),
        ))
        .await
        .expect("tentativa de substituir cover com arquivo inválido");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let cover_after_invalid: (Option<String>,) =
        sqlx::query_as("SELECT cover_asset_id FROM events WHERE id=?1")
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("cover preservada após upload inválido");
    assert_eq!(
        cover_after_invalid.0.as_deref(),
        Some(cover_asset.asset_id.as_str())
    );

    let private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&cover_asset.variants["cover"])
                .header(
                    "Cookie",
                    format!("{}={other_token}", crate::security::session_cookie_name()),
                )
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("request private asset"),
        )
        .await
        .expect("private asset response");
    assert_eq!(private.status(), StatusCode::BAD_REQUEST);

    sqlx::query("UPDATE events SET status='active' WHERE id=?1")
        .bind(&event_id)
        .execute(crate::db::pool())
        .await
        .expect("publicar evento in-process");
    let public = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&cover_asset.variants["cover"])
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("request public asset"),
        )
        .await
        .expect("public asset response");
    assert_eq!(public.status(), StatusCode::OK);
    assert_eq!(public.headers()["content-type"], "image/webp");
    assert!(public.headers()["cache-control"]
        .to_str()
        .expect("cache header")
        .contains("immutable"));

    let removed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/custom/events/{event_id}/cover/remove"))
                .header(
                    "Cookie",
                    format!("{}={owner_token}", crate::security::session_cookie_name()),
                )
                .header("X-CSRF-Token", &owner_csrf)
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("remover cover publicada pelo dono"),
        )
        .await
        .expect("remover cover publicada");
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn legacy_manifest_v1_import_works_without_tcp() {
    test_server().await;
    let slug = format!("legacy-v1-smoke-{}", uuid::Uuid::new_v4().simple());
    let content = serde_json::json!({
        "schemaVersion": 1,
        "name": "Legacy sem imagens",
        "slug": slug,
        "kind": "custom",
        "items": [{
            "externalKey": "choice",
            "kind": "single_choice",
            "title": "Escolha",
            "lockAt": "2099-01-01T00:00:00Z",
            "revealAt": "2099-01-02T00:00:00Z",
            "options": [
                {"externalKey": "a", "label": "A"},
                {"externalKey": "b", "label": "B"}
            ]
        }]
    })
    .to_string();
    let manifest =
        crate::custom_event_manifest::parse_and_validate(&content).expect("manifest v1 legado");
    assert_eq!(
        crate::custom_event_manifest::import(manifest, false)
            .await
            .unwrap(),
        (1, 2)
    );
    assert_eq!(
        crate::custom_event_manifest::import(
            crate::custom_event_manifest::parse_and_validate(&content).unwrap(),
            true,
        )
        .await
        .unwrap(),
        (1, 2)
    );
    let state: (String, i64, i64) = sqlx::query_as("SELECT e.status,(SELECT COUNT(*) FROM prediction_items pi JOIN event_versions v ON v.id=pi.event_version_id WHERE v.event_id=e.id AND v.state='working'),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id JOIN event_versions v ON v.id=pi.event_version_id WHERE v.event_id=e.id AND v.state='working') FROM events e WHERE e.slug=?1")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("estado do import legado");
    assert_eq!(state, ("draft".into(), 1, 2));
    assert_eq!(
        crate::custom_event_manifest::import(
            crate::custom_event_manifest::parse_and_validate(&content).unwrap(),
            true,
        )
        .await
        .unwrap(),
        (1, 2)
    );
    let event_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE slug=?1")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("idempotência do import legado");
    assert_eq!(event_count.0, 1);
}

fn package_smoke_manifest(name: &str, slug: &str, asset_sha: &str) -> String {
    serde_json::json!({
        "schemaVersion": 2,
        "name": name,
        "slug": slug,
        "kind": "custom",
        "description": "Evento de promoção com imagem interna.",
        "externalUrl": "https://example.test/evento",
        "startsAt": "2099-01-01T00:00:00Z",
        "endsAt": "2099-01-02T00:00:00Z",
        "coverAsset": {"kind": "asset", "sha256": asset_sha, "mediaType": "image/webp"},
        "items": [
            {
                "externalKey": "choice",
                "kind": "single_choice",
                "title": "Escolha",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "options": [
                    {"externalKey": "a", "label": "A", "imageAsset": {"kind": "asset", "sha256": asset_sha, "mediaType": "image/webp"}, "links": [{"kind": "official", "label": "Site oficial", "url": "https://example.test/a"}]},
                    {"externalKey": "b", "label": "B"}
                ]
            },
            {
                "externalKey": "number",
                "kind": "numeric",
                "title": "Número",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "decimalPlaces": 1,
                "unitLabel": "pontos",
                "minValue": "0.0",
                "maxValue": "10.0"
            },
            {
                "externalKey": "multiple",
                "kind": "multiple_choice",
                "title": "Múltipla",
                "lockAt": "2099-01-01T00:00:00Z",
                "revealAt": "2099-01-02T00:00:00Z",
                "minSelections": 1,
                "maxSelections": 2,
                "options": [
                    {"externalKey": "x", "label": "X"},
                    {"externalKey": "y", "label": "Y"}
                ]
            }
        ]
    })
    .to_string()
}

fn vma_smoke_manifest(asset_sha: &str) -> crate::custom_event_manifest::CustomEventManifest {
    let mut manifest = crate::custom_event_manifest::parse_and_validate(include_str!(
        "../data/events/vma-2026.json"
    ))
    .expect("manifesto VMA do smoke");
    manifest.schema_version = crate::custom_event_manifest::CURRENT_SCHEMA_VERSION;
    let asset = crate::custom_event_manifest::AssetReference {
        kind: "asset".into(),
        sha256: asset_sha.into(),
        media_type: "image/webp".into(),
    };
    manifest.cover_asset = Some(asset.clone());
    for item in &mut manifest.items {
        if matches!(
            item.external_key.as_str(),
            "video-of-the-year" | "best-pop" | "best-k-pop"
        ) {
            if let Some(option) = item.options.first_mut() {
                option.image_asset = Some(asset.clone());
            }
        }
    }
    manifest
}

fn package_smoke_master(rgb: [u8; 3]) -> Vec<u8> {
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb};
    use std::io::Cursor;

    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(24, 12, Rgb(rgb)));
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageOutputFormat::WebP)
        .expect("master de smoke");
    output.into_inner()
}

fn package_smoke_root() -> PathBuf {
    PathBuf::from(
        std::env::var("PACKAGE_SMOKE_ROOT")
            .expect("PACKAGE_SMOKE_ROOT obrigatório no smoke de pacote"),
    )
}

fn package_smoke_env(db: &Path, assets: &Path) {
    seed_http_test_env();
    std::env::set_var("DATABASE_PATH", db.to_string_lossy().to_string());
    std::env::set_var("PRESUMIDOS_ASSET_DIR", assets.to_string_lossy().to_string());
}

fn package_smoke_package(root: &Path, name: &str) -> Vec<u8> {
    fs::read(root.join(name)).expect("pacote de smoke")
}

fn package_smoke_structural_view(
    mut manifest: crate::custom_event_manifest::CustomEventManifest,
) -> serde_json::Value {
    manifest.name.clear();
    manifest.description = None;
    manifest.cover_url = None;
    manifest.cover_asset = None;
    manifest.external_url = None;
    for item in &mut manifest.items {
        for option in &mut item.options {
            option.image_url = None;
            option.image_asset = None;
            option.links.clear();
        }
    }
    serde_json::to_value(manifest).expect("estrutura de manifesto")
}

fn package_smoke_entry_names(bytes: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip smoke");
    let mut names = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .expect("entrada zip")
                .name()
                .to_string()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn package_smoke_without_assets(slug: &str) -> Vec<u8> {
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "name": "Pacote HTTP",
        "slug": slug,
        "kind": "custom",
        "items": [{
            "externalKey": "choice",
            "kind": "single_choice",
            "title": "Escolha",
            "lockAt": "2099-01-01T00:00:00Z",
            "revealAt": "2099-01-02T00:00:00Z",
            "options": [{"externalKey": "a", "label": "A"}, {"externalKey": "b", "label": "B"}]
        }]
    })
    .to_string();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file(
            "manifest.json",
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .expect("manifest HTTP");
    std::io::Write::write_all(&mut writer, manifest.as_bytes()).expect("manifest bytes HTTP");
    writer.finish().expect("zip HTTP").into_inner()
}

#[tokio::test]
async fn admin_package_http_flow_works_without_tcp() {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use std::net::SocketAddr;
    use tower::ServiceExt;

    test_server().await;
    let app = axum::Router::new()
        .nest("/api", crate::api::router())
        .route(
            "/media/assets/{asset_id}/{variant}",
            axum::routing::get(crate::api::media_asset),
        )
        .layer(axum::middleware::from_fn(crate::api::context_middleware));
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_user(
        &format!("package-http-admin-{suffix}"),
        &format!("package-http-admin-{suffix}@example.test"),
        "Senha-forte-123",
        true,
    )
    .await;
    let (token, csrf) = seed_session(&admin_id).await;
    sqlx::query("UPDATE sessions SET admin_reauthed_at=datetime('now') WHERE token=?1")
        .bind(&token)
        .execute(crate::db::pool())
        .await
        .expect("reauth package HTTP");
    let slug = format!("pacote-http-{}", suffix);
    let package = package_smoke_without_assets(&slug);
    let boundary = "package-http-boundary";
    let multipart = |base_fingerprint: Option<&str>| {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"pacote-http.zip\"\r\nContent-Type: application/zip\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(&package);
        if let Some(fingerprint) = base_fingerprint {
            body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"baseFingerprint\"\r\n\r\n{fingerprint}\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        } else {
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        }
        body
    };
    let send = |path: &str, body: Vec<u8>| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", &csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(body))
            .expect("request package HTTP")
    };
    let preview = app
        .clone()
        .oneshot(send(
            "/api/admin/events/import/package/preview",
            multipart(None),
        ))
        .await
        .expect("preview package HTTP");
    assert_eq!(preview.status(), StatusCode::OK);
    let preview_body = to_bytes(preview.into_body(), 1_000_000)
        .await
        .expect("preview body HTTP");
    let preview_json: serde_json::Value = serde_json::from_slice(&preview_body).unwrap();
    assert_eq!(preview_json["manifest"]["action"], "create");
    assert_eq!(preview_json["manifest"]["itemCount"], 1);
    let base_fingerprint = preview_json["manifest"]["baseFingerprint"]
        .as_str()
        .expect("base fingerprint HTTP")
        .to_string();
    let before_apply: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM events WHERE slug=?1),(SELECT COUNT(*) FROM prediction_items pi JOIN events e ON e.id=pi.event_id WHERE e.slug=?1),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id JOIN events e ON e.id=pi.event_id WHERE e.slug=?1)")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("contagens antes do apply HTTP");
    assert_eq!(before_apply, (0, 0, 0));
    let applied = app
        .clone()
        .oneshot(send(
            "/api/admin/events/import/package/apply",
            multipart(Some(&base_fingerprint)),
        ))
        .await
        .expect("apply package HTTP");
    assert_eq!(applied.status(), StatusCode::OK);
    let applied_body = to_bytes(applied.into_body(), 1_000_000)
        .await
        .expect("apply body HTTP");
    let applied_json: serde_json::Value = serde_json::from_slice(&applied_body).unwrap();
    assert_eq!(applied_json["result"]["action"], "create");
    let created_status: (String,) = sqlx::query_as("SELECT status FROM events WHERE slug=?1")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("status do evento importado HTTP");
    assert_eq!(created_status.0, "draft");
    let repeat = app
        .clone()
        .oneshot(send(
            "/api/admin/events/import/package/preview",
            multipart(None),
        ))
        .await
        .expect("repeat preview package HTTP");
    let repeat_body = to_bytes(repeat.into_body(), 1_000_000)
        .await
        .expect("repeat body HTTP");
    let repeat_json: serde_json::Value = serde_json::from_slice(&repeat_body).unwrap();
    assert_eq!(repeat_json["manifest"]["action"], "noChange");
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM events WHERE slug=?1),(SELECT COUNT(*) FROM prediction_items WHERE event_id=(SELECT id FROM events WHERE slug=?1)),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=(SELECT id FROM events WHERE slug=?1))")
        .bind(&slug)
        .fetch_one(crate::db::pool())
        .await
        .expect("contagens package HTTP");
    assert_eq!(counts, (1, 1, 2));
    let events = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/events")
                .header(
                    "Cookie",
                    format!("{}={token}", crate::security::session_cookie_name()),
                )
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("listar eventos no Admin"),
        )
        .await
        .expect("resposta de eventos no Admin");
    assert_eq!(events.status(), StatusCode::OK);
    let events_body = to_bytes(events.into_body(), 1_000_000)
        .await
        .expect("corpo de eventos no Admin");
    let events_json: serde_json::Value = serde_json::from_slice(&events_body).unwrap();
    let admin_event = events_json
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["slug"] == slug)
        .expect("evento de pacote na lista Admin");
    let expected_username = format!("package-http-admin-{suffix}");
    assert_eq!(
        admin_event["createdByUsername"].as_str(),
        Some(expected_username.as_str())
    );
    assert_eq!(admin_event["itemCount"], 1);
    assert_eq!(admin_event["optionCount"], 2);
    assert_eq!(admin_event["poolCount"], 0);
    assert!(admin_event["updatedAt"].as_str().is_some());
}

#[tokio::test]
async fn user_event_builder_asset_pool_flow_works_without_tcp() {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use std::net::SocketAddr;
    use tower::ServiceExt;

    test_server().await;
    let app = axum::Router::new()
        .nest("/api", crate::api::router())
        .route(
            "/media/assets/{asset_id}/{variant}",
            axum::routing::get(crate::api::media_asset),
        )
        .layer(axum::middleware::from_fn(crate::api::context_middleware));
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = seed_user(
        &format!("builder-asset-user-{suffix}"),
        &format!("builder-asset-user-{suffix}@example.test"),
        "Senha-forte-123",
        false,
    )
    .await;
    let (token, csrf) = seed_session(&user).await;
    let json_request = |path: String, payload: serde_json::Value| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/json")
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", &csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(payload.to_string()))
            .expect("request JSON builder")
    };
    let response_json = |response: axum::response::Response| async move {
        let status = response.status();
        let body = to_bytes(response.into_body(), 2_000_000)
            .await
            .expect("body JSON builder");
        assert!(
            status.is_success(),
            "resposta builder: {status} {}",
            String::from_utf8_lossy(&body)
        );
        serde_json::from_slice::<serde_json::Value>(&body).expect("JSON builder")
    };
    let created = response_json(
        app.clone()
            .oneshot(json_request(
                "/api/custom/events".into(),
                json!({"name":"Evento com imagens","startsAt":null,"endsAt":null}),
            ))
            .await
            .expect("criar event builder"),
    )
    .await;
    let event_id = created["id"]
        .as_str()
        .expect("id event builder")
        .to_string();
    let lock = "2099-01-01T00:00:00Z";
    let reveal = "2099-01-02T00:00:00Z";
    let add_item = |path: String, payload: serde_json::Value| async {
        response_json(
            app.clone()
                .oneshot(json_request(path, payload))
                .await
                .expect("add item builder"),
        )
        .await["id"]
            .as_str()
            .expect("id item builder")
            .to_string()
    };
    let single_id = add_item(
        format!("/api/custom/events/{event_id}/items"),
        json!({"title":"Escolha","lockAt":lock,"revealAt":reveal}),
    )
    .await;
    let numeric_id = add_item(
        format!("/api/custom/events/{event_id}/items/numeric"),
        json!({"title":"Número","lockAt":lock,"revealAt":reveal,"decimalPlaces":1,"unitLabel":"pontos","minValue":"0.0","maxValue":"10.0"}),
    )
    .await;
    let multiple_id = add_item(
        format!("/api/custom/events/{event_id}/items/multiple-choice"),
        json!({"title":"Múltipla","lockAt":lock,"revealAt":reveal,"minSelections":1,"maxSelections":2}),
    )
    .await;
    let add_option = |item_id: String, label: String| {
        let app = app.clone();
        let request = json_request(
            format!("/api/custom/events/{event_id}/items/{item_id}/options"),
            json!({"label": label}),
        );
        async move {
            let response = app.oneshot(request).await.expect("add option builder");
            let status = response.status();
            let body = to_bytes(response.into_body(), 2_000_000)
                .await
                .expect("option body builder");
            assert!(
                status.is_success(),
                "resposta option builder: {status} {}",
                String::from_utf8_lossy(&body)
            );
            let value: serde_json::Value =
                serde_json::from_slice(&body).expect("option JSON builder");
            value["id"].as_str().expect("id option builder").to_string()
        }
    };
    let single_option = add_option(single_id.clone(), "A".into()).await;
    add_option(single_id.clone(), "B".into()).await;
    add_option(multiple_id.clone(), "X".into()).await;
    add_option(multiple_id, "Y".into()).await;

    let image = package_smoke_master([20, 40, 80]);
    let boundary = "builder-asset-boundary";
    let multipart = |bytes: &[u8]| {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"cover.webp\"\r\nContent-Type: image/webp\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        body
    };
    let upload_request = |path: String| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", &csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(multipart(&image)))
            .expect("upload builder")
    };
    let cover = app
        .clone()
        .oneshot(upload_request(format!(
            "/api/custom/events/{event_id}/cover"
        )))
        .await
        .expect("upload cover builder");
    assert_eq!(cover.status(), StatusCode::OK);
    let option_image = app
        .clone()
        .oneshot(upload_request(format!(
            "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/image"
        )))
        .await
        .expect("upload option builder");
    assert_eq!(option_image.status(), StatusCode::OK);

    let publish = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/custom/events/{event_id}/publish"))
                .header(
                    "Cookie",
                    format!("{}={token}", crate::security::session_cookie_name()),
                )
                .header("X-CSRF-Token", &csrf)
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("publish builder"),
        )
        .await
        .expect("publicar builder");
    assert_eq!(publish.status(), StatusCode::NO_CONTENT);
    let published_media_update = app
        .clone()
        .oneshot(json_request(
            format!(
                "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/media"
            ),
            json!({"imageUrl":"https://example.test/published-option.webp","links":[]}),
        ))
        .await
        .expect("editar mídia editorial publicada");
    assert_eq!(published_media_update.status(), StatusCode::BAD_REQUEST);
    let published_label_update = app
        .clone()
        .oneshot(json_request(
            format!(
                "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/update"
            ),
            json!({"label":"A renomeada"}),
        ))
        .await
        .expect("editar nome editorial publicado");
    assert_eq!(published_label_update.status(), StatusCode::BAD_REQUEST);
    let saved_label: (String,) =
        sqlx::query_as("SELECT label FROM custom_question_options WHERE id=?1")
            .bind(&single_option)
            .fetch_one(crate::db::pool())
            .await
            .expect("ler nome editorial publicado");
    assert_eq!(saved_label.0, "A");
    let replacement_image = package_smoke_master([100, 50, 20]);
    let replacement_request = {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"replacement.webp\"\r\nContent-Type: image/webp\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(&replacement_image);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/custom/events/{event_id}/items/{single_id}/options/{single_option}/image"
            ))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .header(
                "Cookie",
                format!("{}={token}", crate::security::session_cookie_name()),
            )
            .header("X-CSRF-Token", &csrf)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
            .body(Body::from(body))
            .expect("trocar imagem publicada")
    };
    let replacement = app
        .clone()
        .oneshot(replacement_request)
        .await
        .expect("troca de imagem publicada");
    assert_eq!(replacement.status(), StatusCode::BAD_REQUEST);
    let pool = response_json(
        app.clone()
            .oneshot(json_request(
                "/api/pools".into(),
                json!({"name":"Pool do smoke","eventId":event_id}),
            ))
            .await
            .expect("criar pool builder"),
    )
    .await;
    let pool_id = pool["id"].as_str().expect("id pool builder").to_string();
    let showcase = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/custom/event-showcase?poolId={pool_id}"))
                    .header(
                        "Cookie",
                        format!("{}={token}", crate::security::session_cookie_name()),
                    )
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                    .body(Body::empty())
                    .expect("ler showcase do pool"),
            )
            .await
            .expect("ler showcase builder"),
    )
    .await;
    assert!(showcase["coverAssetUrl"].as_str().is_some());
    let questions = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/custom/questions?poolId={pool_id}"))
                    .header(
                        "Cookie",
                        format!("{}={token}", crate::security::session_cookie_name()),
                    )
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                    .body(Body::empty())
                    .expect("ler perguntas do pool"),
            )
            .await
            .expect("ler perguntas builder"),
    )
    .await;
    assert!(questions
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| item["options"].as_array().into_iter().flatten())
        .any(|option| option["imageAssetUrl"].as_str().is_some()));
    let draft = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/custom/events/{event_id}/draft"))
                .header(
                    "Cookie",
                    format!("{}={token}", crate::security::session_cookie_name()),
                )
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                .body(Body::empty())
                .expect("ler draft builder"),
        )
        .await
        .expect("ler draft publicado");
    let draft_body = to_bytes(draft.into_body(), 2_000_000)
        .await
        .expect("draft body builder");
    let draft_json: serde_json::Value =
        serde_json::from_slice(&draft_body).expect("draft JSON builder");
    assert!(draft_json["event"]["coverAssetUrl"].as_str().is_some());
    assert!(draft_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "numeric" && item["decimalPlaces"] == 1));
    assert!(draft_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "multiple_choice"));
    assert!(draft_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| item["options"].as_array().into_iter().flatten())
        .any(|option| option["imageAssetUrl"].as_str().is_some()));
    let mine = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri("/api/custom/events/mine")
                    .header(
                        "Cookie",
                        format!("{}={token}", crate::security::session_cookie_name()),
                    )
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
                    .body(Body::empty())
                    .expect("listar eventos builder"),
            )
            .await
            .expect("listar eventos do builder"),
    )
    .await;
    assert!(mine
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["id"] == event_id && event["coverAssetUrl"].as_str().is_some()));
    let pool_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pools WHERE event_id=?1")
        .bind(&event_id)
        .fetch_one(crate::db::pool())
        .await
        .expect("pool count builder");
    assert_eq!(pool_count.0, 1);
    let _ = numeric_id;
}

async fn run_package_smoke_stage(stage: &str) {
    let root = package_smoke_root();
    let dev_db = root.join("dev.db");
    let prod_db = root.join("prod.db");
    let dev_assets = root.join("dev-assets");
    let prod_assets = root.join("prod-assets");
    let dev_slug = "package-promotion-smoke";
    match stage {
        "dev-a" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor = seed_user(
                "package-smoke-dev",
                "package-smoke-dev@example.test",
                "Senha-forte-123",
                true,
            )
            .await;
            let master = package_smoke_master([20, 40, 80]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor)
                .await
                .expect("asset DEV");
            let content = package_smoke_manifest("Evento de promoção", dev_slug, &hash);
            let preview = crate::custom_event_manifest::preview(&content)
                .await
                .expect("preview DEV");
            assert_eq!(
                preview.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            let result = crate::custom_event_manifest::apply_admin(
                &content,
                &preview.base_fingerprint,
                &actor,
            )
            .await
            .expect("apply DEV");
            let event_id = result.event_id.expect("event DEV");
            let package = crate::event_package::export(&event_id)
                .await
                .expect("export A");
            fs::write(root.join("package-a.zip"), package).expect("gravar package A");
        }
        "prod-a" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor = seed_user(
                "package-smoke-prod",
                "package-smoke-prod@example.test",
                "Senha-forte-123",
                true,
            )
            .await;
            let package = package_smoke_package(&root, "package-a.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview Create");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            assert_eq!(
                (
                    preview.manifest.item_count,
                    preview.manifest.option_count,
                    preview.manifest.link_count
                ),
                (3, 4, 1)
            );
            assert_eq!(preview.added_asset_count, 1);
            let applied =
                crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor)
                    .await
                    .expect("apply Create");
            assert_eq!(
                applied.result.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            let event_id = applied.result.event_id.clone().expect("event PROD");
            sqlx::query("UPDATE events SET status='active' WHERE id=?1")
                .bind(&event_id)
                .execute(crate::db::pool())
                .await
                .expect("publicar PROD");
            let pool_id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO pools(id,event_id,name,invite_code,created_by) VALUES(?1,?2,'Pool de promoção','package-smoke-invite',?3)")
                .bind(&pool_id)
                .bind(&event_id)
                .bind(&actor)
                .execute(crate::db::pool())
                .await
                .expect("pool de promoção");
            sqlx::query("INSERT INTO pool_members(pool_id,user_id) VALUES(?1,?2)")
                .bind(&pool_id)
                .bind(&actor)
                .execute(crate::db::pool())
                .await
                .expect("membro do pool de promoção");
            let item_id: (String,) = sqlx::query_as(
                "SELECT id FROM prediction_items WHERE event_id=?1 AND external_key='choice'",
            )
            .bind(&event_id)
            .fetch_one(crate::db::pool())
            .await
            .expect("item do pool de promoção");
            let option_id: (String,) = sqlx::query_as(
                "SELECT id FROM custom_question_options WHERE item_id=?1 AND external_key='a'",
            )
            .bind(&item_id.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("opção do pool de promoção");
            let prediction_id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO predictions(id,pool_id,user_id,item_id,match_id,home_score,away_score) VALUES(?1,?2,?3,?4,NULL,NULL,NULL)")
                .bind(&prediction_id)
                .bind(&pool_id)
                .bind(&actor)
                .bind(&item_id.0)
                .execute(crate::db::pool())
                .await
                .expect("prediction de promoção");
            sqlx::query(
                "INSERT INTO custom_prediction_values(prediction_id,option_id) VALUES(?1,?2)",
            )
            .bind(&prediction_id)
            .bind(&option_id.0)
            .execute(crate::db::pool())
            .await
            .expect("valor do prediction de promoção");
            let repeat = crate::event_package::preview(&package)
                .await
                .expect("preview NoChange");
            assert_eq!(
                repeat.manifest.action,
                crate::custom_event_manifest::ImportAction::NoChange
            );
            assert_eq!(repeat.existing_asset_count, 1);
            assert_eq!(repeat.added_asset_count, 0);
            let repeated =
                crate::event_package::apply(&package, &repeat.manifest.base_fingerprint, &actor)
                    .await
                    .expect("apply NoChange");
            assert_eq!(
                repeated.result.action,
                crate::custom_event_manifest::ImportAction::NoChange
            );
            let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM events WHERE slug=?1),(SELECT COUNT(*) FROM prediction_items WHERE event_id=?2),(SELECT COUNT(*) FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?2),(SELECT COUNT(*) FROM assets)")
                .bind(dev_slug).bind(&event_id).fetch_one(crate::db::pool()).await.expect("contagens PROD");
            assert_eq!(counts, (1, 3, 4, 1));
            let prod_package = crate::event_package::export(&event_id)
                .await
                .expect("export PROD A");
            assert_eq!(
                package_smoke_entry_names(&package),
                package_smoke_entry_names(&prod_package)
            );
            let dev_parsed = crate::event_package::parse(&package).expect("manifest DEV A");
            let prod_parsed = crate::event_package::parse(&prod_package).expect("manifest PROD A");
            assert_eq!(dev_parsed.manifest, prod_parsed.manifest);
            assert_eq!(
                dev_parsed
                    .assets
                    .keys()
                    .collect::<std::collections::HashSet<_>>(),
                prod_parsed
                    .assets
                    .keys()
                    .collect::<std::collections::HashSet<_>>()
            );
            fs::write(root.join("package-prod-a.zip"), prod_package).expect("gravar round-trip");
        }
        "dev-b" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-dev'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor DEV");
            let master = package_smoke_master([120, 20, 40]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor.0)
                .await
                .expect("asset B");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event DEV B");
            let asset: (String,) = sqlx::query_as("SELECT id FROM assets WHERE sha256=?1")
                .bind(&hash)
                .fetch_one(crate::db::pool())
                .await
                .expect("asset id B");
            sqlx::query("UPDATE events SET name='Evento de promoção atualizado',description='Descrição editorial B',cover_asset_id=?2 WHERE id=?1")
                .bind(&event.0).bind(&asset.0).execute(crate::db::pool()).await.expect("metadata B");
            let option: (String,) = sqlx::query_as("SELECT o.id FROM custom_question_options o JOIN prediction_items pi ON pi.id=o.item_id WHERE pi.event_id=?1 AND o.external_key='a'").bind(&event.0).fetch_one(crate::db::pool()).await.expect("option B");
            sqlx::query("UPDATE custom_question_options SET image_asset_id=?2 WHERE id=?1")
                .bind(&option.0)
                .bind(&asset.0)
                .execute(crate::db::pool())
                .await
                .expect("option asset B");
            let package = crate::event_package::export(&event.0)
                .await
                .expect("export B");
            fs::write(root.join("package-b.zip"), package).expect("gravar package B");
        }
        "prod-b" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor PROD B");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event PROD B");
            let before = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest antes B")
                .0;
            let used_before: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM pools WHERE event_id=?1),(SELECT COUNT(*) FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)")
                .bind(&event.0)
                .fetch_one(crate::db::pool())
                .await
                .expect("uso antes B");
            assert_eq!(used_before, (1, 1));
            let package = package_smoke_package(&root, "package-b.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview SafeUpdate");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::SafeUpdate
            );
            assert!(!preview.manifest.safe_changes.is_empty());
            assert!(preview.manifest.blocked_changes.is_empty());
            assert_eq!(preview.added_asset_count, 1);
            let result =
                crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
                    .await
                    .expect("apply SafeUpdate");
            assert_eq!(
                result.result.action,
                crate::custom_event_manifest::ImportAction::SafeUpdate
            );
            let after = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest depois B")
                .0;
            assert_eq!(
                package_smoke_structural_view(before),
                package_smoke_structural_view(after.clone())
            );
            assert_eq!(after.name, "Evento de promoção atualizado");
            assert_eq!(after.description.as_deref(), Some("Descrição editorial B"));
            let asset_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets")
                .fetch_one(crate::db::pool())
                .await
                .expect("assets B");
            assert_eq!(asset_count.0, 2);
            let used_after: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM pools WHERE event_id=?1),(SELECT COUNT(*) FROM predictions pr JOIN prediction_items pi ON pi.id=pr.item_id WHERE pi.event_id=?1)")
                .bind(&event.0)
                .fetch_one(crate::db::pool())
                .await
                .expect("uso depois B");
            assert_eq!(used_after, used_before);
        }
        "dev-c" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event DEV C");
            let item: (String,) = sqlx::query_as(
                "SELECT id FROM prediction_items WHERE event_id=?1 AND external_key='choice'",
            )
            .bind(&event.0)
            .fetch_one(crate::db::pool())
            .await
            .expect("item C");
            sqlx::query(
                "UPDATE prediction_items SET title='Título estrutural incompatível' WHERE id=?1",
            )
            .bind(&item.0)
            .execute(crate::db::pool())
            .await
            .expect("alteração estrutural C");
            let package = crate::event_package::export(&event.0)
                .await
                .expect("export C");
            fs::write(root.join("package-c.zip"), package).expect("gravar package C");
        }
        "prod-c" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor PROD C");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug=?1")
                .bind(dev_slug)
                .fetch_one(crate::db::pool())
                .await
                .expect("event PROD C");
            let package_b = package_smoke_package(&root, "package-b.zip");
            let stale_preview = crate::event_package::preview(&package_b)
                .await
                .expect("preview stale");
            sqlx::query("UPDATE events SET description='alteração concorrente' WHERE id=?1")
                .bind(&event.0)
                .execute(crate::db::pool())
                .await
                .expect("concorrência");
            assert!(crate::event_package::apply(
                &package_b,
                &stale_preview.manifest.base_fingerprint,
                &actor.0
            )
            .await
            .is_err());
            let package = package_smoke_package(&root, "package-c.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview Conflict");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::Conflict
            );
            assert!(preview
                .manifest
                .blocked_changes
                .iter()
                .any(|change| change.path.contains("title")));
            let before = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest antes C")
                .0;
            assert!(crate::event_package::apply(
                &package,
                &preview.manifest.base_fingerprint,
                &actor.0
            )
            .await
            .is_err());
            let after = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest depois C")
                .0;
            assert_eq!(before, after);
        }
        "vma-dev-a" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-dev'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA DEV");
            let master = package_smoke_master([30, 100, 180]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor.0)
                .await
                .expect("asset VMA DEV");
            let manifest = vma_smoke_manifest(&hash);
            let content = serde_json::to_string(&manifest).expect("serializar VMA DEV");
            let preview = crate::custom_event_manifest::preview(&content)
                .await
                .expect("preview VMA Create");
            assert_eq!(
                preview.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            assert_eq!(
                (preview.item_count, preview.option_count, preview.link_count),
                (19, 121, 4)
            );
            let result = crate::custom_event_manifest::apply_admin(
                &content,
                &preview.base_fingerprint,
                &actor.0,
            )
            .await
            .expect("apply VMA DEV");
            let event_id = result.event_id.expect("event VMA DEV");
            let package = crate::event_package::export(&event_id)
                .await
                .expect("export VMA A");
            fs::write(root.join("vma-package-a.zip"), package).expect("gravar VMA A");
        }
        "vma-prod-a" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA PROD");
            let package = package_smoke_package(&root, "vma-package-a.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview VMA PROD Create");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::Create
            );
            assert_eq!(
                (
                    preview.manifest.item_count,
                    preview.manifest.option_count,
                    preview.manifest.link_count
                ),
                (19, 121, 4)
            );
            assert_eq!(preview.asset_count, 1);
            let applied =
                crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
                    .await
                    .expect("apply VMA PROD");
            let event_id = applied.result.event_id.expect("event VMA PROD");
            sqlx::query("UPDATE events SET status='active' WHERE id=?1")
                .bind(&event_id)
                .execute(crate::db::pool())
                .await
                .expect("publicar VMA PROD");
            let repeat = crate::event_package::preview(&package)
                .await
                .expect("preview VMA NoChange");
            assert_eq!(
                repeat.manifest.action,
                crate::custom_event_manifest::ImportAction::NoChange
            );
            let prod_package = crate::event_package::export(&event_id)
                .await
                .expect("export VMA PROD A");
            let dev_parsed = crate::event_package::parse(&package).expect("parse VMA DEV A");
            let prod_parsed = crate::event_package::parse(&prod_package).expect("parse VMA PROD A");
            assert_eq!(dev_parsed.manifest, prod_parsed.manifest);
            assert_eq!(
                dev_parsed.assets.keys().collect::<HashSet<_>>(),
                prod_parsed.assets.keys().collect::<HashSet<_>>()
            );
        }
        "vma-dev-b" => {
            package_smoke_env(&dev_db, &dev_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-dev'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA DEV B");
            let master = package_smoke_master([180, 80, 30]);
            let hash = hex::encode(sha2::Sha256::digest(&master));
            crate::assets::ingest_package_master(&master, &hash, &actor.0)
                .await
                .expect("asset VMA B");
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug='vma-2026'")
                .fetch_one(crate::db::pool())
                .await
                .expect("event VMA DEV B");
            let mut manifest = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest VMA DEV B")
                .0;
            let base_fingerprint = crate::custom_event_manifest::fingerprint(&manifest)
                .expect("base fingerprint VMA B");
            let asset = crate::custom_event_manifest::AssetReference {
                kind: "asset".into(),
                sha256: hash,
                media_type: "image/webp".into(),
            };
            manifest.cover_asset = Some(asset.clone());
            let option = manifest
                .items
                .iter_mut()
                .find(|item| item.external_key == "best-pop")
                .and_then(|item| item.options.first_mut())
                .expect("opção VMA B");
            option.image_asset = Some(asset);
            let content =
                crate::custom_event_manifest::canonical_json(&manifest).expect("serializar VMA B");
            crate::custom_event_manifest::apply_admin(&content, &base_fingerprint, &actor.0)
                .await
                .expect("aplicar alteração VMA DEV B");
            let package = crate::event_package::export(&event.0)
                .await
                .expect("export VMA B");
            fs::write(root.join("vma-package-b.zip"), package).expect("gravar VMA B");
        }
        "vma-prod-b" => {
            package_smoke_env(&prod_db, &prod_assets);
            crate::db::init().await;
            let actor: (String,) =
                sqlx::query_as("SELECT id FROM users WHERE username='package-smoke-prod'")
                    .fetch_one(crate::db::pool())
                    .await
                    .expect("actor VMA PROD B");
            let package = package_smoke_package(&root, "vma-package-b.zip");
            let preview = crate::event_package::preview(&package)
                .await
                .expect("preview VMA SafeUpdate");
            assert_eq!(
                preview.manifest.action,
                crate::custom_event_manifest::ImportAction::SafeUpdate
            );
            assert_eq!(
                (
                    preview.manifest.item_count,
                    preview.manifest.option_count,
                    preview.manifest.link_count
                ),
                (19, 121, 4)
            );
            assert!(preview.manifest.blocked_changes.is_empty());
            assert_eq!(preview.added_asset_count, 1);
            let event: (String,) = sqlx::query_as("SELECT id FROM events WHERE slug='vma-2026'")
                .fetch_one(crate::db::pool())
                .await
                .expect("event VMA PROD B");
            crate::event_package::apply(&package, &preview.manifest.base_fingerprint, &actor.0)
                .await
                .expect("apply VMA SafeUpdate");
            let after = crate::custom_event_manifest::export_for_event(&event.0)
                .await
                .expect("manifest VMA depois B")
                .0;
            assert_eq!(
                (
                    after.items.len(),
                    after
                        .items
                        .iter()
                        .map(|item| item.options.len())
                        .sum::<usize>()
                ),
                (19, 121)
            );
            assert_eq!(
                after
                    .items
                    .iter()
                    .map(|item| item
                        .options
                        .iter()
                        .map(|option| option.links.len())
                        .sum::<usize>())
                    .sum::<usize>(),
                4
            );
            let asset_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM assets")
                .fetch_one(crate::db::pool())
                .await
                .expect("assets VMA PROD B");
            assert_eq!(
                asset_count.0, 4,
                "2 assets do smoke genérico + 2 assets do VMA"
            );
        }
        _ => panic!("stage de package smoke desconhecido: {stage}"),
    }
}

#[tokio::test]
#[ignore = "smoke explícito usa processos e dois bancos SQLite independentes"]
async fn package_promotion_two_sqlite_smoke() {
    if let Ok(stage) = std::env::var("PACKAGE_SMOKE_STAGE") {
        run_package_smoke_stage(&stage).await;
        return;
    }
    let root =
        std::env::temp_dir().join(format!("presumidos-package-smoke-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("diretório do smoke");
    let executable = std::env::current_exe().expect("binário de testes");
    for stage in [
        "dev-a",
        "prod-a",
        "dev-b",
        "prod-b",
        "dev-c",
        "prod-c",
        "vma-dev-a",
        "vma-prod-a",
        "vma-dev-b",
        "vma-prod-b",
    ] {
        let status = Command::new(&executable)
            .arg("--exact")
            .arg("http_tests::package_promotion_two_sqlite_smoke")
            .arg("--ignored")
            .arg("--nocapture")
            .env("PACKAGE_SMOKE_ROOT", &root)
            .env("PACKAGE_SMOKE_STAGE", stage)
            .status()
            .expect("iniciar etapa do smoke");
        assert!(status.success(), "etapa {stage} falhou");
    }
    let _ = fs::remove_dir_all(root);
}
