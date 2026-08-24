-- Football definitions follow EventVersion too. Match outcomes remain in
-- matches and are deliberately not copied when a working revision is made.
ALTER TABLE matches ADD COLUMN event_version_id TEXT REFERENCES event_versions(id);
ALTER TABLE matches ADD COLUMN source_match_id TEXT REFERENCES matches(id);

UPDATE matches
SET event_version_id = (
    SELECT pi.event_version_id
    FROM prediction_items pi
    WHERE pi.id = matches.prediction_item_id
)
WHERE event_version_id IS NULL;

CREATE INDEX matches_event_version_idx ON matches(event_version_id, kickoff, id);
CREATE INDEX matches_source_match_idx ON matches(source_match_id);

CREATE TRIGGER matches_event_version_backfill_after_insert
AFTER INSERT ON matches
WHEN NEW.event_version_id IS NULL
BEGIN
    UPDATE matches
    SET event_version_id = (
        SELECT pi.event_version_id
        FROM prediction_items pi
        WHERE pi.id = NEW.prediction_item_id
    )
    WHERE id = NEW.id;
END;

CREATE TRIGGER matches_event_version_item_guard
BEFORE INSERT ON matches
WHEN NEW.event_version_id IS NOT NULL
 AND NOT EXISTS (
     SELECT 1 FROM prediction_items
     WHERE id = NEW.prediction_item_id
       AND event_version_id = NEW.event_version_id
 )
BEGIN
    SELECT RAISE(ABORT, 'partida incompatível com a EventVersion do item');
END;

CREATE TRIGGER matches_event_version_item_update_guard
BEFORE UPDATE OF event_version_id,prediction_item_id ON matches
WHEN NEW.event_version_id IS NOT NULL
 AND NOT EXISTS (
     SELECT 1 FROM prediction_items
     WHERE id = NEW.prediction_item_id
       AND event_version_id = NEW.event_version_id
 )
BEGIN
    SELECT RAISE(ABORT, 'partida incompatível com a EventVersion do item');
END;
