use std::sync::OnceLock;

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitBackendKind {
    Memory,
    Redis,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub database_path: String,
    pub backup_dir: String,
    pub public_base_url: Option<String>,
    pub listen_address: String,
    pub shutdown_timeout_secs: u64,
    pub min_free_bytes: u64,
    pub database_busy_timeout_ms: u64,
    pub max_body_bytes: usize,
    pub json_logs: bool,
    pub metrics_enabled: bool,
    pub contact_email: Option<String>,
    pub session_secret: String,
    pub admin_bootstrap_secret: String,
    pub session_ttl_hours: i64,
    pub cookie_secure: bool,
    pub admin_reauth_ttl_minutes: i64,
    pub trusted_proxy_cidrs: Vec<ipnet::IpNet>,
    pub require_trusted_proxy: bool,
    pub resend_api_key: String,
    pub resend_from_email: String,
    pub disable_auth_emails: bool,
    pub rate_limit_backend: RateLimitBackendKind,
    pub redis_url: Option<String>,
    pub rate_limit_identity_secret: String,
    pub argon2_memory_kib: u32,
    pub argon2_time_cost: u32,
    pub argon2_parallelism: u32,
    pub argon2_policy_version: String,
    pub football: FootballConfig,
    pub web_push: WebPushConfig,
    pub asset_dir: String,
    pub asset_max_upload_bytes: usize,
    pub asset_max_pixels: u64,
}

/// Configuração da integração de resultados ao vivo via provedor público de placares.
/// Tudo é opcional: se `enabled` for false, o poller nunca sobe. A API é pública
/// (sem chave), então não há cota/segredo aqui.
#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct FootballConfig {
    /// Liga a integração (sync + leitura). Sem isso, nada de chamadas externas.
    pub enabled: bool,
    /// Sobe o poller em background nesta instância. Mantenha `true` em apenas
    /// uma réplica para não duplicar requisições à API pública.
    pub poller_enabled: bool,
    pub base_url: String,
    pub poll_interval_secs: u64,
    /// Intervalo (menor) usado enquanto há jogo na janela, para a pontuação ao
    /// vivo andar mais rápido. Fora de jogo, usa `poll_interval_secs`.
    pub live_poll_interval_secs: u64,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct WebPushConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    pub vapid_public_key: Option<String>,
    pub vapid_private_key: Option<String>,
    pub contact_email: Option<String>,
}

#[cfg(feature = "server")]
pub(crate) static CONFIG: OnceLock<AppConfig> = OnceLock::new();
