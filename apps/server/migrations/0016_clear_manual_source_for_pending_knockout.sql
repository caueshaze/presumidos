-- Confrontos de mata-mata criados pelo admin não são "resultado manual".
--
-- A origem 'manual' deve significar apenas que o resultado oficial foi lançado
-- pelo admin. Confrontos eliminatórios cadastrados antes desta correção nasciam
-- com result_source='manual' mesmo sem placar, o que impedia o poller de gravar
-- a sugestão final aguardando confirmação.
UPDATE matches
   SET result_source = NULL
 WHERE result_source = 'manual'
   AND home_score IS NULL
   AND away_score IS NULL
   AND phase IS NOT NULL
   AND lower(trim(phase)) NOT LIKE 'fase de grupos%';
