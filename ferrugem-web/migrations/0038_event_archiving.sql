ALTER TABLE events ADD COLUMN archived_at TEXT;
ALTER TABLE events ADD COLUMN archived_by TEXT REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX idx_events_catalog_active
    ON events(status, archived_at, starts_at);
