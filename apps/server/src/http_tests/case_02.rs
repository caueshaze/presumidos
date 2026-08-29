use super::*;

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
