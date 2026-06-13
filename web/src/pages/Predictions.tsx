import { useAuth } from "@/hooks/useAuth";
import { useMatches, useMyPredictions, useKnockoutReleased } from "@/hooks/queries";
import { isMatchLocked } from "@/lib/utils";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";
import { KnockoutControl } from "@/components/KnockoutControl";
import { MatchCard } from "@/components/MatchCard";

export function PredictionsPage() {
  const { isAdmin } = useAuth();
  const matches = useMatches();
  const predictions = useMyPredictions();
  const knockout = useKnockoutReleased();

  const isLoading = matches.isLoading || predictions.isLoading || knockout.isLoading;
  const error = matches.error || predictions.error || knockout.error;

  return (
    <PageShell>
      <h1 className="text-3xl">Palpites</h1>
      <p className="mt-1 text-ink-muted">
        Dê seu palpite de placar para cada partida antes do apito inicial.
      </p>
      <p className="mt-1 max-w-3xl text-sm text-ink-muted">
        Em jogos de mata-mata, além do placar no tempo normal, escolha quem se classifica. Se apostar
        em empate no tempo normal, você pode marcar que o jogo vai para os pênaltis e, opcionalmente,
        palpitar o placar da disputa.
      </p>

      <div className="mt-6">
        {isLoading ? (
          <Card>
            <p className="text-ink-muted">Carregando...</p>
          </Card>
        ) : error ? (
          <ErrorBanner>Erro ao carregar partidas: {(error as Error).message}</ErrorBanner>
        ) : (
          <>
            {isAdmin && <KnockoutControl released={knockout.data?.released ?? false} />}
            <div>
              {matches.data?.map((game, i) => (
                <MatchCard
                  key={game.id}
                  index={i}
                  game={game}
                  prediction={predictions.data?.find((p) => p.matchId === game.id)}
                  locked={isMatchLocked(game.kickoff)}
                  isAdmin={isAdmin}
                />
              ))}
            </div>
          </>
        )}
      </div>
    </PageShell>
  );
}
