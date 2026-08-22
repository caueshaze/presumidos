-- no-transaction
-- Fase 8: previsões numéricas usam inteiros escalados. Nunca há float no
-- contrato persistido nem na comparação de scoring.
-- SQLite não possui ALTER CHECK e esta tabela é referenciada por várias
-- relações históricas. Atualizamos apenas o DDL registrado, sem reconstruir
-- nem desligar relações existentes; uma nova conexão recarrega o schema.
PRAGMA writable_schema = ON;
UPDATE sqlite_master SET sql = replace(sql, "'football_match', 'single_choice'", "'football_match', 'single_choice', 'numeric'")
WHERE type = 'table' AND name = 'prediction_items';
PRAGMA writable_schema = OFF;

CREATE TABLE numeric_questions (
    item_id TEXT PRIMARY KEY REFERENCES prediction_items(id) ON DELETE CASCADE,
    decimal_places INTEGER NOT NULL CHECK(decimal_places BETWEEN 0 AND 6),
    unit_label TEXT,
    min_value_scaled INTEGER,
    max_value_scaled INTEGER,
    result_value_scaled INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(min_value_scaled IS NULL OR max_value_scaled IS NULL OR min_value_scaled <= max_value_scaled)
);
CREATE TRIGGER numeric_question_is_numeric BEFORE INSERT ON numeric_questions
WHEN NOT EXISTS (SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND kind='numeric')
BEGIN SELECT RAISE(ABORT,'numeric question requer item numeric'); END;

CREATE TABLE numeric_prediction_values (
    prediction_id TEXT PRIMARY KEY REFERENCES predictions(id) ON DELETE CASCADE,
    value_scaled INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TRIGGER numeric_prediction_value_valid_insert BEFORE INSERT ON numeric_prediction_values
WHEN NOT EXISTS (
    SELECT 1 FROM predictions p JOIN prediction_items pi ON pi.id=p.item_id
    WHERE p.id=NEW.prediction_id AND pi.kind='numeric' AND p.match_id IS NULL
)
BEGIN SELECT RAISE(ABORT,'valor numeric incompatível com prediction'); END;
CREATE TRIGGER numeric_prediction_value_valid_update BEFORE UPDATE OF prediction_id,value_scaled ON numeric_prediction_values
WHEN NOT EXISTS (
    SELECT 1 FROM predictions p JOIN prediction_items pi ON pi.id=p.item_id
    WHERE p.id=NEW.prediction_id AND pi.kind='numeric' AND p.match_id IS NULL
)
BEGIN SELECT RAISE(ABORT,'valor numeric incompatível com prediction'); END;

CREATE TABLE numeric_pool_item_scoring (
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    exact_points INTEGER NOT NULL CHECK(exact_points BETWEEN 0 AND 1000),
    tolerance_scaled INTEGER NOT NULL CHECK(tolerance_scaled >= 0),
    within_tolerance_points INTEGER NOT NULL CHECK(within_tolerance_points BETWEEN 0 AND 1000),
    incorrect_points INTEGER NOT NULL CHECK(incorrect_points BETWEEN 0 AND 1000),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY(pool_id,item_id)
);
CREATE TRIGGER numeric_pool_item_scoring_valid_insert BEFORE INSERT ON numeric_pool_item_scoring
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id AND pi.kind='numeric'
)
BEGIN SELECT RAISE(ABORT,'configuração numeric incompatível com pool/item'); END;

CREATE TRIGGER numeric_scoring_defaults_after_pool AFTER INSERT ON pools BEGIN
    INSERT INTO numeric_pool_item_scoring(pool_id,item_id,exact_points,tolerance_scaled,within_tolerance_points,incorrect_points)
    SELECT NEW.id,n.item_id,1,0,0,0 FROM prediction_items pi JOIN numeric_questions n ON n.item_id=pi.id
    WHERE pi.event_id=NEW.event_id AND pi.kind='numeric';
END;
CREATE TRIGGER numeric_scoring_defaults_after_question AFTER INSERT ON numeric_questions BEGIN
    INSERT INTO numeric_pool_item_scoring(pool_id,item_id,exact_points,tolerance_scaled,within_tolerance_points,incorrect_points)
    SELECT p.id,NEW.item_id,1,0,0,0 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
    WHERE pi.id=NEW.item_id;
END;
INSERT INTO numeric_pool_item_scoring(pool_id,item_id,exact_points,tolerance_scaled,within_tolerance_points,incorrect_points)
SELECT p.id,n.item_id,1,0,0,0 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
JOIN numeric_questions n ON n.item_id=pi.id WHERE pi.kind='numeric';

CREATE TABLE numeric_prediction_score_breakdowns (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    predicted_value_scaled INTEGER NOT NULL,
    official_value_scaled INTEGER NOT NULL,
    difference_scaled INTEGER NOT NULL,
    outcome TEXT NOT NULL CHECK(outcome IN ('exact','within_tolerance','incorrect')),
    exact_points INTEGER NOT NULL,
    tolerance_scaled INTEGER NOT NULL,
    within_tolerance_points INTEGER NOT NULL,
    incorrect_points INTEGER NOT NULL,
    total_points INTEGER NOT NULL,
    eligible INTEGER NOT NULL DEFAULT 0,
    eligibility_reason TEXT NOT NULL DEFAULT '',
    computed_at TEXT NOT NULL DEFAULT(datetime('now')),
    UNIQUE(pool_id,user_id,item_id)
);

-- 0021's trigger predates numeric. Recreate it with the third legal kind.
DROP TRIGGER predictions_kind_consistent_insert;
DROP TRIGGER predictions_kind_consistent_update;
CREATE TRIGGER predictions_kind_consistent_insert BEFORE INSERT ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND (
        (kind='football_match' AND NEW.match_id IS NOT NULL AND EXISTS (SELECT 1 FROM matches m WHERE m.id=NEW.match_id AND m.prediction_item_id=NEW.item_id) AND NEW.home_score IS NOT NULL AND NEW.away_score IS NOT NULL)
        OR (kind IN ('single_choice','numeric') AND NEW.match_id IS NULL AND NEW.home_score IS NULL AND NEW.away_score IS NULL)
    )
)
BEGIN SELECT RAISE(ABORT,'prediction incompatível com item'); END;
CREATE TRIGGER predictions_kind_consistent_update BEFORE UPDATE OF pool_id,item_id,match_id,home_score,away_score ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND (
        (kind='football_match' AND NEW.match_id IS NOT NULL AND EXISTS (SELECT 1 FROM matches m WHERE m.id=NEW.match_id AND m.prediction_item_id=NEW.item_id) AND NEW.home_score IS NOT NULL AND NEW.away_score IS NOT NULL)
        OR (kind IN ('single_choice','numeric') AND NEW.match_id IS NULL AND NEW.home_score IS NULL AND NEW.away_score IS NULL)
    )
)
BEGIN SELECT RAISE(ABORT,'prediction incompatível com item'); END;
