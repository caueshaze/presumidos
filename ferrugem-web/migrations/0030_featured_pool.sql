INSERT INTO app_settings (key, value) VALUES ('featured_pool_id', '')
ON CONFLICT(key) DO NOTHING;
