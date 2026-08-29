-- EventVersion carries the editorial default. A Pool may inherit, replace or
-- disable it without affecting any other Pool bound to the same version.
ALTER TABLE prediction_items ADD COLUMN tie_break_priority INTEGER;
CREATE UNIQUE INDEX idx_prediction_items_version_tiebreak_priority
    ON prediction_items(event_version_id, tie_break_priority)
    WHERE tie_break_priority IS NOT NULL;

CREATE TRIGGER prediction_items_compact_tiebreak_after_delete
AFTER DELETE ON prediction_items
WHEN OLD.tie_break_priority IS NOT NULL
BEGIN
    UPDATE prediction_items
    SET tie_break_priority = tie_break_priority - 1
    WHERE event_version_id=OLD.event_version_id AND tie_break_priority > OLD.tie_break_priority;
END;

CREATE TABLE pool_tiebreak_configs (
    pool_id TEXT PRIMARY KEY REFERENCES pools(id) ON DELETE CASCADE,
    mode TEXT NOT NULL DEFAULT 'inherit' CHECK(mode IN ('inherit','custom','disabled')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE pool_tiebreak_items (
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL CHECK(priority >= 0),
    PRIMARY KEY(pool_id, item_id),
    UNIQUE(pool_id, priority)
);

CREATE TRIGGER pool_tiebreak_items_validate_insert
BEFORE INSERT ON pool_tiebreak_items
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id
)
BEGIN SELECT RAISE(ABORT,'desempate incompatível com pool/item'); END;

CREATE TRIGGER pool_tiebreak_default_after_pool
AFTER INSERT ON pools
BEGIN
    INSERT INTO pool_tiebreak_configs(pool_id,mode) VALUES(NEW.id,'inherit');
END;
