-- no-transaction
-- Reações pertencem à Prediction, não a uma partida. O match era apenas o
-- primeiro tipo de item que possuía Prediction; mantemos os dados históricos
-- via backfill fail-closed antes de remover a coluna antiga.
CREATE TABLE prediction_reactions_new (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    prediction_id TEXT NOT NULL REFERENCES predictions(id) ON DELETE CASCADE,
    target_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reactor_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    updated_at TEXT NOT NULL DEFAULT(datetime('now')),
    UNIQUE(pool_id,prediction_id,reactor_user_id)
);
INSERT INTO prediction_reactions_new(id,pool_id,prediction_id,target_user_id,reactor_user_id,emoji,created_at,updated_at)
SELECT r.id,r.pool_id,p.id,r.target_user_id,r.reactor_user_id,r.emoji,r.created_at,r.updated_at
FROM prediction_reactions r JOIN predictions p ON p.pool_id=r.pool_id AND p.user_id=r.target_user_id AND p.match_id=r.match_id;
DROP TABLE prediction_reactions;
ALTER TABLE prediction_reactions_new RENAME TO prediction_reactions;
CREATE INDEX idx_prediction_reactions_target ON prediction_reactions(pool_id,target_user_id,prediction_id);
