-- no-transaction
-- Fase 9: conjuntos de opções para perguntas multiple_choice. A sequência
-- 0027 já foi consumida pela migração de reactions da Fase 8, por isso esta
-- evolução incremental usa 0028 e preserva todo o histórico.
PRAGMA writable_schema = ON;
UPDATE sqlite_master SET sql = replace(sql, "'football_match', 'single_choice', 'numeric'", "'football_match', 'single_choice', 'numeric', 'multiple_choice'")
WHERE type = 'table' AND name = 'prediction_items';
-- Options eram referenciadas exclusivamente por custom_questions. Agora a
-- mesma tabela é compartilhada por dois kinds baseados em options.
UPDATE sqlite_master SET sql = replace(sql, 'REFERENCES custom_questions(item_id)', 'REFERENCES prediction_items(id)')
WHERE type = 'table' AND name = 'custom_question_options';
PRAGMA writable_schema = OFF;

CREATE TABLE multiple_choice_questions (
    item_id TEXT PRIMARY KEY REFERENCES prediction_items(id) ON DELETE CASCADE,
    min_selections INTEGER NOT NULL DEFAULT 1 CHECK(min_selections >= 1),
    max_selections INTEGER CHECK(max_selections IS NULL OR max_selections >= min_selections),
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    updated_at TEXT NOT NULL DEFAULT(datetime('now'))
);
CREATE TRIGGER multiple_choice_question_kind BEFORE INSERT ON multiple_choice_questions
WHEN NOT EXISTS (SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND kind='multiple_choice')
BEGIN SELECT RAISE(ABORT,'multiple choice question requer item multiple_choice'); END;

CREATE TABLE multiple_choice_prediction_options (
    prediction_id TEXT NOT NULL REFERENCES predictions(id) ON DELETE CASCADE,
    option_id TEXT NOT NULL REFERENCES custom_question_options(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    PRIMARY KEY(prediction_id, option_id)
);
CREATE TRIGGER multiple_choice_prediction_option_valid BEFORE INSERT ON multiple_choice_prediction_options
WHEN NOT EXISTS (
    SELECT 1 FROM predictions p
    JOIN prediction_items pi ON pi.id=p.item_id
    JOIN custom_question_options o ON o.id=NEW.option_id AND o.item_id=p.item_id
    WHERE p.id=NEW.prediction_id AND pi.kind='multiple_choice' AND p.match_id IS NULL
)
BEGIN SELECT RAISE(ABORT,'opção multiple choice incompatível com prediction'); END;

CREATE TABLE multiple_choice_results (
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    option_id TEXT NOT NULL REFERENCES custom_question_options(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    PRIMARY KEY(item_id, option_id)
);
CREATE TRIGGER multiple_choice_result_option_valid BEFORE INSERT ON multiple_choice_results
WHEN NOT EXISTS (
    SELECT 1 FROM prediction_items pi JOIN custom_question_options o ON o.id=NEW.option_id AND o.item_id=NEW.item_id
    WHERE pi.id=NEW.item_id AND pi.kind='multiple_choice'
)
BEGIN SELECT RAISE(ABORT,'opção multiple choice incompatível com resultado'); END;

CREATE TABLE multiple_choice_pool_item_scoring (
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    exact_points INTEGER NOT NULL CHECK(exact_points BETWEEN 0 AND 1000),
    partial_points INTEGER NOT NULL CHECK(partial_points BETWEEN 0 AND 1000),
    incorrect_points INTEGER NOT NULL CHECK(incorrect_points BETWEEN 0 AND 1000),
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    updated_at TEXT NOT NULL DEFAULT(datetime('now')),
    PRIMARY KEY(pool_id,item_id)
);
CREATE TRIGGER multiple_choice_pool_scoring_valid BEFORE INSERT ON multiple_choice_pool_item_scoring
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id AND pi.kind='multiple_choice'
)
BEGIN SELECT RAISE(ABORT,'configuração multiple choice incompatível com pool/item'); END;
CREATE TRIGGER multiple_choice_scoring_defaults_after_pool AFTER INSERT ON pools BEGIN
    INSERT INTO multiple_choice_pool_item_scoring(pool_id,item_id,exact_points,partial_points,incorrect_points)
    SELECT NEW.id,q.item_id,1,0,0 FROM prediction_items pi JOIN multiple_choice_questions q ON q.item_id=pi.id
    WHERE pi.event_id=NEW.event_id;
END;
CREATE TRIGGER multiple_choice_scoring_defaults_after_question AFTER INSERT ON multiple_choice_questions BEGIN
    INSERT INTO multiple_choice_pool_item_scoring(pool_id,item_id,exact_points,partial_points,incorrect_points)
    SELECT p.id,NEW.item_id,1,0,0 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
    WHERE pi.id=NEW.item_id;
END;

CREATE TABLE multiple_choice_prediction_score_breakdowns (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    outcome TEXT NOT NULL CHECK(outcome IN ('exact','partial','incorrect')),
    selected_count INTEGER NOT NULL,
    correct_count INTEGER NOT NULL,
    intersection_count INTEGER NOT NULL,
    exact_points INTEGER NOT NULL,
    partial_points INTEGER NOT NULL,
    incorrect_points INTEGER NOT NULL,
    total_points INTEGER NOT NULL,
    eligible INTEGER NOT NULL DEFAULT 0,
    eligibility_reason TEXT NOT NULL DEFAULT '',
    computed_at TEXT NOT NULL DEFAULT(datetime('now')),
    UNIQUE(pool_id,user_id,item_id)
);

-- Expande o trigger genérico sem tocar suas migrations de origem.
DROP TRIGGER predictions_kind_consistent_insert;
DROP TRIGGER predictions_kind_consistent_update;
CREATE TRIGGER predictions_kind_consistent_insert BEFORE INSERT ON predictions
WHEN NOT EXISTS (SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id)
OR NOT EXISTS (SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND (
    (kind='football_match' AND NEW.match_id IS NOT NULL AND EXISTS (SELECT 1 FROM matches m WHERE m.id=NEW.match_id AND m.prediction_item_id=NEW.item_id) AND NEW.home_score IS NOT NULL AND NEW.away_score IS NOT NULL)
    OR (kind IN ('single_choice','numeric','multiple_choice') AND NEW.match_id IS NULL AND NEW.home_score IS NULL AND NEW.away_score IS NULL)
))
BEGIN SELECT RAISE(ABORT,'prediction incompatível com item'); END;
CREATE TRIGGER predictions_kind_consistent_update BEFORE UPDATE OF pool_id,item_id,match_id,home_score,away_score ON predictions
WHEN NOT EXISTS (SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id)
OR NOT EXISTS (SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND (
    (kind='football_match' AND NEW.match_id IS NOT NULL AND EXISTS (SELECT 1 FROM matches m WHERE m.id=NEW.match_id AND m.prediction_item_id=NEW.item_id) AND NEW.home_score IS NOT NULL AND NEW.away_score IS NOT NULL)
    OR (kind IN ('single_choice','numeric','multiple_choice') AND NEW.match_id IS NULL AND NEW.home_score IS NULL AND NEW.away_score IS NULL)
))
BEGIN SELECT RAISE(ABORT,'prediction incompatível com item'); END;
