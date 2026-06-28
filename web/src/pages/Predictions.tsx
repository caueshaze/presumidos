import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { ChevronDown, Search, SlidersHorizontal, X } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import {
  useMatches,
  useMyPredictions,
  useMyPredictionOverrides,
  useMyMatchPoints,
  useKnockoutReleased,
} from "@/hooks/queries";
import { cn, formatKnockoutPhase, isMatchLocked } from "@/lib/utils";
import { formatSelectionLabel } from "@/lib/selections";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";
import { KnockoutControl } from "@/components/KnockoutControl";
import { MatchCard } from "@/components/MatchCard";
import type { MatchPointsSummary, MatchRecord } from "@/types";

// Ordem natural das fases de uma Copa; fases desconhecidas vão para o fim.
const PHASE_ORDER = [
  "Fase de grupos",
  "Oitavas de final",
  "Quartas de final",
  "Semifinal",
  "Disputa de terceiro",
  "Final",
];

function phaseRank(phase: string): number {
  const i = PHASE_ORDER.indexOf(phase);
  return i === -1 ? PHASE_ORDER.length : i;
}

// Remove acentos e caixa para uma busca tolerante ("arabia" acha "Arábia").
function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();
}

function matchesSearch(game: MatchRecord, query: string): boolean {
  if (!query) return true;
  const q = normalize(query);
  const haystack = normalize(
    `${game.homeTeam} ${game.awayTeam} ` +
      `${formatSelectionLabel(game.homeTeam)} ${formatSelectionLabel(game.awayTeam)}`,
  );
  return haystack.includes(q);
}

function Chip({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "rounded-pill px-3 py-1.5 text-xs font-heading font-semibold transition-colors duration-200",
        "focus-visible:outline-none focus-visible:shadow-glow",
        active
          ? "bg-mint-dark text-accent-fg shadow-card"
          : "border-2 border-mint-dark/30 bg-card/60 text-ink-muted hover:border-mint-dark hover:text-ink",
      )}
    >
      {children}
    </button>
  );
}

