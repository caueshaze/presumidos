-- Integração de resultados ao vivo via API-Football v3.
-- Adiciona o mapeamento para o fixture externo, as colunas de placar ao vivo
-- (apenas exibição — não contam no ranking) e os metadados de origem do
-- resultado oficial, para que o resultado lançado manualmente pelo admin seja
-- soberano e nunca sobrescrito pelo poller automático.

-- Mapeamento jogo local (jogo-NNN) -> fixture.id da API-Football.
ALTER TABLE matches ADD COLUMN external_fixture_id INTEGER;

-- Placar ao vivo (parcial). É só para exibição na UI; o ranking continua
-- contando apenas home_score/away_score (preenchidos no encerramento).
ALTER TABLE matches ADD COLUMN live_home_score INTEGER;
ALTER TABLE matches ADD COLUMN live_away_score INTEGER;
ALTER TABLE matches ADD COLUMN live_status TEXT;
ALTER TABLE matches ADD COLUMN live_elapsed INTEGER;
ALTER TABLE matches ADD COLUMN live_updated_at TEXT;

-- Origem do resultado oficial: 'manual' (admin) ou 'api' (poller).
-- 'manual' é soberano: o poller nunca sobrescreve, apenas registra conflito.
ALTER TABLE matches ADD COLUMN result_source TEXT;
ALTER TABLE matches ADD COLUMN result_synced_at TEXT;
ALTER TABLE matches ADD COLUMN result_external_raw_status TEXT;

-- Resultados que já existiam (lançados manualmente antes desta migration) são
-- tratados como manuais, preservando a soberania do admin.
UPDATE matches
   SET result_source = 'manual'
 WHERE home_score IS NOT NULL AND away_score IS NOT NULL;

CREATE INDEX idx_matches_external_fixture ON matches(external_fixture_id);
