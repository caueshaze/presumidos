use super::super::*;
pub(crate) fn argon2_policy() -> argon2::Argon2<'static> {
    use argon2::{Algorithm, Argon2, Params, Version};

    let cfg = crate::config::settings();
    let params = Params::new(
        cfg.argon2_memory_kib,
        cfg.argon2_time_cost,
        cfg.argon2_parallelism,
        None,
    )
    .expect("parametros de argon2 invalidos");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

#[cfg(feature = "server")]
pub(crate) fn needs_rehash(parsed_hash: &argon2::password_hash::PasswordHash<'_>) -> bool {
    use argon2::{Params, Version};

    let cfg = crate::config::settings();
    if parsed_hash.version != Some(Version::V0x13 as u32) {
        return true;
    }

    match Params::try_from(parsed_hash) {
        Ok(params) => {
            params.m_cost() != cfg.argon2_memory_kib
                || params.t_cost() != cfg.argon2_time_cost
                || params.p_cost() != cfg.argon2_parallelism
        }
        Err(_) => true,
    }
}

#[cfg(feature = "server")]
pub(crate) fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::password_hash::SaltString;
    use argon2::PasswordHasher;
    use rand_core::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    argon2_policy()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| crate::security::internal_error("hash_password", e))
        .map(|hash| hash.to_string())
}
