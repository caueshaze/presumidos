-- Encerramento da Copa: permanece desligado até a confirmação do administrador.
-- Versioned after the historical closing-screen migration.
INSERT INTO app_settings (key, value) VALUES ('closing_screen_enabled', '0')
ON CONFLICT(key) DO NOTHING;
