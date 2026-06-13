import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { usePools, useLeaderboard } from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Label, Select, ErrorBanner } from "@/components/ui/field";

const medals = ["🥇", "🥈", "🥉"];

export function LeaderboardPage() {
  const pools = usePools();
  const [selectedPool, setSelectedPool] = useState("");

  useEffect(() => {
    if (!selectedPool && pools.data && pools.data.length > 0) {
      setSelectedPool(pools.data[0].id);
    }
  }, [pools.data, selectedPool]);

  const leaderboard = useLeaderboard(selectedPool || null);

  const entries = leaderboard.data ?? [];
  const podium = entries.slice(0, 3);
  const rest = entries.slice(3);

  return (
    <PageShell>
      <h1 className="text-3xl">Ranking</h1>
      <p className="mt-2 max-w-3xl text-sm text-ink-muted">
        A pontuação considera o placar do tempo normal. Placar exato vale 7 pontos; resultado
        correto vale 3; acertar os gols de um time que marcou pelo menos 1 gol dá +1. No mata-mata,
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
                      key={entry.username}
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
                          <tr key={entry.username} className="border-t border-mint/20">
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
        </>
      )}
    </PageShell>
  );
}
