-- Defaults são materializados por pool; o scorer nunca consulta estes valores.
-- Versioned after the historical closing-screen migration.
CREATE TABLE football_pool_scoring (
    pool_id TEXT PRIMARY KEY REFERENCES pools(id) ON DELETE CASCADE,
    exact_score_points INTEGER NOT NULL CHECK(exact_score_points BETWEEN 0 AND 1000),
    correct_result_exact_side_points INTEGER NOT NULL CHECK(correct_result_exact_side_points BETWEEN 0 AND 1000),
    correct_result_points INTEGER NOT NULL CHECK(correct_result_points BETWEEN 0 AND 1000),
    incorrect_result_points INTEGER NOT NULL CHECK(incorrect_result_points BETWEEN 0 AND 1000),
    knockout_bonus_points INTEGER NOT NULL CHECK(knockout_bonus_points BETWEEN 0 AND 1000),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
INSERT INTO football_pool_scoring
SELECT id, 7, 4, 3, 0, 3, datetime('now'), datetime('now') FROM pools;

CREATE TABLE custom_pool_item_scoring (
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    correct_points INTEGER NOT NULL CHECK(correct_points BETWEEN 0 AND 1000),
    incorrect_points INTEGER NOT NULL CHECK(incorrect_points BETWEEN 0 AND 1000),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY(pool_id,item_id)
);
INSERT INTO custom_pool_item_scoring (pool_id,item_id,correct_points,incorrect_points)
SELECT p.id, q.item_id, q.points, 0 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id
JOIN custom_questions q ON q.item_id=pi.id WHERE pi.kind='single_choice';
CREATE TRIGGER custom_pool_item_scoring_valid_insert BEFORE INSERT ON custom_pool_item_scoring
WHEN NOT EXISTS (SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id AND pi.kind='single_choice')
BEGIN SELECT RAISE(ABORT,'configuração custom incompatível com pool/item'); END;
CREATE TRIGGER pool_scoring_defaults_after_pool AFTER INSERT ON pools BEGIN
 INSERT INTO football_pool_scoring VALUES (NEW.id,7,4,3,0,3,datetime('now'),datetime('now'));
 INSERT INTO custom_pool_item_scoring(pool_id,item_id,correct_points,incorrect_points)
 SELECT NEW.id,q.item_id,q.points,0 FROM prediction_items pi JOIN custom_questions q ON q.item_id=pi.id WHERE pi.event_id=NEW.event_id AND pi.kind='single_choice';
END;
CREATE TRIGGER pool_scoring_defaults_after_question AFTER INSERT ON custom_questions BEGIN
 INSERT INTO custom_pool_item_scoring(pool_id,item_id,correct_points,incorrect_points)
 SELECT p.id,NEW.item_id,NEW.points,0 FROM pools p JOIN prediction_items pi ON pi.event_id=p.event_id WHERE pi.id=NEW.item_id;
END;

CREATE TABLE custom_prediction_score_breakdowns (
    id TEXT PRIMARY KEY, pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    correct_points INTEGER NOT NULL DEFAULT 0, incorrect_points INTEGER NOT NULL DEFAULT 0, total_points INTEGER NOT NULL DEFAULT 0,
    eligible INTEGER NOT NULL DEFAULT 0, eligibility_reason TEXT NOT NULL DEFAULT '', computed_at TEXT NOT NULL DEFAULT(datetime('now')),
    UNIQUE(pool_id,user_id,item_id)
);
