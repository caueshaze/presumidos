import { Trophy, Target, CheckCircle2, Star } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { useMatches, useMyMatchPoints, useMyPredictions, usePools } from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card, MotionCard } from "@/components/ui/card";

type StatCardProps = {
  icon: typeof Target;
  value: string;
  label: string;
  detail?: string;
};

function StatCard({ icon: Icon, value, label, detail }: StatCardProps) {
  return (
    <Card className="border border-mint-dark/15 bg-card/85 text-center">
      <Icon className="mx-auto h-5 w-5 text-mint-dark" aria-hidden="true" />
      <strong className="mt-2 block font-heading text-3xl text-ink">{value}</strong>
      <span className="mt-1 block text-sm font-semibold text-ink">{label}</span>
      {detail && <span className="mt-1 block text-xs text-ink-muted">{detail}</span>}
    </Card>
  );
}

/** Resultado final e retrospectiva pessoal. Cada jogo é contado uma única vez. */
export function CupClosingPage() {
  const navigate = useNavigate();
  const pools = usePools();
  const matches = useMatches();
  const predictions = useMyPredictions();
  const matchPoints = useMyMatchPoints();

  const predictionByMatch = new Map((predictions.data ?? []).map((prediction) => [prediction.matchId, prediction]));
  const settledPredictions = (matches.data ?? []).flatMap((match) => {
    const prediction = predictionByMatch.get(match.id);
    if (!prediction || match.homeScore === null || match.awayScore === null) return [];
    return [{ match, prediction }];
  });
  const pointsByMatch = new Map((matchPoints.data ?? []).map((summary) => [summary.matchId, summary]));
  const correctResults = settledPredictions.filter(({ match, prediction }) => {
    const officialOutcome = Math.sign(match.homeScore! - match.awayScore!);
    const predictedOutcome = Math.sign(prediction.homeScore - prediction.awayScore);
    return officialOutcome === predictedOutcome;
  }).length;
  const exactScores = settledPredictions.filter(
    ({ match, prediction }) =>
      match.homeScore === prediction.homeScore && match.awayScore === prediction.awayScore,
  ).length;
  const totalPoints = settledPredictions.reduce(
    (total, { match }) => total + (pointsByMatch.get(match.id)?.totalPoints ?? 0),
    0,
  );
  const correctPercent =
    settledPredictions.length === 0 ? 0 : Math.round((correctResults / settledPredictions.length) * 100);
  const loading = pools.isLoading || matches.isLoading || predictions.isLoading || matchPoints.isLoading;

  return (
    <PageShell>
      <section className="mx-auto max-w-4xl text-center">
        <div className="inline-flex items-center gap-2 rounded-pill bg-yellow/35 px-4 py-2 text-sm font-semibold text-ink">
          <Trophy className="h-4 w-4 text-yellow-dark" aria-hidden="true" />
          Copa encerrada
        </div>
        <div className="mt-5 rounded-[32px] border border-yellow-dark/30 bg-card/85 px-6 py-9 shadow-card sm:px-12">
          <span className="text-6xl" role="img" aria-label="Bandeira da Espanha">🇪🇸</span>
          <h1 className="mt-4 text-4xl sm:text-5xl">A Espanha é campeã!</h1>
          <p className="mx-auto mt-3 max-w-2xl text-base text-ink-muted sm:text-lg">
            A Copa chegou ao fim. Confira como foram seus palpites e veja a classificação da galera.
          </p>
        </div>
      </section>

      <section className="mx-auto mt-8 max-w-4xl" aria-labelledby="my-cup-stats">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h2 id="my-cup-stats" className="text-2xl">Sua campanha</h2>
        </div>
        {loading ? (
          <Card className="mt-4"><p className="text-ink-muted">Calculando suas estatísticas...</p></Card>
        ) : settledPredictions.length === 0 ? (
          <Card className="mt-4">
            <h3 className="text-lg">Ainda não há palpites apurados</h3>
            <p className="mt-1 text-ink-muted">Quando houver resultados oficiais para seus palpites, seu resumo aparecerá aqui.</p>
          </Card>
        ) : (
          <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <StatCard icon={Target} value={String(settledPredictions.length)} label="palpites apurados" />
            <StatCard icon={CheckCircle2} value={String(correctResults)} label="resultados corretos" detail={`${correctPercent}% de acerto`} />
            <StatCard icon={Star} value={String(exactScores)} label="placares exatos" />
            <StatCard icon={Trophy} value={`${totalPoints} pts`} label="pontos na Copa" />
          </div>
        )}
      </section>

      <section className="mx-auto mt-8 max-w-4xl" aria-labelledby="pool-rankings">
        <h2 id="pool-rankings" className="text-2xl">Rankings dos seus bolões</h2>
        <p className="mt-1 text-ink-muted">Abra um bolão para ver a classificação final.</p>
        {pools.isLoading ? (
          <Card className="mt-4"><p className="text-ink-muted">Carregando bolões...</p></Card>
        ) : pools.isError ? (
          <Card className="mt-4"><p className="text-danger">Não foi possível carregar seus bolões.</p></Card>
        ) : pools.data?.length === 0 ? (
          <Card className="mt-4"><p className="text-ink-muted">Você não participou de nenhum bolão nesta Copa.</p></Card>
        ) : (
          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            {pools.data?.map((pool, index) => (
              <MotionCard key={pool.id} transition={{ delay: index * 0.05, duration: 0.25 }} className="flex items-center justify-between gap-4">
                <div className="min-w-0">
                  <h3 className="truncate text-lg">{pool.name}</h3>
                  <p className="mt-1 text-sm text-ink-muted">{pool.memberCount} participante(s)</p>
                </div>
                <Button size="sm" onClick={() => navigate(`/leaderboard?poolId=${encodeURIComponent(pool.id)}&from=closing`)}>
                  Ver ranking
                </Button>
              </MotionCard>
            ))}
          </div>
        )}
      </section>
    </PageShell>
  );
}
