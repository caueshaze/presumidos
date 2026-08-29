pub(crate) fn sqlite_utc_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn sqlite_utc_after_hours(hours: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::hours(hours))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
pub(crate) fn sqlite_utc_after_minutes(minutes: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::minutes(minutes))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Validade dos codigos de verificacao por email, em minutos.
#[cfg(feature = "server")]
pub(crate) const EMAIL_CODE_TTL_MINUTES: i64 = 15;

/// Numero maximo de tentativas de digitacao de um codigo antes de exigir um novo envio.
#[cfg(feature = "server")]
pub(crate) const EMAIL_CODE_MAX_ATTEMPTS: i64 = 5;

#[cfg(feature = "server")]
pub(crate) type UserPublicRow = (String, String, String, bool, Option<String>, Option<String>);

#[cfg(feature = "server")]
pub(crate) type LoginRow = (
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
);

#[cfg(feature = "server")]
pub(crate) fn parsed_sqlite_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
}
