-- EventVersion: conteúdo publicado imutável e revisões de trabalho.
-- Pools apontam para uma versão compartilhada; não há cópia de conteúdo por Pool.

CREATE TABLE event_versions (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL CHECK(version_number > 0),
    state TEXT NOT NULL CHECK(state IN ('working', 'published')),
    is_current_published INTEGER NOT NULL DEFAULT 0 CHECK(is_current_published IN (0,1)),
    name TEXT NOT NULL,
    description TEXT,
    cover_url TEXT,
    cover_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL,
    external_url TEXT,
    fingerprint TEXT NOT NULL DEFAULT '',
    base_fingerprint TEXT,
    created_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    updated_at TEXT NOT NULL DEFAULT(datetime('now')),
    UNIQUE(event_id, version_number)
);

CREATE UNIQUE INDEX idx_event_versions_current
    ON event_versions(event_id) WHERE is_current_published = 1;
CREATE INDEX idx_event_versions_event_state
    ON event_versions(event_id, state, version_number);

ALTER TABLE events ADD COLUMN pool_creation_enabled INTEGER NOT NULL DEFAULT 1 CHECK(pool_creation_enabled IN (0,1));
ALTER TABLE events ADD COLUMN current_published_version_id TEXT REFERENCES event_versions(id);
ALTER TABLE pools ADD COLUMN event_version_id TEXT REFERENCES event_versions(id);
ALTER TABLE prediction_items ADD COLUMN event_version_id TEXT REFERENCES event_versions(id);

-- Cada Event existente começa com uma definição publicada equivalente ao estado
-- anterior à migração. randomblob é suficiente aqui: o ID só precisa ser
-- estável dentro desta base, não precisa atravessar ambientes.
INSERT INTO event_versions (
    id, event_id, version_number, state, is_current_published,
    name, description, cover_url, cover_asset_id, external_url,
    fingerprint, created_by, created_at, updated_at
)
SELECT
    lower(hex(randomblob(16))), e.id, 1, 'published', 1,
    e.name, e.description, e.cover_url, e.cover_asset_id, e.external_url,
    '', e.created_by, e.created_at, e.updated_at
FROM events e;

UPDATE events
SET current_published_version_id = (
    SELECT v.id FROM event_versions v
    WHERE v.event_id = events.id AND v.is_current_published = 1
);

UPDATE prediction_items
SET event_version_id = (
    SELECT e.current_published_version_id
    FROM events e WHERE e.id = prediction_items.event_id
)
WHERE event_version_id IS NULL;

UPDATE pools
SET event_version_id = (
    SELECT e.current_published_version_id
    FROM events e WHERE e.id = pools.event_id
)
WHERE event_version_id IS NULL;

CREATE INDEX idx_prediction_items_version_sort
    ON prediction_items(event_version_id, sort_order, id);
CREATE INDEX idx_pools_event_version
    ON pools(event_version_id);

-- Compatibilidade para fixtures/integrações legadas que ainda inserem somente
-- event_id. O código de produção passa explicitamente a versão corrente.
CREATE TRIGGER pools_event_version_backfill_after_insert
AFTER INSERT ON pools
WHEN NEW.event_version_id IS NULL
BEGIN
    UPDATE pools SET event_version_id = (
        SELECT current_published_version_id FROM events WHERE id = NEW.event_id
    ) WHERE id = NEW.id;
END;

CREATE TRIGGER prediction_items_version_backfill_after_insert
AFTER INSERT ON prediction_items
WHEN NEW.event_version_id IS NULL
BEGIN
    UPDATE prediction_items SET event_version_id = (
        SELECT current_published_version_id FROM events WHERE id = NEW.event_id
    ) WHERE id = NEW.id;
END;

-- O conteúdo de uma versão publicada não pode ser alterado pela aplicação.
-- A proteção operacional completa fica no serviço de revisão; estes triggers
-- impedem exclusões acidentais de itens já usados por uma versão publicada.
CREATE TRIGGER event_version_published_item_delete_guard
BEFORE DELETE ON prediction_items
WHEN EXISTS (
    SELECT 1 FROM event_versions v
    WHERE v.id = OLD.event_version_id AND v.state = 'published'
)
AND EXISTS (
    SELECT 1 FROM pools p
    WHERE p.event_version_id = OLD.event_version_id
)
BEGIN
    SELECT RAISE(ABORT, 'prediction item de EventVersion publicada é imutável');
END;

-- Recria os vínculos de scoring para a versão do Pool, não apenas para o
-- Event. Isso é essencial quando duas versões têm itens físicos diferentes.
DROP TRIGGER custom_pool_item_scoring_valid_insert;
CREATE TRIGGER custom_pool_item_scoring_valid_insert BEFORE INSERT ON custom_pool_item_scoring
WHEN NOT EXISTS (
    SELECT 1 FROM pools p
    JOIN prediction_items pi ON pi.event_version_id = p.event_version_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id AND pi.kind='single_choice'
)
BEGIN SELECT RAISE(ABORT,'configuração custom incompatível com pool/item'); END;

DROP TRIGGER pool_scoring_defaults_after_pool;
CREATE TRIGGER pool_scoring_defaults_after_pool AFTER INSERT ON pools BEGIN
 INSERT INTO football_pool_scoring VALUES (NEW.id,7,4,3,0,3,datetime('now'),datetime('now'));
 INSERT INTO custom_pool_item_scoring(pool_id,item_id,correct_points,incorrect_points)
 SELECT NEW.id,q.item_id,q.points,0
 FROM prediction_items pi JOIN custom_questions q ON q.item_id=pi.id
 WHERE pi.event_version_id=COALESCE(NEW.event_version_id,
   (SELECT current_published_version_id FROM events WHERE id=NEW.event_id))
   AND pi.kind='single_choice';
