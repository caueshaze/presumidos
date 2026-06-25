-- Limpa os jogos de mata-mata pré-cadastrados na seed (jogo-073..jogo-104).
-- A partir de agora o admin monta os confrontos e horários da fase eliminatória
-- manualmente, então removemos as partidas de mata-mata vindas da seed e quaisquer
-- palpites/breakdowns associados a elas. Os jogos da fase de grupos permanecem.
--
-- "Mata-mata" = qualquer fase que não comece com "fase de grupos" (mesma regra de
-- crate::models::is_knockout).

DELETE FROM prediction_score_breakdowns
 WHERE match_id IN (
   SELECT id FROM matches
    WHERE phase IS NOT NULL
      AND lower(trim(phase)) NOT LIKE 'fase de grupos%'
 );

DELETE FROM predictions
 WHERE match_id IN (
   SELECT id FROM matches
    WHERE phase IS NOT NULL
      AND lower(trim(phase)) NOT LIKE 'fase de grupos%'
 );

DELETE FROM matches
 WHERE phase IS NOT NULL
   AND lower(trim(phase)) NOT LIKE 'fase de grupos%';
