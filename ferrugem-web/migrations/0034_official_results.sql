-- Official outcomes are attached to the EventVersion that defines the item.
-- The legacy result columns remain as compatibility projections for this
-- rollout, but a result can no longer be interpreted without its version.
DROP INDEX IF EXISTS idx_prediction_items_event_external_key;
CREATE UNIQUE INDEX idx_prediction_items_version_external_key
    ON prediction_items(event_version_id, external_key) WHERE external_key IS NOT NULL;

CREATE TABLE official_results (
    id TEXT PRIMARY KEY,
    event_version_id TEXT NOT NULL REFERENCES event_versions(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES prediction_items(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('single_choice','multiple_choice','numeric')),
    state TEXT NOT NULL DEFAULT 'resolved' CHECK (state IN ('resolved','not_representable','pending_decision')),
    option_id TEXT REFERENCES custom_question_options(id) ON DELETE RESTRICT,
    option_ids_json TEXT,
    value_scaled INTEGER,
    reason TEXT,
    updated_by TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(event_version_id, item_id)
);

CREATE INDEX official_results_version_idx ON official_results(event_version_id, item_id);

-- Legacy fixtures may insert a Pool without the new column and rely on the
-- compatibility backfill trigger. Validate against the same fallback before
-- that AFTER INSERT trigger runs.
DROP TRIGGER custom_pool_item_scoring_valid_insert;
CREATE TRIGGER custom_pool_item_scoring_valid_insert BEFORE INSERT ON custom_pool_item_scoring
WHEN NOT EXISTS (
    SELECT 1 FROM pools p
    JOIN prediction_items pi ON pi.event_version_id=COALESCE(p.event_version_id,(SELECT current_published_version_id FROM events WHERE id=p.event_id))
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id AND pi.kind='single_choice'
)
BEGIN SELECT RAISE(ABORT,'configuração custom incompatível com pool/item'); END;

DROP TRIGGER event_version_published_item_delete_guard;
CREATE TRIGGER event_version_published_item_delete_guard
BEFORE DELETE ON prediction_items
WHEN EXISTS (
    SELECT 1 FROM event_versions v JOIN events e ON e.id=v.event_id
    WHERE v.id=OLD.event_version_id AND v.state='published' AND e.kind='custom'
)
AND EXISTS (
    SELECT 1 FROM pools p WHERE p.event_version_id=OLD.event_version_id
)
BEGIN SELECT RAISE(ABORT,'prediction item de EventVersion publicada é imutável'); END;

-- Keep old SQL fixtures/integrations usable when they create an already
-- active Event directly. The application creation path creates its working
-- revision explicitly, so draft Events are intentionally excluded.
CREATE TRIGGER events_active_version_compat_after_insert
AFTER INSERT ON events
WHEN NEW.status='active' AND NOT EXISTS (SELECT 1 FROM event_versions WHERE event_id=NEW.id)
BEGIN
    INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,created_by)
    VALUES(lower(hex(randomblob(16))),NEW.id,1,'published',1,NEW.name,NEW.description,NEW.cover_url,NEW.cover_asset_id,NEW.external_url,'',NEW.created_by);
    UPDATE events SET current_published_version_id=(SELECT id FROM event_versions WHERE event_id=NEW.id AND version_number=1) WHERE id=NEW.id;
END;

CREATE TRIGGER events_active_version_compat_after_update
AFTER UPDATE OF status ON events
WHEN NEW.status='active' AND NOT EXISTS (SELECT 1 FROM event_versions WHERE event_id=NEW.id)
BEGIN
    INSERT INTO event_versions(id,event_id,version_number,state,is_current_published,name,description,cover_url,cover_asset_id,external_url,fingerprint,created_by)
    VALUES(lower(hex(randomblob(16))),NEW.id,1,'published',1,NEW.name,NEW.description,NEW.cover_url,NEW.cover_asset_id,NEW.external_url,'',NEW.created_by);
    UPDATE events SET current_published_version_id=(SELECT id FROM event_versions WHERE event_id=NEW.id AND version_number=1) WHERE id=NEW.id;
END;

CREATE TRIGGER official_results_item_version_guard
BEFORE INSERT ON official_results
WHEN NOT EXISTS (
    SELECT 1 FROM prediction_items
    WHERE id=NEW.item_id AND event_version_id=NEW.event_version_id
)
BEGIN SELECT RAISE(ABORT,'resultado oficial incompatível com a versão'); END;

CREATE TRIGGER official_results_option_version_guard
BEFORE INSERT ON official_results
WHEN NEW.option_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM custom_question_options o
    JOIN prediction_items pi ON pi.id=o.item_id
    WHERE o.id=NEW.option_id AND pi.event_version_id=NEW.event_version_id
)
BEGIN SELECT RAISE(ABORT,'opção oficial incompatível com a versão'); END;

CREATE TRIGGER official_results_option_version_update_guard
BEFORE UPDATE OF option_id,event_version_id ON official_results
WHEN NEW.option_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM custom_question_options o
    JOIN prediction_items pi ON pi.id=o.item_id
    WHERE o.id=NEW.option_id AND pi.event_version_id=NEW.event_version_id
)
BEGIN SELECT RAISE(ABORT,'opção oficial incompatível com a versão'); END;

INSERT OR IGNORE INTO official_results(id,event_version_id,item_id,kind,state,option_id,updated_at)
SELECT lower(hex(randomblob(16))),pi.event_version_id,pi.id,'single_choice','resolved',q.correct_option_id,datetime('now')
FROM prediction_items pi JOIN custom_questions q ON q.item_id=pi.id
WHERE pi.kind='single_choice' AND q.correct_option_id IS NOT NULL;

INSERT OR IGNORE INTO official_results(id,event_version_id,item_id,kind,state,value_scaled,updated_at)
SELECT lower(hex(randomblob(16))),pi.event_version_id,pi.id,'numeric','resolved',n.result_value_scaled,datetime('now')
FROM prediction_items pi JOIN numeric_questions n ON n.item_id=pi.id
WHERE pi.kind='numeric' AND n.result_value_scaled IS NOT NULL;

INSERT OR IGNORE INTO official_results(id,event_version_id,item_id,kind,state,option_ids_json,updated_at)
SELECT lower(hex(randomblob(16))),pi.event_version_id,pi.id,'multiple_choice','resolved',
       json_group_array(r.option_id),datetime('now')
FROM prediction_items pi JOIN multiple_choice_results r ON r.item_id=pi.id
WHERE pi.kind='multiple_choice'
GROUP BY pi.event_version_id,pi.id;
