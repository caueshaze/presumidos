// @ts-nocheck
import { motion } from "framer-motion";
import { ChevronLeft } from "lucide-react";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label, Select, ErrorBanner } from "@/components/ui/field";
import { initials, PredictionDetail } from "./PredictionDetail";

export function FootballPredictionsView({ context }: { context: Record<string, any> }) {
  const { pools, user, navigate, selectedPool, setSelectedPool, members, matches, entries,
    selectedMember, selectedMemberScore, correctPercentage, setSelectedMemberId, openedFromClosing,
    matchIdParam, matchById, breakdownByKey, reactToPrediction, openReactionMatchId,
    setOpenReactionMatchId, showPoolSelector, currentPool } = context;
  return (
    <PageShell>
      <Button variant="link" size="sm" onClick={() => navigate(selectedPool ? `/pools/${selectedPool}` : "/pools")}>← Voltar ao bolão</Button>
      <h1 className="text-3xl">{showPoolSelector ? "Palpites do Bolão" : "Palpiteiros do Bolão"}</h1>
      {currentPool && <p className="mt-1 text-ink-muted">Bolão: {currentPool.name} · Evento: {currentPool.event.name}</p>}
      <p className="mt-2 max-w-3xl text-sm text-ink-muted">
        Veja os palpites dos outros participantes do bolão e compare com os seus. Os palpites
        aparecem aqui assim que os jogos começam, e mostram os pontos que cada um está somando no
        bolão.
      </p>

      {pools.isLoading ? (
        <Card className="mt-6">
          <p className="text-ink-muted">Carregando...</p>
        </Card>
      ) : pools.isError ? (
        <div className="mt-6">
          <ErrorBanner>Erro ao carregar bolões: {(pools.error as Error).message}</ErrorBanner>
        </div>
      ) : pools.data && pools.data.length === 0 ? (
        <Card className="mt-6">
          <h3 className="text-lg">Você ainda não está em nenhum bolão.</h3>
          <p className="mt-1 text-ink-muted">
            Crie um bolão ou entre com um código para ver os palpites da turma.
          </p>
        </Card>
      ) : (
        <>
          {showPoolSelector && <Card className="mt-6 max-w-sm">
            <Label htmlFor="pool-select">Bolão</Label>
            <Select
              id="pool-select"
              value={selectedPool}
              onChange={(e) => setSelectedPool(e.target.value)}
            >
              {pools.data?.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </Card>}

          <div className="mt-6">
            {members.isLoading || matches.isLoading ? (
              <Card>
                <p className="text-ink-muted">Carregando...</p>
              </Card>
            ) : members.isError ? (
              <ErrorBanner>
                Erro ao carregar palpites: {(members.error as Error).message}
              </ErrorBanner>
            ) : selectedMember ? (
              <div>
                <button
                  type="button"
                  onClick={() => navigate(`/pools/${encodeURIComponent(selectedPool)}/leaderboard${openedFromClosing ? "?from=closing" : ""}`)}
                  className="mb-4 inline-flex items-center gap-1 text-sm font-semibold text-ink-muted transition-colors hover:text-ink"
                >
                  <ChevronLeft className="h-4 w-4" /> Voltar
                </button>

                <Card>
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-3">
                      <span className="flex h-12 w-12 items-center justify-center rounded-full bg-mint/40 font-heading text-lg font-bold text-mint-dark">
                        {initials(selectedMember.username)}
                      </span>
                      <div>
                        <h2 className="font-heading text-xl">{selectedMember.username}</h2>
                        <p className="text-sm text-ink-muted">
                          {selectedMember.predictions.length}{" "}
                          {selectedMember.predictions.length === 1
                            ? "palpite visível"
                            : "palpites visíveis"}
                        </p>
                      </div>
                    </div>
                    {selectedMember.unreadReactionCount > 0 && (
                      <span className="rounded-pill bg-sky/15 px-3 py-1 text-xs font-semibold text-sky-dark ring-1 ring-sky/35">
                        {selectedMember.unreadReactionCount} nova
                        {selectedMember.unreadReactionCount > 1 ? "s" : ""} reacao
                      </span>
                    )}
                  </div>

                  <div className="mt-4">
                    {selectedMemberScore && (
                      <div className="mb-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
                        <div className="rounded-lg bg-mint/15 p-3">
                          <div className="text-lg font-heading font-bold text-mint-dark">
                            {selectedMemberScore.points}
                          </div>
                          <div className="text-xs text-ink-muted">pontos</div>
                        </div>
                        <div className="rounded-lg bg-mint/15 p-3">
                          <div className="text-lg font-heading font-bold text-mint-dark">
                            {selectedMemberScore.correctResults}
                          </div>
                          <div className="text-xs text-ink-muted">resultados corretos</div>
                        </div>
                        <div className="rounded-lg bg-mint/15 p-3">
                          <div className="text-lg font-heading font-bold text-mint-dark">
                            {selectedMemberScore.exactScores}
                          </div>
                          <div className="text-xs text-ink-muted">placares exatos</div>
                        </div>
                        <div className="rounded-lg bg-mint/15 p-3">
                          <div className="text-lg font-heading font-bold text-mint-dark">
                            {correctPercentage}%
                          </div>
                          <div className="text-xs text-ink-muted">taxa de acerto</div>
                        </div>
                      </div>
                    )}
                    {selectedMember.predictions.length === 0 ? (
                      <p className="text-sm text-ink-muted">
                        Os palpites aparecem aqui assim que os jogos começam.
                      </p>
                    ) : (
                      selectedMember.predictions.map((prediction) => (
                        <PredictionDetail
                          key={prediction.matchId}
                          poolId={selectedPool}
                          targetUserId={selectedMember.userId}
                          prediction={prediction}
                          game={matchById.get(prediction.matchId)}
                          breakdown={breakdownByKey.get(
                            `${selectedMember.userId}:${prediction.matchId}`,
                          )}
                          highlight={matchIdParam === prediction.matchId}
                          canReact={selectedMember.userId !== user?.id}
                          reactPending={reactToPrediction.isPending}
                          isPickerOpen={openReactionMatchId === prediction.matchId}
                          onTogglePicker={(matchId) =>
                            setOpenReactionMatchId((current) =>
                              current === matchId ? null : matchId,
                            )
                          }
                          onReact={(vars) => reactToPrediction.mutate(vars)}
                        />
                      ))
                    )}
                  </div>
                </Card>
              </div>
            ) : entries.length === 0 ? (
              <Card>
                <h3 className="text-lg">Ninguém por aqui ainda</h3>
                <p className="mt-1 text-ink-muted">Este bolão não tem membros para mostrar.</p>
              </Card>
            ) : (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {entries.map((member, i) => (
                  <motion.button
                    key={member.userId}
                    type="button"
                    onClick={() => setSelectedMemberId(member.userId)}
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: i * 0.05, duration: 0.3, ease: [0.22, 1, 0.36, 1] }}
                    className="flex items-center gap-3 rounded-lg bg-card p-4 text-left shadow-card transition-shadow hover:shadow-card-hover"
                  >
                    <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-mint/40 font-heading font-bold text-mint-dark">
                      {initials(member.username)}
                    </span>
                    <div className="min-w-0 flex-1">
                      <div className="truncate font-heading font-semibold text-ink">
                        {member.username}
                      </div>
                      <div className="text-xs text-ink-muted">
                        {member.predictions.length}{" "}
                        {member.predictions.length === 1 ? "palpite" : "palpites"}
                      </div>
                    </div>
                    {member.unreadReactionCount > 0 && (
                      <span className="rounded-pill bg-sky/15 px-2.5 py-1 text-xs font-semibold text-sky-dark ring-1 ring-sky/35">
                        {member.unreadReactionCount}
                      </span>
                    )}
                  </motion.button>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </PageShell>
  );
}
