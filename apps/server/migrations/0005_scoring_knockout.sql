-- Campos extras para a pontuação de mata-mata (classificado + pênaltis).
-- `qualifier` guarda 'home' ou 'away' (relativo ao confronto), NULL em jogos de grupo.

ALTER TABLE matches ADD COLUMN qualifier TEXT;
ALTER TABLE matches ADD COLUMN went_to_penalties INTEGER NOT NULL DEFAULT 0;
ALTER TABLE matches ADD COLUMN penalty_home_score INTEGER;
ALTER TABLE matches ADD COLUMN penalty_away_score INTEGER;

ALTER TABLE predictions ADD COLUMN qualifier TEXT;
ALTER TABLE predictions ADD COLUMN went_to_penalties INTEGER NOT NULL DEFAULT 0;
ALTER TABLE predictions ADD COLUMN penalty_home_score INTEGER;
ALTER TABLE predictions ADD COLUMN penalty_away_score INTEGER;
