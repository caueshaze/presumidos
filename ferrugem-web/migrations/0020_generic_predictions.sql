-- Prediction passa a pertencer ao pool e ao item. A reconstrução é segura:
-- nenhuma tabela existente possui FK para predictions.id.
ALTER TABLE predictions RENAME TO predictions_before_generic_items;

CREATE TABLE predictions (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    match_id TEXT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    home_score INTEGER NOT NULL,
    away_score INTEGER NOT NULL,
    submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
    qualifier TEXT,
    went_to_penalties INTEGER NOT NULL DEFAULT 0,
    penalty_home_score INTEGER,
    penalty_away_score INTEGER,
    UNIQUE(pool_id, user_id, item_id)
);

-- Uma prediction histórica era global. Ela é preservada em cada pool do mesmo
-- evento em que o usuário já participava; sem membership não há pool legítimo
-- para a nova identidade e a migration aborta em vez de descartar dados.
INSERT INTO predictions (
    id, pool_id, user_id, item_id, match_id, home_score, away_score, submitted_at,
    qualifier, went_to_penalties, penalty_home_score, penalty_away_score
)
SELECT
    old.id || '-' || pm.pool_id,
    pm.pool_id, old.user_id, m.prediction_item_id, old.match_id,
    old.home_score, old.away_score, old.submitted_at,
    old.qualifier, old.went_to_penalties, old.penalty_home_score, old.penalty_away_score
FROM predictions_before_generic_items old
JOIN matches m ON m.id = old.match_id
JOIN pool_members pm ON pm.user_id = old.user_id
JOIN pools p ON p.id = pm.pool_id
JOIN prediction_items pi ON pi.id = m.prediction_item_id AND pi.event_id = p.event_id;

-- Fail closed: nenhum dado histórico pode ser perdido por ausência de pool.
CREATE TRIGGER predictions_historical_backfill_complete
BEFORE DELETE ON predictions_before_generic_items
WHEN EXISTS (
    SELECT 1 FROM predictions_before_generic_items old
    WHERE NOT EXISTS (
        SELECT 1 FROM predictions p
        WHERE p.user_id = old.user_id AND p.match_id = old.match_id
    )
)
BEGIN SELECT RAISE(ABORT, 'prediction histórica sem pool compatível'); END;

DELETE FROM predictions_before_generic_items;
DROP TABLE predictions_before_generic_items;

CREATE INDEX idx_predictions_pool_user_item ON predictions(pool_id, user_id, item_id);
CREATE INDEX idx_predictions_item ON predictions(item_id);

CREATE TRIGGER predictions_match_item_consistent_insert
BEFORE INSERT ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM matches m WHERE m.id = NEW.match_id AND m.prediction_item_id = NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id = p.event_id
    WHERE p.id = NEW.pool_id AND pi.id = NEW.item_id
)
BEGIN SELECT RAISE(ABORT, 'prediction item/match/pool inconsistente'); END;

CREATE TRIGGER predictions_match_item_consistent_update
BEFORE UPDATE OF pool_id, item_id, match_id ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM matches m WHERE m.id = NEW.match_id AND m.prediction_item_id = NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id = p.event_id
    WHERE p.id = NEW.pool_id AND pi.id = NEW.item_id
)
BEGIN SELECT RAISE(ABORT, 'prediction item/match/pool inconsistente'); END;
