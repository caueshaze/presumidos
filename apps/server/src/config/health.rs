use std::path::Path;

use super::loader::settings;

pub fn check_config() -> Result<(), String> {
    let _ = dotenvy::dotenv();
    match std::panic::catch_unwind(settings) {
        Ok(config) => {
            if config.app_env == "production" {
                for (name, configured_path) in [
                    ("DATABASE_PATH", config.database_path.as_str()),
                    ("PRESUMIDOS_ASSET_DIR", config.asset_dir.as_str()),
                    ("PRESUMIDOS_BACKUP_DIR", config.backup_dir.as_str()),
                ] {
                    if let Some(parent) = Path::new(configured_path).parent() {
                        if !parent.as_os_str().is_empty() && !parent.exists() {
                            return Err(format!("o diretório pai de {name} não existe"));
                        }
                    }
                }
            }
            Ok(())
        }
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|value| value.to_string())
                })
                .unwrap_or_else(|| "configuração inválida".to_string());
            Err(message)
        }
    }
}
