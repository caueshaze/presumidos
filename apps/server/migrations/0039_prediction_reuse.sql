-- Fase 18: a inicialização de palpites em um Pool é uma decisão única do
-- membro. Nenhuma Prediction é migrada ou compartilhada por esta migration.
ALTER TABLE pool_members ADD COLUMN prediction_reuse_decision TEXT NOT NULL DEFAULT 'undecided'
    CHECK(prediction_reuse_decision IN ('undecided', 'started_empty', 'copied'));
ALTER TABLE pool_members ADD COLUMN prediction_reuse_decided_at TEXT;
