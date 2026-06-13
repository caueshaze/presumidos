-- Estado explícito de "jogo finalizado" (rótulo oficial).
-- O placar continua contando no ranking assim que é salvo; este flag é o
-- indicador visual de que a partida está oficialmente encerrada.
ALTER TABLE matches ADD COLUMN finished INTEGER NOT NULL DEFAULT 0;

-- Jogos que já têm placar oficial passam a aparecer como finalizados.
UPDATE matches SET finished = 1 WHERE home_score IS NOT NULL AND away_score IS NOT NULL;
