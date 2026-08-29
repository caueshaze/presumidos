-- Lifecycle local do Pool. Estes timestamps nunca alteram a EventVersion.
ALTER TABLE pools ADD COLUMN predictions_closed_at TEXT;
ALTER TABLE pools ADD COLUMN closed_at TEXT;

CREATE INDEX idx_pools_lifecycle ON pools(predictions_closed_at, closed_at);
