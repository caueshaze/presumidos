import { useEffect, useState, type FormEvent } from "react";
import { motion } from "framer-motion";
import { X } from "lucide-react";
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

  // ---- Formulário de ajuste (organizador) ----
  const [adjUser, setAdjUser] = useState("");
  const [adjDelta, setAdjDelta] = useState("");
  const [adjReason, setAdjReason] = useState("");
  const [adjError, setAdjError] = useState("");

  useEffect(() => {
    setAdjUser("");
    setAdjDelta("");
    setAdjReason("");
    setAdjError("");
  }, [selectedPool]);

  const onAdjust = async (e: FormEvent) => {
    e.preventDefault();
    setAdjError("");
    const delta = parseInt(adjDelta, 10);
    if (!adjUser || Number.isNaN(delta) || delta === 0) {
      setAdjError("Escolha um membro e um valor de pontos diferente de zero.");
      return;
    }
    try {
      await addAdjustment.mutateAsync({
        poolId: selectedPool,
        userId: adjUser,
        delta,
        reason: adjReason.trim(),
      });
      setAdjUser("");
      setAdjDelta("");
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
      <p className="mt-2 max-w-3xl text-sm text-ink-muted">
        A pontuação considera o placar do tempo normal. Placar exato vale 7 pontos; resultado
        correto vale 3; acertar os gols de um time que marcou pelo menos 1 gol dá +1 se você acertou o ganhador. No mata-mata,
        acertar o classificado dá +2, e palpites corretos sobre pênaltis podem render bônus extras.
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
                        </tr>
                      </thead>
                      <tbody>
                        {rest.map((entry, i) => (
                          <tr key={entry.userId} className="border-t border-mint/20">
                            <td className="px-5 py-3">{i + 4}</td>
                            <td className="px-5 py-3">{entry.username}</td>
                            <td className="px-5 py-3">{entry.points}</td>
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
                Lance pontos manualmente para corrigir erros. Valores negativos descontam. O ajuste
                e o motivo ficam visíveis para todos os participantes.
              </p>
              <form onSubmit={onAdjust} className="mt-3 grid gap-3 sm:grid-cols-[1fr_auto_2fr_auto] sm:items-end">
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
                  <Label htmlFor="adj-delta">Pontos</Label>
                  <Input
                    id="adj-delta"
                    type="number"
                    inputMode="numeric"
                    placeholder="+3"
                    value={adjDelta}
                    onChange={(e) => setAdjDelta(e.target.value)}
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
                  {addAdjustment.isPending ? "Lançando..." : "Lançar"}
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
    </PageShell>
  );
}