END;

DROP TRIGGER pool_scoring_defaults_after_question;
CREATE TRIGGER pool_scoring_defaults_after_question AFTER INSERT ON custom_questions BEGIN
 INSERT INTO custom_pool_item_scoring(pool_id,item_id,correct_points,incorrect_points)
 SELECT p.id,NEW.item_id,NEW.points,0
 FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
 WHERE pi.id=NEW.item_id;
END;

DROP TRIGGER numeric_pool_item_scoring_valid_insert;
CREATE TRIGGER numeric_pool_item_scoring_valid_insert BEFORE INSERT ON numeric_pool_item_scoring
WHEN NOT EXISTS (
    SELECT 1 FROM pools p
    JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id AND pi.kind='numeric'
)
BEGIN SELECT RAISE(ABORT,'configuração numeric incompatível com pool/item'); END;

DROP TRIGGER numeric_scoring_defaults_after_pool;
CREATE TRIGGER numeric_scoring_defaults_after_pool AFTER INSERT ON pools BEGIN
    INSERT INTO numeric_pool_item_scoring(pool_id,item_id,exact_points,tolerance_scaled,within_tolerance_points,incorrect_points)
    SELECT NEW.id,n.item_id,1,0,0,0
    FROM prediction_items pi JOIN numeric_questions n ON n.item_id=pi.id
    WHERE pi.event_version_id=COALESCE(NEW.event_version_id,
      (SELECT current_published_version_id FROM events WHERE id=NEW.event_id))
      AND pi.kind='numeric';
END;

DROP TRIGGER numeric_scoring_defaults_after_question;
CREATE TRIGGER numeric_scoring_defaults_after_question AFTER INSERT ON numeric_questions BEGIN
    INSERT INTO numeric_pool_item_scoring(pool_id,item_id,exact_points,tolerance_scaled,within_tolerance_points,incorrect_points)
    SELECT p.id,NEW.item_id,1,0,0,0
    FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
    WHERE pi.id=NEW.item_id;
END;

DROP TRIGGER multiple_choice_pool_scoring_valid;
CREATE TRIGGER multiple_choice_pool_scoring_valid BEFORE INSERT ON multiple_choice_pool_item_scoring
WHEN NOT EXISTS (
    SELECT 1 FROM pools p
    JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id AND pi.kind='multiple_choice'
)
BEGIN SELECT RAISE(ABORT,'configuração multiple choice incompatível com pool/item'); END;

DROP TRIGGER multiple_choice_scoring_defaults_after_pool;
CREATE TRIGGER multiple_choice_scoring_defaults_after_pool AFTER INSERT ON pools BEGIN
    INSERT INTO multiple_choice_pool_item_scoring(pool_id,item_id,exact_points,partial_points,incorrect_points)
    SELECT NEW.id,q.item_id,1,0,0
    FROM prediction_items pi JOIN multiple_choice_questions q ON q.item_id=pi.id
    WHERE pi.event_version_id=COALESCE(NEW.event_version_id,
      (SELECT current_published_version_id FROM events WHERE id=NEW.event_id));
END;

DROP TRIGGER multiple_choice_scoring_defaults_after_question;
CREATE TRIGGER multiple_choice_scoring_defaults_after_question AFTER INSERT ON multiple_choice_questions BEGIN
    INSERT INTO multiple_choice_pool_item_scoring(pool_id,item_id,exact_points,partial_points,incorrect_points)
    SELECT p.id,NEW.item_id,1,0,0
    FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
    WHERE pi.id=NEW.item_id;
END;

DROP TRIGGER predictions_kind_consistent_insert;
DROP TRIGGER predictions_kind_consistent_update;
CREATE TRIGGER predictions_kind_consistent_insert BEFORE INSERT ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND (
        (kind='football_match' AND NEW.match_id IS NOT NULL AND EXISTS (SELECT 1 FROM matches m WHERE m.id=NEW.match_id AND m.prediction_item_id=NEW.item_id) AND NEW.home_score IS NOT NULL AND NEW.awAY_score IS NOT NULL)
        OR (kind IN ('single_choice','numeric','multiple_choice') AND NEW.match_id IS NULL AND NEW.home_score IS NULL AND NEW.awAY_score IS NULL)
    )
)
BEGIN SELECT RAISE(ABORT,'prediction incompatível com item'); END;
CREATE TRIGGER predictions_kind_consistent_update BEFORE UPDATE OF pool_id,item_id,match_id,home_score,away_score ON predictions
WHEN NOT EXISTS (
    SELECT 1 FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id
    WHERE p.id=NEW.pool_id AND pi.id=NEW.item_id
) OR NOT EXISTS (
    SELECT 1 FROM prediction_items WHERE id=NEW.item_id AND (
        (kind='football_match' AND NEW.match_id IS NOT NULL AND EXISTS (SELECT 1 FROM matches m WHERE m.id=NEW.match_id AND m.prediction_item_id=NEW.item_id) AND NEW.home_score IS NOT NULL AND NEW.awAY_score IS NOT NULL)
        OR (kind IN ('single_choice','numeric','multiple_choice') AND NEW.match_id IS NULL AND NEW.home_score IS NULL AND NEW.awAY_score IS NULL)
    )
)
BEGIN SELECT RAISE(ABORT,'prediction incompatível com item'); END;
