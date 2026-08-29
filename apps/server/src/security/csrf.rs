use sha2::Digest;

use crate::{config::settings, error::ServerFnError, security::public_error};

#[cfg(feature = "server")]
pub fn csrf_token() -> String {
    let seed = format!(
        "{}:{}:{}",
        settings().session_secret,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        uuid::Uuid::new_v4()
    );
    let digest = sha2::Sha256::digest(seed.as_bytes());
    hex::encode(digest)
}

/// Gera um codigo numerico de 6 digitos para verificacao por email.
#[cfg(feature = "server")]
pub fn verification_code() -> String {
    use rand_core::{OsRng, RngCore};
    format!("{:06}", OsRng.next_u32() % 1_000_000)
}

/// Hash de um codigo de verificacao para armazenamento (nunca guardar em texto puro).
#[cfg(feature = "server")]
pub fn hash_code(code: &str) -> String {
    let seed = format!("{}:{}", settings().session_secret, code.trim());
    let digest = sha2::Sha256::digest(seed.as_bytes());
    hex::encode(digest)
}

#[cfg(feature = "server")]
pub fn require_csrf(expected: &str, provided: &str) -> Result<(), ServerFnError> {
    if expected.is_empty() || provided.trim().is_empty() || expected != provided.trim() {
        return Err(public_error(
            "Falha de seguranca da sessao. Atualize a pagina e tente novamente.",
        ));
    }
    Ok(())
}
