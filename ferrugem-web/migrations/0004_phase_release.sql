-- Controle de liberação de fases do bolão.
-- Enquanto `knockout_released` = '0', os jogos de mata-mata (tudo que não é
-- 'Fase de grupos') ficam ocultos para os usuários comuns; o admin sempre vê
-- todos e libera tudo de uma vez quando a fase de grupos terminar.

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO app_settings (key, value) VALUES ('knockout_released', '0');
