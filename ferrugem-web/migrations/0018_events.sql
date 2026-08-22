-- Evento explícito que representa o conteúdo previsto pelos bolões.
-- Nesta fase existe apenas a Copa de 2026; `kind` já reserva o valor `custom`
-- para uma etapa futura, sem introduzir comportamento para ele agora.
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('football', 'custom')),
    status TEXT NOT NULL CHECK (status IN ('draft', 'active', 'finished')),
    created_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    starts_at TEXT,
    ends_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ID UUID estável e determinístico para manter o mesmo formato dos IDs do
-- domínio. O código de aplicação resolve este evento por `slug`.
INSERT INTO events (
    id, name, slug, kind, status, created_by, starts_at, ends_at
) VALUES (
    '8e4cfe71-9123-4bd1-a4a9-989eeb55b77f',
    'Copa do Mundo FIFA 2026',
    'world-cup-2026',
    'football',
    'active',
    NULL,
    '2026-06-11T19:00:00Z',
    '2026-07-19T19:00:00Z'
);

-- SQLite não permite ADD COLUMN REFERENCES com DEFAULT não nulo. Adicionamos a
-- FK nullable, fazemos o backfill e usamos triggers para tornar o vínculo
-- obrigatório sem reconstruir `pools` — o que preserva as FKs de todas as
-- tabelas filhas já existentes.
ALTER TABLE pools ADD COLUMN event_id TEXT REFERENCES events(id);

UPDATE pools
SET event_id = '8e4cfe71-9123-4bd1-a4a9-989eeb55b77f'
WHERE event_id IS NULL;

CREATE TRIGGER pools_event_id_required_insert
BEFORE INSERT ON pools
WHEN NEW.event_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'pools.event_id é obrigatório');
END;

CREATE TRIGGER pools_event_id_required_update
BEFORE UPDATE OF event_id ON pools
WHEN NEW.event_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'pools.event_id é obrigatório');
END;

CREATE INDEX idx_pools_event_id ON pools(event_id);
