import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import {
  useMatches,
  useMyPredictions,
  useMyPredictionOverrides,
  useMyMatchPoints,
  useKnockoutReleased,
} from "@/hooks/queries";
import { isMatchLocked } from "@/lib/utils";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/ui/field";
import { KnockoutControl } from "@/components/KnockoutControl";
import { MatchCard } from "@/components/MatchCard";
import type { MatchPointsSummary } from "@/types";

export function PredictionsPage() {
  const { isAdmin } = useAuth();
  const [searchParams] = useSearchParams();
  const [hideFinished, setHideFinished] = useState(true);
  const matches = useMatches();
  const predictions = useMyPredictions();
  const overrides = useMyPredictionOverrides();
  const matchPoints = useMyMatchPoints();
  const knockout = useKnockoutReleased();

  const pointsByMatch = useMemo(() => {
    const map = new Map<string, MatchPointsSummary>();
    for (const p of matchPoints.data ?? []) map.set(p.matchId, p);
    return map;
  }, [matchPoints.data]);

  // Reaberturas administrativas liberam o palpite mesmo com a partida travada por horário.
  const reopenedMatchIds = useMemo(
    () => new Set((overrides.data ?? []).map((o) => o.matchId)),
    [overrides.data],
  );

  const isLoading = matches.isLoading || predictions.isLoading || knockout.isLoading;
  const error = matches.error || predictions.error || knockout.error;
  const targetMatchId = searchParams.get("matchId");
  const allMatches = useMemo(() => matches.data ?? [], [matches.data]);
  const finishedCount = useMemo(
    () => allMatches.filter((game) => game.finished).length,
    [allMatches],
  );
  const hiddenFinishedCount = useMemo(
    () =>
      hideFinished
        ? allMatches.filter((game) => game.finished && game.id !== targetMatchId).length
        : 0,
    [allMatches, hideFinished, targetMatchId],
  );
  const visibleMatches = useMemo(
    () =>
      allMatches.filter((game) => {
        if (!hideFinished) return true;
        if (!game.finished) return true;
        return game.id === targetMatchId;
      }),
    [allMatches, hideFinished, targetMatchId],
  );

  useEffect(() => {
    if (!targetMatchId || visibleMatches.length === 0) return;
    const element = document.getElementById(`match-card-${targetMatchId}`);
    if (!element) return;

    const timer = window.setTimeout(() => {
      element.scrollIntoView({ behavior: "smooth", block: "start" });
    }, 120);

    return () => window.clearTimeout(timer);
  }, [targetMatchId, visibleMatches]);

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
            {finishedCount > 0 && (
              <div className="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-mint/20 bg-card/60 px-4 py-3">
                <div className="text-sm text-ink-muted">
                  {hideFinished ? (
                    <>
                      {visibleMatches.length} jogo(s) em foco agora.
                      {hiddenFinishedCount > 0 && ` ${hiddenFinishedCount} finalizado(s) oculto(s).`}
                    </>
                  ) : (
                    <>
                      Mostrando todos os {allMatches.length} jogo(s), incluindo {finishedCount} finalizado(s).
                    </>
                  )}
                </div>
                <Button
                  type="button"
                  variant={hideFinished ? "primary" : "outline"}
                  size="sm"
                  onClick={() => setHideFinished((current) => !current)}
                >
                  {hideFinished ? "Mostrar finalizados" : "Ocultar finalizados"}
                </Button>
              </div>
            )}
            <div>
              {visibleMatches.map((game, i) => (
                <MatchCard
                  key={game.id}
                  index={i}
                  game={game}
                  prediction={predictions.data?.find((p) => p.matchId === game.id)}
                  locked={isMatchLocked(game.kickoff) && !reopenedMatchIds.has(game.id)}
                  isAdmin={isAdmin}
                  cardId={`match-card-${game.id}`}
                  highlighted={game.id === targetMatchId}
                  points={pointsByMatch.get(game.id)}
                />
              ))}
            </div>
          </>
        )}
      </div>
    </PageShell>
  );
}
