-- Fase 4: segundo tipo real de PredictionItem. Valores football continuam
-- temporariamente na tabela principal, mas são nulos para single_choice.
ALTER TABLE predictions RENAME TO predictions_before_custom_questions;

CREATE TABLE predictions (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    match_id TEXT REFERENCES matches(id) ON DELETE CASCADE,
    home_score INTEGER,
    away_score INTEGER,
    submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
    qualifier TEXT,
    went_to_penalties INTEGER NOT NULL DEFAULT 0,
    penalty_home_score INTEGER,
    penalty_away_score INTEGER,
    UNIQUE(pool_id, user_id, item_id)
);

INSERT INTO predictions
SELECT id, pool_id, user_id, item_id, match_id, home_score, away_score,
       submitted_at, qualifier, went_to_penalties, penalty_home_score, penalty_away_score
FROM predictions_before_custom_questions;
DROP TABLE predictions_before_custom_questions;

CREATE INDEX idx_predictions_pool_user_item ON predictions(pool_id, user_id, item_id);
CREATE INDEX idx_predictions_item ON predictions(item_id);

CREATE TRIGGER predictions_kind_consistent_insert
BEFORE INSERT ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id = p.event_id
    WHERE p.id = NEW.pool_id AND pi.id = NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM prediction_items WHERE id = NEW.item_id AND (
        (kind = 'football_match' AND NEW.match_id IS NOT NULL
         AND EXISTS (SELECT 1 FROM matches m WHERE m.id = NEW.match_id AND m.prediction_item_id = NEW.item_id)
         AND NEW.home_score IS NOT NULL AND NEW.away_score IS NOT NULL)
        OR (kind = 'single_choice' AND NEW.match_id IS NULL
            AND NEW.home_score IS NULL AND NEW.away_score IS NULL)
    )
)
BEGIN SELECT RAISE(ABORT, 'prediction incompatível com item'); END;

CREATE TRIGGER predictions_kind_consistent_update
BEFORE UPDATE OF pool_id, item_id, match_id, home_score, away_score ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id = p.event_id
    WHERE p.id = NEW.pool_id AND pi.id = NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM prediction_items WHERE id = NEW.item_id AND (
        (kind = 'football_match' AND NEW.match_id IS NOT NULL
         AND EXISTS (SELECT 1 FROM matches m WHERE m.id = NEW.match_id AND m.prediction_item_id = NEW.item_id)
         AND NEW.home_score IS NOT NULL AND NEW.away_score IS NOT NULL)
        OR (kind = 'single_choice' AND NEW.match_id IS NULL
            AND NEW.home_score IS NULL AND NEW.away_score IS NULL)
    )
)
BEGIN SELECT RAISE(ABORT, 'prediction incompatível com item'); END;

CREATE TABLE custom_questions (
    item_id TEXT PRIMARY KEY REFERENCES prediction_items(id) ON DELETE CASCADE,
    points INTEGER NOT NULL DEFAULT 0 CHECK (points >= 0),
    correct_option_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE custom_question_options (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES custom_questions(item_id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(item_id, sort_order)
);

CREATE TRIGGER custom_question_is_single_choice
BEFORE INSERT ON custom_questions
WHEN NOT EXISTS (SELECT 1 FROM prediction_items WHERE id = NEW.item_id AND kind = 'single_choice')
BEGIN SELECT RAISE(ABORT, 'custom question requer item single_choice'); END;

CREATE TRIGGER custom_question_correct_option_valid_insert
BEFORE INSERT ON custom_questions
WHEN NEW.correct_option_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM custom_question_options o WHERE o.id = NEW.correct_option_id AND o.item_id = NEW.item_id
)
BEGIN SELECT RAISE(ABORT, 'opção correta não pertence à pergunta'); END;

CREATE TRIGGER custom_question_correct_option_valid_update
BEFORE UPDATE OF correct_option_id ON custom_questions
WHEN NEW.correct_option_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM custom_question_options o WHERE o.id = NEW.correct_option_id AND o.item_id = NEW.item_id
)
BEGIN SELECT RAISE(ABORT, 'opção correta não pertence à pergunta'); END;

CREATE TABLE custom_prediction_values (
    prediction_id TEXT PRIMARY KEY REFERENCES predictions(id) ON DELETE CASCADE,
    option_id TEXT NOT NULL REFERENCES custom_question_options(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER custom_prediction_value_valid_insert
BEFORE INSERT ON custom_prediction_values
WHEN NOT EXISTS (
    SELECT 1 FROM predictions p
    JOIN prediction_items pi ON pi.id = p.item_id
    JOIN custom_question_options o ON o.id = NEW.option_id AND o.item_id = p.item_id
    WHERE p.id = NEW.prediction_id AND pi.kind = 'single_choice' AND p.match_id IS NULL
)
BEGIN SELECT RAISE(ABORT, 'opção incompatível com prediction'); END;

CREATE TRIGGER custom_prediction_value_valid_update
BEFORE UPDATE OF prediction_id, option_id ON custom_prediction_values
WHEN NOT EXISTS (
    SELECT 1 FROM predictions p
    JOIN prediction_items pi ON pi.id = p.item_id
    JOIN custom_question_options o ON o.id = NEW.option_id AND o.item_id = p.item_id
    WHERE p.id = NEW.prediction_id AND pi.kind = 'single_choice' AND p.match_id IS NULL
)
BEGIN SELECT RAISE(ABORT, 'opção incompatível com prediction'); END;
