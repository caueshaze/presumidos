-- Versioned after the historical closing-screen migration.
CREATE TABLE prediction_items (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(id),
    kind TEXT NOT NULL CHECK (kind IN ('football_match', 'single_choice')),
    title TEXT NOT NULL,
    description TEXT,
    lock_at TEXT NOT NULL,
    reveal_at TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    -- Administrativo nesta fase: tempo e `matches.finished` continuam sendo a
    -- fonte de verdade operacional para football.
    status TEXT NOT NULL CHECK (status IN ('draft', 'open', 'locked', 'resolved')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO prediction_items (
    id, event_id, kind, title, description, lock_at, reveal_at, sort_order, status
)
SELECT
    'prediction-item-' || m.id,
    e.id,
    'football_match',
    m.home_team || ' x ' || m.away_team,
    NULL,
    m.kickoff,
    m.kickoff,
    ROW_NUMBER() OVER (ORDER BY datetime(m.kickoff) ASC, m.id ASC) - 1,
    'open'
FROM matches m
JOIN events e ON e.slug = 'world-cup-2026';

ALTER TABLE matches ADD COLUMN prediction_item_id TEXT REFERENCES prediction_items(id);

UPDATE matches
SET prediction_item_id = 'prediction-item-' || id
WHERE prediction_item_id IS NULL;

CREATE UNIQUE INDEX idx_matches_prediction_item_id ON matches(prediction_item_id);
CREATE INDEX idx_prediction_items_event_sort ON prediction_items(event_id, sort_order);

CREATE TRIGGER matches_prediction_item_required_insert
BEFORE INSERT ON matches
WHEN NEW.prediction_item_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'matches.prediction_item_id é obrigatório');
END;

CREATE TRIGGER matches_prediction_item_required_update
BEFORE UPDATE OF prediction_item_id ON matches
WHEN NEW.prediction_item_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'matches.prediction_item_id é obrigatório');
END;

-- Nesta fase todos os matches são football da Copa. Isso impede uma associação
-- silenciosa a item de outro tipo ou de outro evento.
CREATE TRIGGER matches_prediction_item_football_event_insert
BEFORE INSERT ON matches
WHEN NOT EXISTS (
    SELECT 1
    FROM prediction_items pi
    JOIN events e ON e.id = pi.event_id
    WHERE pi.id = NEW.prediction_item_id
      AND pi.kind = 'football_match'
      AND e.slug = 'world-cup-2026'
)
BEGIN
    SELECT RAISE(ABORT, 'prediction item de football da Copa obrigatório');
END;

CREATE TRIGGER matches_prediction_item_football_event_update
BEFORE UPDATE OF prediction_item_id ON matches
WHEN NOT EXISTS (
    SELECT 1
    FROM prediction_items pi
    JOIN events e ON e.id = pi.event_id
    WHERE pi.id = NEW.prediction_item_id
      AND pi.kind = 'football_match'
      AND e.slug = 'world-cup-2026'
)
BEGIN
    SELECT RAISE(ABORT, 'prediction item de football da Copa obrigatório');
END;
