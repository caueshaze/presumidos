#[cfg(feature = "server")]
use std::sync::OnceLock;

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub database_path: String,
    pub session_secret: String,
    pub session_ttl_hours: i64,
    pub cookie_secure: bool,
    pub admin_reauth_ttl_minutes: i64,
}

#[cfg(feature = "server")]
static CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[cfg(feature = "server")]
fn required_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("variavel {name} ausente no .env"))
}

#[cfg(feature = "server")]
fn parse_bool_var(name: &str) -> bool {
    match required_var(name).trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => panic!("variavel {name} deve ser booleana"),
    }
}

#[cfg(feature = "server")]
fn parse_i64_var(name: &str) -> i64 {
    required_var(name)
        .trim()
        .parse::<i64>()
        .unwrap_or_else(|_| panic!("variavel {name} deve ser numerica"))
}

#[cfg(feature = "server")]
pub fn settings() -> &'static AppConfig {
    CONFIG.get_or_init(|| {
        dotenvy::dotenv().expect("arquivo .env nao encontrado; crie-o a partir de .env.example");

        let app_env = required_var("APP_ENV").trim().to_lowercase();
        let database_path = required_var("DATABASE_PATH");
        let session_secret = required_var("SESSION_SECRET");
        let session_ttl_hours = parse_i64_var("SESSION_TTL_HOURS");
        let cookie_secure = parse_bool_var("COOKIE_SECURE");
        let admin_reauth_ttl_minutes = parse_i64_var("ADMIN_REAUTH_TTL_MINUTES");

        assert!(
            session_secret.trim().len() >= 32,
            "SESSION_SECRET precisa ter pelo menos 32 caracteres"
        );
        assert!(session_ttl_hours > 0, "SESSION_TTL_HOURS deve ser > 0");
        assert!(
            admin_reauth_ttl_minutes > 0,
            "ADMIN_REAUTH_TTL_MINUTES deve ser > 0"
        );

        if app_env == "production" {
            assert!(
                cookie_secure,
                "COOKIE_SECURE precisa estar habilitado em producao"
            );
        }

        AppConfig {
            app_env,
            database_path,
            session_secret,
            session_ttl_hours,
            cookie_secure,
            admin_reauth_ttl_minutes,
        }
    })
}
