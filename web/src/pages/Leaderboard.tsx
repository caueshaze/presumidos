import { useEffect, useState, type FormEvent } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X, Info } from "lucide-react";
import {
  usePools,
  useLeaderboard,
  useMatches,
  usePoolAdjustments,
  useAddAdjustment,
  useRemoveAdjustment,
} from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label, Select, ErrorBanner } from "@/components/ui/field";

const medals = ["🥇", "🥈", "🥉"];

export function LeaderboardPage() {
  const { user, isAdmin } = useAuth();
  const pools = usePools();
  const [selectedPool, setSelectedPool] = useState("");

  useEffect(() => {
    if (!selectedPool && pools.data && pools.data.length > 0) {
      setSelectedPool(pools.data[0].id);
    }
  }, [pools.data, selectedPool]);

  const leaderboard = useLeaderboard(selectedPool || null);
  const matches = useMatches();
  const adjustments = usePoolAdjustments(selectedPool || null);

  // Há jogo em andamento? Aí a pontuação exibida é provisória (ao vivo).
  const liveMatches = (matches.data ?? []).filter((m) => m.liveStatus && !m.finished);
  const hasLive = liveMatches.length > 0;
  const addAdjustment = useAddAdjustment();
  const removeAdjustment = useRemoveAdjustment();

  const entries = leaderboard.data ?? [];
  const podium = entries.slice(0, 3);
  const rest = entries.slice(3);

  const currentPool = pools.data?.find((p) => p.id === selectedPool);
  const isOrganizer = !!currentPool && (currentPool.createdBy === user?.id || isAdmin);

  const [scoringOpen, setScoringOpen] = useState(false);

  // ---- Formulário de ajuste (organizador) ----
  const [adjUser, setAdjUser] = useState("");
  const [adjMode, setAdjMode] = useState<"add" | "subtract">("add");
  const [adjPoints, setAdjPoints] = useState("");
  const [adjReason, setAdjReason] = useState("");
  const [adjError, setAdjError] = useState("");

  useEffect(() => {
    setAdjUser("");
    setAdjMode("add");
    setAdjPoints("");
    setAdjReason("");
    setAdjError("");
  }, [selectedPool]);

  const onAdjust = async (e: FormEvent) => {
    e.preventDefault();
    setAdjError("");
    const points = parseInt(adjPoints, 10);
    if (!adjUser || Number.isNaN(points) || points <= 0) {
      setAdjError("Escolha um membro e uma quantidade de pontos maior que zero.");
      return;
    }
    const delta = adjMode === "subtract" ? -points : points;
    try {
      await addAdjustment.mutateAsync({
        poolId: selectedPool,
        userId: adjUser,
        delta,
        reason: adjReason.trim(),
      });
      setAdjUser("");
      setAdjMode("add");
      setAdjPoints("");
      setAdjReason("");
    } catch (err) {
      setAdjError(err instanceof Error ? err.message : "Falha ao lançar ajuste.");
    }
  };

  const onRemoveAdjustment = async (adjustmentId: string) => {
    setAdjError("");
    try {
      await removeAdjustment.mutateAsync({ poolId: selectedPool, adjustmentId });
    } catch (err) {
      setAdjError(err instanceof Error ? err.message : "Falha ao remover ajuste.");
    }
  };

  const adjustmentList = adjustments.data ?? [];

  return (
    <PageShell>
      <div className="flex flex-wrap items-center gap-3">
        <h1 className="text-3xl">Ranking</h1>
        {hasLive && (
          <span className="inline-flex items-center gap-1.5 rounded-pill bg-danger-bg px-3 py-1 text-xs font-semibold text-danger ring-1 ring-danger/40">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-danger opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-danger" />
            </span>
            Parcial ao vivo
          </span>
        )}
      </div>
      {hasLive && (
        <p className="mt-2 max-w-3xl text-sm font-semibold text-danger">
          Há {liveMatches.length === 1 ? "1 jogo" : `${liveMatches.length} jogos`} em andamento — a
          pontuação abaixo é provisória e muda conforme o placar ao vivo. Ela só se firma quando os
          jogos terminam.
        </p>
      )}
      <Button variant="outline" size="sm" className="mt-3" onClick={() => setScoringOpen(true)}>
        <Info className="h-4 w-4" /> Como funciona a pontuação
      </Button>

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
          <h3 className="text-lg">Ainda não há ranking por aqui.</h3>
          <p className="mt-1 text-ink-muted">
            Crie um bolão ou entre com um código e deixe a disputa começar.
          </p>
        </Card>
      ) : (
        <>
          <Card className="mt-6 max-w-sm">
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
          </Card>

          <div className="mt-6">
            {leaderboard.isLoading ? (
              <Card>
                <p className="text-ink-muted">Carregando...</p>
              </Card>
            ) : leaderboard.isError ? (
              <ErrorBanner>
                Erro ao carregar ranking: {(leaderboard.error as Error).message}
              </ErrorBanner>
            ) : entries.length === 0 ? (
              <Card>
                <h3 className="text-lg">Ainda ninguém balançou esse ranking</h3>
                <p className="mt-1 text-ink-muted">
                  Quando os resultados oficiais entrarem, a tabela ganha vida por aqui.
                </p>
              </Card>
            ) : (
              <>
                <div className="grid grid-cols-3 gap-3">
                  {podium.map((entry, i) => (
                    <motion.div
                      key={entry.userId}
                      initial={{ opacity: 0, scale: 0.92 }}
                      animate={{ opacity: 1, scale: 1 }}
                      transition={{ delay: i * 0.1, duration: 0.32, ease: [0.22, 1, 0.36, 1] }}
                      className={`flex flex-col items-center rounded-lg bg-card p-4 shadow-card ${
                        i === 0 ? "ring-2 ring-yellow-dark/50" : ""
                      }`}
                    >
                      <span className="text-3xl">{medals[i]}</span>
                      <div className="mt-1 font-heading font-semibold">{entry.username}</div>
                      <div className="text-sm text-mint-dark">{entry.points} pts</div>
                      <div className="text-xs text-ink-muted">
                        🎯 {entry.exactScores} {entry.exactScores === 1 ? "exato" : "exatos"}
                      </div>
                    </motion.div>
                  ))}
                </div>

                {rest.length > 0 && (
                  <Card className="mt-5 overflow-hidden p-0">
                    <table className="w-full text-left">
                      <thead className="bg-mint/20 text-sm">
                        <tr>
                          <th className="px-5 py-3">Posição</th>
                          <th className="px-5 py-3">Usuário</th>
                          <th className="px-5 py-3">Pontos</th>
                          <th
                            className="whitespace-nowrap px-3 py-3"
                            title="Placares exatos (1º critério de desempate)"
                          >
                            🎯 Exatos
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {rest.map((entry, i) => (
                          <tr key={entry.userId} className="border-t border-mint/20">
                            <td className="px-5 py-3">{i + 4}</td>
                            <td className="px-5 py-3">{entry.username}</td>
                            <td className="px-5 py-3">{entry.points}</td>
                            <td className="whitespace-nowrap px-3 py-3 text-ink-muted">{entry.exactScores}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </Card>
                )}
              </>
            )}
          </div>

          {/* Painel do organizador: lançar ajustes manuais */}
          {isOrganizer && entries.length > 0 && (
            <Card className="mt-6 border-l-4 border-yellow-dark">
              <h2 className="text-xl">Ajustar pontos</h2>
              <p className="mt-1 text-sm text-ink-muted">
                Lance pontos manualmente para corrigir erros: escolha adicionar ou descontar e a
                quantidade. O ajuste e o motivo ficam visíveis para todos os participantes.
              </p>
              <form onSubmit={onAdjust} className="mt-3 grid gap-3 sm:grid-cols-[1fr_auto_auto_2fr_auto] sm:items-end">
                <div>
                  <Label htmlFor="adj-user">Membro</Label>
                  <Select id="adj-user" value={adjUser} onChange={(e) => setAdjUser(e.target.value)}>
                    <option value="">Selecione</option>
                    {entries.map((entry) => (
                      <option key={entry.userId} value={entry.userId}>
                        {entry.username}
                      </option>
                    ))}
                  </Select>
                </div>
                <div>
                  <Label>Operação</Label>
                  <div className="flex gap-1 rounded-pill bg-secondary/40 p-1" role="group" aria-label="Tipo de ajuste">
                    <Button
                      type="button"
                      size="sm"
                      variant={adjMode === "add" ? "primary" : "outline"}
                      className={adjMode === "add" ? "" : "border-transparent bg-transparent"}
                      aria-pressed={adjMode === "add"}
                      onClick={() => setAdjMode("add")}
                    >
                      + Adicionar
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant={adjMode === "subtract" ? "primary" : "outline"}
                      className={adjMode === "subtract" ? "" : "border-transparent bg-transparent"}
                      aria-pressed={adjMode === "subtract"}
                      onClick={() => setAdjMode("subtract")}
                    >
                      − Descontar
                    </Button>
                  </div>
                </div>
                <div>
                  <Label htmlFor="adj-points">Pontos</Label>
                  <Input
                    id="adj-points"
                    type="number"
                    inputMode="numeric"
                    min={1}
                    max={1000}
                    placeholder="3"
                    value={adjPoints}
                    onChange={(e) => setAdjPoints(e.target.value)}
                    className="w-24"
                  />
                </div>
                <div>
                  <Label htmlFor="adj-reason">Motivo (opcional)</Label>
                  <Input
                    id="adj-reason"
                    placeholder="Ex.: erro de cadastro de placar"
                    value={adjReason}
                    maxLength={200}
                    onChange={(e) => setAdjReason(e.target.value)}
                  />
                </div>
                <Button type="submit" disabled={addAdjustment.isPending} className="self-start sm:self-auto">
                  {addAdjustment.isPending
                    ? "Lançando..."
                    : adjMode === "subtract"
                      ? "Descontar"
                      : "Adicionar"}
                </Button>
              </form>
              {adjError && (
                <div className="mt-3">
                  <ErrorBanner>{adjError}</ErrorBanner>
                </div>
              )}
            </Card>
          )}

          {/* Transparência: ajustes visíveis a todos */}
          {adjustmentList.length > 0 && (
            <Card className="mt-6">
              <h2 className="text-xl">Ajustes de pontos</h2>
              <ul className="mt-3 divide-y divide-mint/20">
                {adjustmentList.map((adj) => (
                  <li key={adj.id} className="flex items-center justify-between gap-3 py-3">
                    <div className="min-w-0">
                      <div className="font-heading font-semibold text-ink">
                        {adj.username}{" "}
                        <span className={adj.delta >= 0 ? "text-mint-dark" : "text-danger"}>
                          {adj.delta >= 0 ? `+${adj.delta}` : adj.delta} pts
                        </span>
                      </div>
                      {adj.reason && (
                        <div className="truncate text-sm text-ink-muted">{adj.reason}</div>
                      )}
                    </div>
                    {isOrganizer && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => onRemoveAdjustment(adj.id)}
                        disabled={removeAdjustment.isPending}
                        className="shrink-0"
                      >
                        <X className="h-4 w-4" /> Remover
                      </Button>
                    )}
                  </li>
                ))}
              </ul>
            </Card>
          )}
        </>
      )}
      <AnimatePresence>
        {scoringOpen && (
          <motion.div
            key="scoring-backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18 }}
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
            onClick={() => setScoringOpen(false)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 8 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 8 }}
              transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
              className="relative w-full max-w-lg rounded-2xl bg-card p-6 shadow-card-hover"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-start justify-between gap-4">
                <h2 className="text-xl font-heading font-semibold">Como funciona a pontuação</h2>
                <button
                  onClick={() => setScoringOpen(false)}
                  className="mt-0.5 shrink-0 rounded-full p-1 text-ink-muted transition-colors hover:bg-secondary hover:text-ink"
                  aria-label="Fechar"
                >
                  <X className="h-5 w-5" />
                </button>
              </div>

              <p className="mt-1 text-sm text-ink-muted">Aplica-se a todos os jogos (grupos e mata-mata).</p>

              <table className="mt-4 w-full text-sm">
                <thead>
                  <tr className="border-b border-mint/30 text-left text-xs font-semibold uppercase tracking-wide text-ink-muted">
                    <th className="pb-2 pr-4">Situação</th>
                    <th className="pb-2 text-right">Pontos</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-mint/15">
                  {[
                    ["Placar exato", "7"],
                    ["Resultado certo + gols de um time acertados", "4"],
                    ["Só o resultado certo", "3"],
                    ["Resultado errado", "0"],
                  ].map(([label, pts]) => (
                    <tr key={label}>
                      <td className="py-2 pr-4 text-ink">{label}</td>
                      <td className="py-2 text-right font-semibold text-mint-dark">{pts}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <p className="mt-5 text-sm font-semibold text-ink">Bônus mata-mata — pênaltis</p>
              <p className="text-xs text-ink-muted">Somado à base quando o jogo vai para a disputa de pênaltis.</p>

              <table className="mt-2 w-full text-sm">
                <thead>
                  <tr className="border-b border-mint/30 text-left text-xs font-semibold uppercase tracking-wide text-ink-muted">
                    <th className="pb-2 pr-4">Situação</th>
                    <th className="pb-2 text-right">Total</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-mint/15">
                  {[
                    ["Placar exato + pênaltis exatos (ex: 1×1, 5×4)", "10"],
                    ["Placar exato + classificado certo", "9"],
                    ["Placar exato + errou a disputa", "7"],
                    ["Empate não exato + classificado certo", "4"],
                    ["Empate não exato + errou a disputa", "3"],
                  ].map(([label, pts]) => (
                    <tr key={label}>
                      <td className="py-2 pr-4 text-ink">{label}</td>
                      <td className="py-2 text-right font-semibold text-mint-dark">{pts}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <div className="mt-5 rounded-xl bg-secondary/50 px-4 py-3 text-sm text-ink-muted">
                <span className="font-semibold text-ink">Desempate</span> — na ordem: (1) mais placares
                exatos; (2) mais acertos de resultado; (3) mais bônus de precisão. Ajustes manuais
                não entram no desempate.
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </PageShell>
  );
}
