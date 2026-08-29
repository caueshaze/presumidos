use crate::{error::ServerFnError, security::public_error};

#[cfg(feature = "server")]
pub fn normalize_required_text(
    field: &str,
    value: String,
    min_len: usize,
    max_len: usize,
) -> Result<String, ServerFnError> {
    let value = value.trim().to_string();
    if value.len() < min_len {
        return Err(public_error(format!("{field} muito curto.")));
    }
    if value.len() > max_len {
        return Err(public_error(format!("{field} muito longo.")));
    }
    Ok(value)
}

#[cfg(feature = "server")]
pub fn normalize_optional_text(value: String, max_len: usize) -> Result<String, ServerFnError> {
    let value = value.trim().to_string();
    if value.len() > max_len {
        return Err(public_error("Texto acima do tamanho permitido."));
    }
    Ok(value)
}

#[cfg(feature = "server")]
pub fn normalize_email(email: String) -> Result<String, ServerFnError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || email.len() > 120 || !email_format_is_valid(&email) {
        return Err(public_error("Email invalido."));
    }
    Ok(email)
}

/// Valida o formato de um email `local@dominio.tld` de forma conservadora.
///
/// O objetivo nao e cobrir 100% da RFC 5322 (inviavel e desnecessario), mas barrar
/// typos e enderecos obviamente invalidos antes de gerar um envio inutil no Resend.
/// A prova final de posse continua sendo o codigo de verificacao.
#[cfg(feature = "server")]
fn email_format_is_valid(email: &str) -> bool {
    // Exatamente um '@', separando parte local e dominio.
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };

    // Parte local: 1..=64 chars, sem espacos, apenas [A-Za-z0-9._%+-],
    // sem ponto no inicio/fim e sem '..'.
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-'))
    {
        return false;
    }

    // Dominio: ao menos um '.', rotulos nao-vazios com [A-Za-z0-9-] sem hifen
    // no inicio/fim, e TLD final com >=2 letras.
    if !domain.contains('.') {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    let all_labels_ok = labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    });
    if !all_labels_ok {
        return false;
    }
    let tld = labels.last().copied().unwrap_or("");
    tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic())
}

#[cfg(feature = "server")]
pub fn validate_uuid(field: &str, value: &str) -> Result<(), ServerFnError> {
    uuid::Uuid::parse_str(value).map_err(|_| public_error(format!("{field} invalido.")))?;
    Ok(())
}

/// Valida um identificador de partida. Os IDs de partidas vêm do seed (ex.: `jogo-001`),
/// não são UUIDs — então aceitamos um token curto de [A-Za-z0-9_-]. A existência real é
/// verificada na consulta seguinte ("Partida nao encontrada.").
#[cfg(feature = "server")]
pub fn validate_match_id(value: &str) -> Result<(), ServerFnError> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(public_error("Partida invalida."));
    }
    Ok(())
}