export function PredictionsPage() {
  const { isAdmin } = useAuth();
  const [searchParams] = useSearchParams();
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [phaseFilter, setPhaseFilter] = useState<string | null>(null);
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

  // Fases presentes nos dados, na ordem natural da competição.
  const phases = useMemo(() => {
    const set = new Set<string>();
    for (const game of allMatches) if (game.phase) set.add(game.phase);
    return [...set].sort((a, b) => phaseRank(a) - phaseRank(b) || a.localeCompare(b));
  }, [allMatches]);

  // Um jogo passa quando bate todos os filtros — mas o jogo do deep-link
  // (?matchId=) é sempre mostrado, para não "sumir" ao chegar por um link.
  const visibleMatches = useMemo(
    () =>
      allMatches.filter((game) => {
        if (game.id === targetMatchId) return true;
        if (hideFinished && game.finished) return false;
        if (phaseFilter && game.phase !== phaseFilter) return false;
        if (!matchesSearch(game, search)) return false;
        return true;
      }),
    [allMatches, hideFinished, phaseFilter, search, targetMatchId],
  );

  const hiddenFinishedCount = useMemo(
    () =>
      hideFinished
        ? allMatches.filter((game) => game.finished && game.id !== targetMatchId).length
        : 0,
    [allMatches, hideFinished, targetMatchId],
  );

  const activeFilterCount =
    (search.trim() ? 1 : 0) + (phaseFilter ? 1 : 0) + (!hideFinished ? 1 : 0);

  const clearFilters = () => {
    setSearch("");
    setPhaseFilter(null);
    setHideFinished(true);
  };

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
        Em jogos de mata-mata, dê o placar do tempo normal. Se você palpitar empate, o jogo vai para os
        pênaltis: informe o placar da disputa (que não pode terminar empatado) — é ele que define quem
        se classifica.
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

            {allMatches.length > 0 && (
              <div className="mb-4 rounded-lg border border-mint/20 bg-card/60">
                <button
                  type="button"
                  onClick={() => setFiltersOpen((open) => !open)}
                  aria-expanded={filtersOpen}
                  className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left focus-visible:outline-none focus-visible:shadow-glow rounded-lg"
                >
                  <span className="flex items-center gap-2 font-heading font-semibold text-ink">
                    <SlidersHorizontal className="h-4 w-4 text-mint-dark" />
                    Filtro
                    {activeFilterCount > 0 && (
                      <span className="inline-flex h-5 min-w-5 items-center justify-center rounded-pill bg-mint-dark px-1.5 text-xs font-bold text-accent-fg">
                        {activeFilterCount}
                      </span>
                    )}
                  </span>
                  <span className="flex items-center gap-2 text-sm text-ink-muted">
                    <span className="hidden sm:inline">
                      {visibleMatches.length} de {allMatches.length}
                    </span>
                    <ChevronDown
                      className={cn(
                        "h-4 w-4 transition-transform duration-200",
                        filtersOpen && "rotate-180",
                      )}
                    />
                  </span>
                </button>

                {filtersOpen && (
                  <div className="space-y-4 border-t border-mint/15 px-4 py-4">
                    {/* Busca por seleção */}
                    <div className="relative">
                      <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-muted" />
                      <Input
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        placeholder="Buscar seleção (ex.: Brasil)"
                        className="pl-10 pr-10"
                        aria-label="Buscar seleção"
                      />
                      {search && (
                        <button
                          type="button"
                          onClick={() => setSearch("")}
                          aria-label="Limpar busca"
                          className="absolute right-2 top-1/2 -translate-y-1/2 rounded-full p-1 text-ink-muted hover:text-ink focus-visible:outline-none focus-visible:shadow-glow"
                        >
                          <X className="h-4 w-4" />
                        </button>
                      )}
                    </div>

                    {/* Fases */}
                    {phases.length > 1 && (
                      <div>
                        <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-ink-muted">
                          Fase
                        </p>
                        <div className="flex flex-wrap gap-2">
                          <Chip active={phaseFilter === null} onClick={() => setPhaseFilter(null)}>
                            Todas
                          </Chip>
                          {phases.map((phase) => (
                            <Chip
                              key={phase}
                              active={phaseFilter === phase}
                              onClick={() =>
                                setPhaseFilter((current) => (current === phase ? null : phase))
                              }
                            >
                              {formatKnockoutPhase(phase)}
                            </Chip>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Finalizados */}
                    {finishedCount > 0 && (
                      <div>
                        <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-ink-muted">
                          Status
                        </p>
                        <div className="flex flex-wrap items-center gap-2">
                          <Chip
                            active={!hideFinished}
                            onClick={() => setHideFinished((current) => !current)}
                          >
                            Incluir finalizados ({finishedCount})
                          </Chip>
                          {hideFinished && hiddenFinishedCount > 0 && (
                            <span className="text-xs text-ink-muted">
                              {hiddenFinishedCount} oculto(s)
                            </span>
                          )}
                        </div>
                      </div>
                    )}

                    {activeFilterCount > 0 && (
                      <Button type="button" variant="link" size="sm" onClick={clearFilters}>
                        Limpar filtros
                      </Button>
                    )}
                  </div>
                )}
              </div>
            )}

            <div>
              {visibleMatches.length === 0 ? (
                <Card>
                  <p className="text-ink-muted">
                    Nenhum jogo com esses filtros.{" "}
                    {hideFinished && finishedCount > 0 && (
                      <button
                        type="button"
                        onClick={() => setHideFinished(false)}
                        className="font-semibold text-mint-dark underline-offset-4 hover:underline"
                      >
                        Incluir finalizados
                      </button>
                    )}
                    {hideFinished && finishedCount > 0 && activeFilterCount > 0 && " ou "}
                    {activeFilterCount > 0 && (
                      <button
                        type="button"
                        onClick={clearFilters}
                        className="font-semibold text-mint-dark underline-offset-4 hover:underline"
                      >
                        limpar filtros
                      </button>
                    )}
                    .
                  </p>
                </Card>
              ) : (
                visibleMatches.map((game, i) => (
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
                ))
              )}
            </div>
          </>
        )}
      </div>
    </PageShell>
  );
}
