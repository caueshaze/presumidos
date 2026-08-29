import { ChevronDown, Search, SlidersHorizontal, X } from "lucide-react";
import { PageShell } from "@/components/PageShell";
import { KnockoutControl } from "@/components/KnockoutControl";
import { PredictionItemRenderer } from "@/components/PredictionItemRenderer";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { cn, formatKnockoutPhase, isMatchLocked } from "@/lib/utils";

function Chip({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return <button type="button" onClick={onClick} aria-pressed={active} className={cn("rounded-pill px-3 py-1.5 text-xs font-heading font-semibold transition-colors duration-200", "focus-visible:outline-none focus-visible:shadow-glow", active ? "bg-mint-dark text-accent-fg shadow-card" : "border-2 border-mint-dark/30 bg-card/60 text-ink-muted hover:border-mint-dark hover:text-ink")}>{children}</button>;
}

export function FootballPredictionsView({ context }: { context: Record<string, any> }) {
  const { navigate, poolId, currentPool, isAdmin, knockout, isLoading, error, allMatches, filtersOpen, setFiltersOpen, activeFilterCount, visibleMatches, search, setSearch, phases, phaseFilter, setPhaseFilter, finishedCount, hideFinished, setHideFinished, hiddenFinishedCount, clearFilters, predictions, reopenedMatchIds, pointsByMatch, targetMatchId } = context;
  return (
    <PageShell>
      <Button variant="link" size="sm" onClick={() => navigate(poolId ? `/pools/${poolId}` : "/pools")}>
        ← Voltar ao bolão
      </Button>
      <h1 className="text-3xl">Palpites</h1>
      {currentPool && <p className="mt-1 text-ink-muted">{currentPool.name} · {currentPool.event.name}</p>}
      {currentPool?.event.isHistorical && (
        <p className="mt-2 text-sm font-semibold text-mint-dark">
          Edição encerrada — os palpites estão somente para consulta.
        </p>
      )}
      <p className="mt-1 text-ink-muted">
        Dê seu palpite de placar para cada partida antes do apito inicial.
      </p>
      <p className="mt-1 max-w-3xl text-sm text-ink-muted">
        Em jogos de mata-mata, informe o placar final do jogo antes dos pênaltis: 90 minutos se acabar
        no tempo normal, 120 minutos se houver prorrogação. Se o palpite continuar empatado, informe
        também o placar dos pênaltis (que não pode terminar empatado) — é ele que define quem se
        classifica.
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
                  onClick={() => setFiltersOpen((open: boolean) => !open)}
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
                          {phases.map((phase: string) => (
                            <Chip
                              key={phase}
                              active={phaseFilter === phase}
                              onClick={() =>
                                setPhaseFilter((current: string | null) => (current === phase ? null : phase))
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
                            onClick={() => setHideFinished((current: boolean) => !current)}
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
                visibleMatches.map((game: any, i: number) => (
                  <PredictionItemRenderer
                    key={game.id}
                    item={{
                      kind: "football_match",
                      match: {
                        poolId: poolId!,
                        index: i,
                        game,
                        prediction: predictions.data?.find((p: any) => p.matchId === game.id),
                        locked: isMatchLocked(game.kickoff) && !reopenedMatchIds.has(game.id),
                        isAdmin,
                        cardId: `match-card-${game.id}`,
                        highlighted: game.id === targetMatchId,
                        points: pointsByMatch.get(game.id),
                      },
                    }}
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
