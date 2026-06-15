//! Envio de emails transacionais via Resend.
//!
//! O cliente le `RESEND_API_KEY`/`RESEND_FROM_EMAIL` da configuracao
//! (carregada do `.env` em [`crate::config`]).

use crate::error::ServerFnError;
use std::sync::OnceLock;

use resend_rs::types::CreateEmailBaseOptions;
use resend_rs::Resend;

static CLIENT: OnceLock<Resend> = OnceLock::new();

fn client() -> &'static Resend {
    CLIENT.get_or_init(|| Resend::new(&crate::config::settings().resend_api_key))
}

/// Extrai apenas o endereco de um remetente no formato `Nome <email>` ou `email`.
fn from_address(from: &str) -> &str {
    match (from.find('<'), from.find('>')) {
        (Some(start), Some(end)) if end > start => from[start + 1..end].trim(),
        _ => from.trim(),
    }
}

async fn send(to: &str, subject: &str, html: String) -> Result<(), ServerFnError> {
    let settings = crate::config::settings();
    if settings.disable_auth_emails {
        eprintln!(
            "[dev-auth-email] to={to} subject={subject} (emails desativados neste ambiente)"
        );
        return Ok(());
    }

    let from = settings.resend_from_email.as_str();
    let email = CreateEmailBaseOptions::new(from, [to], subject)
        .with_html(&html)
        // Reforca que a caixa nao monitora respostas.
        .with_reply(from_address(from));

    client()
        .emails
        .send(email)
        .await
        .map_err(|e| crate::security::internal_error("send_email", e))?;

    Ok(())
}

fn code_card(intro: &str, code: &str, footer: &str) -> String {
    format!(
        r#"<div style="margin:0;padding:32px 16px;background:linear-gradient(180deg,#eaf6f0 0%,#fff8e7 100%);font-family:'Segoe UI',Helvetica,Arial,sans-serif">
  <div style="max-width:480px;margin:0 auto;background:#ffffff;border-radius:20px;overflow:hidden;box-shadow:0 4px 20px rgba(45,58,58,0.08)">
    <div style="background:linear-gradient(135deg,#a8e6cf 0%,#a0d2eb 100%);padding:28px 32px;text-align:center">
      <h1 style="margin:0;color:#2d3a3a;font-size:26px;font-weight:700;letter-spacing:0.5px">Presumidos</h1>
    </div>
    <div style="padding:32px">
      <p style="margin:0 0 8px;color:#2d3a3a;font-size:16px;line-height:1.5">{intro}</p>
      <div style="margin:24px 0;text-align:center">
        <span style="display:inline-block;background:#eaf6f0;border:2px solid #a8e6cf;border-radius:14px;padding:16px 28px;color:#5fbf9f;font-size:34px;font-weight:700;letter-spacing:10px">{code}</span>
      </div>
      <p style="margin:0;color:#6b7a7a;font-size:14px;line-height:1.5">{footer}</p>
    </div>
    <div style="border-top:1px solid #eef2ee;padding:18px 32px;text-align:center">
      <p style="margin:0;color:#9aa6a6;font-size:12px;line-height:1.5">Este e um email automatico, por favor nao responda.<br>Presumidos &middot; seu bolão entre amigos</p>
    </div>
  </div>
</div>"#,
    )
}

/// Envia o codigo de verificacao para confirmar a criacao da conta.
pub async fn send_verification_code(to: &str, code: &str) -> Result<(), ServerFnError> {
    if crate::config::settings().disable_auth_emails {
        eprintln!("[dev-auth-email] verification code for {to}: {code}");
    }
    let html = code_card(
        "Use o codigo abaixo para confirmar a criacao da sua conta no Presumidos:",
        code,
        "O codigo expira em 15 minutos. Se voce nao tentou se cadastrar, ignore este email.",
    );
    send(to, "Confirme sua conta no Presumidos", html).await
}

/// Envia o codigo para redefinicao de senha.
pub async fn send_password_reset_code(to: &str, code: &str) -> Result<(), ServerFnError> {
    if crate::config::settings().disable_auth_emails {
        eprintln!("[dev-auth-email] password reset code for {to}: {code}");
    }
    let html = code_card(
        "Use o codigo abaixo para redefinir a senha da sua conta no Presumidos:",
        code,
        "O codigo expira em 15 minutos. Se voce nao pediu a redefinicao, ignore este email.",
    );
    send(to, "Redefinicao de senha - Presumidos", html).await
}
