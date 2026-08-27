-- A primeira versão de EventVersion substituiu a unicidade por evento pela
-- unicidade por versão. Esta migration existe para bases que já aplicaram a
-- migration de versões antes dessa correção de índice.
DROP INDEX IF EXISTS idx_prediction_items_event_external_key;

CREATE UNIQUE INDEX IF NOT EXISTS idx_prediction_items_version_external_key
    ON prediction_items(event_version_id, external_key)
    WHERE external_key IS NOT NULL;
