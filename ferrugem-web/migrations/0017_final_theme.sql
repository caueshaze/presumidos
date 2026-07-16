-- Edição visual temporária da final. Permanece desligada até um admin ativá-la.
INSERT INTO app_settings (key, value) VALUES ('final_theme_enabled', '0')
ON CONFLICT(key) DO NOTHING;
