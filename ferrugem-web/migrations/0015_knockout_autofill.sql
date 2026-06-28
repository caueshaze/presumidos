-- Auto-detecção de resultado de mata-mata via provedor público de placares.
--
-- Diferente da fase de grupos, no mata-mata o resultado oficial depende de quem
-- se classificou e do placar dos pênaltis. O poller passa a CALCULAR o recorte
-- completo (placar + classificado + pênaltis, estes via o endpoint `summary`) e
-- grava estas colunas `auto_*` para auditoria/revisão. Quando os dados são
-- completos e coerentes, o jogo pode ser finalizado automaticamente; em conflito
-- ou dado incompleto, fica pendente para o admin revisar.
--
-- "Pendente de confirmação" é DERIVADO (não tem coluna própria): é um jogo de
-- mata-mata com auto_detected_at preenchido, ainda não finalizado e cuja origem
-- não é 'manual'. Ao confirmar/corrigir manualmente, vira result_source='manual'
-- + finished=1 e sai do estado pendente.

-- Sugestão auto-detectada (NÃO é o resultado oficial).
ALTER TABLE matches ADD COLUMN auto_home_score INTEGER;
ALTER TABLE matches ADD COLUMN auto_away_score INTEGER;
ALTER TABLE matches ADD COLUMN auto_penalty_home_score INTEGER;
ALTER TABLE matches ADD COLUMN auto_penalty_away_score INTEGER;
-- 'home' ou 'away' — quem a fonte indica como classificado.
ALTER TABLE matches ADD COLUMN auto_qualifier TEXT;
-- status.type.name da fonte no momento da detecção (ex.: STATUS_FULL_TIME, STATUS_FINAL_PEN).
ALTER TABLE matches ADD COLUMN auto_status TEXT;
ALTER TABLE matches ADD COLUMN auto_detected_at TEXT;

-- Instrumentação / debug. Enxuto de propósito: o raw é um recorte combinado
-- (scoreboard_event + summary_shootout + fetched_at), não o summary inteiro.
ALTER TABLE matches ADD COLUMN source_home_team_id TEXT;
ALTER TABLE matches ADD COLUMN source_away_team_id TEXT;
ALTER TABLE matches ADD COLUMN source_last_checked_at TEXT;
ALTER TABLE matches ADD COLUMN source_last_status TEXT;
ALTER TABLE matches ADD COLUMN source_last_payload_hash TEXT;
ALTER TABLE matches ADD COLUMN source_raw_payload TEXT;
