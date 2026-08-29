use super::super::*;

pub(crate) fn is_email_code_expired(expires_at: &str, attempts: i64) -> bool {
    if attempts >= EMAIL_CODE_MAX_ATTEMPTS {
        return true;
    }
    match parsed_sqlite_utc(expires_at) {
        Some(stamp) => chrono::Utc::now() > stamp,
        None => true,
    }
}

/// Incrementa o contador de tentativas de um codigo de email na tabela informada.
#[cfg(feature = "server")]
pub(crate) async fn register_email_code_attempt(
    db: &sqlx::SqlitePool,
    table: &str,
    email: &str,
) -> Result<(), ServerFnError> {
    let sql = format!("UPDATE {table} SET attempts = attempts + 1 WHERE email = ?1");
    sqlx::query(&sql)
        .bind(email)
        .execute(db)
        .await
        .map_err(|e| crate::security::internal_error("register_email_code_attempt", e))?;
    Ok(())
}
