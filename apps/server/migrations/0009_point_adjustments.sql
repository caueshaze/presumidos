-- Ajustes manuais de pontos lançados pelo organizador do bolão (ou admin).
-- Cada linha é um delta (positivo ou negativo) aplicado a um membro, com motivo.
-- O total do ranking soma esses deltas à pontuação calculada dos palpites.
CREATE TABLE point_adjustments (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    delta INTEGER NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_point_adjustments_pool ON point_adjustments(pool_id);
