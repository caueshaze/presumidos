import { motion } from "framer-motion";
import type { LeaderboardEntry, PoolSummary } from "@/types";
import { Card } from "@/components/ui/card";
import { Label, Select, ErrorBanner } from "@/components/ui/field";

const medals = ["🥇", "🥈", "🥉"];
interface Props {
  pools: PoolSummary[];
  selectedPool: string;
  onSelectPool: (poolId: string) => void;
  entries: LeaderboardEntry[];
  isFootball: boolean;
  isLoading: boolean;
  error: string | null;
  onOpenMember: (userId: string) => void;
}

export function LeaderboardRanking({
  pools,
  selectedPool,
  onSelectPool,
  entries,
  isFootball,
  isLoading,
  error,
  onOpenMember,
}: Props) {
  const podium = entries.slice(0, 3);
  const rest = entries.slice(3);
  return (
    <>
      <Card className="mt-6 max-w-sm">
        <Label htmlFor="pool-select">Bolão</Label>
        <Select
          id="pool-select"
          value={selectedPool}
          onChange={(event) => onSelectPool(event.target.value)}
        >
          {pools.map((pool) => (
            <option key={pool.id} value={pool.id}>
              {pool.name}
            </option>
          ))}
        </Select>
      </Card>
      <div className="mt-6">
        {isLoading ? (
          <Card>
            <p className="text-ink-muted">Carregando...</p>
          </Card>
        ) : error ? (
          <ErrorBanner>Erro ao carregar ranking: {error}</ErrorBanner>
        ) : entries.length === 0 ? (
          <Card>
            <h3 className="text-lg">Ainda ninguém balançou esse ranking</h3>
            <p className="mt-1 text-ink-muted">
              Quando os resultados oficiais entrarem, a tabela ganha vida por
              aqui.
            </p>
          </Card>
        ) : (
          <>
            <div className="grid grid-cols-3 gap-3">
              {podium.map((entry, index) => (
                <PodiumEntry
                  key={entry.userId}
                  entry={entry}
                  index={index}
                  isFootball={isFootball}
                  onOpenMember={onOpenMember}
                />
              ))}
            </div>
            {rest.length > 0 && (
              <RankingTable
                entries={rest}
                isFootball={isFootball}
                onOpenMember={onOpenMember}
              />
            )}
          </>
        )}
      </div>
    </>
  );
}
function PodiumEntry({
  entry,
  index,
  isFootball,
  onOpenMember,
}: {
  entry: LeaderboardEntry;
  index: number;
  isFootball: boolean;
  onOpenMember: (userId: string) => void;
}) {
  const hits = isFootball ? entry.exactScores : entry.correctResults;
  return (
    <motion.button
      type="button"
      onClick={() => onOpenMember(entry.userId)}
      initial={{ opacity: 0, scale: 0.92 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{
        delay: index * 0.1,
        duration: 0.32,
        ease: [0.22, 1, 0.36, 1],
      }}
      className={`flex flex-col items-center rounded-lg bg-card p-4 text-center shadow-card transition-shadow hover:shadow-card-hover ${index === 0 ? "ring-2 ring-yellow-dark/50" : ""}`}
    >
      <span className="text-3xl">{medals[index]}</span>
      <div className="mt-1 font-heading font-semibold">{entry.username}</div>
      <div className="text-sm text-mint-dark">{entry.points} pts</div>
      <div className="text-xs text-ink-muted">
        {isFootball ? "🎯" : "✓"} {hits}{" "}
        {hits === 1
          ? isFootball
            ? "exato"
            : "acerto"
          : isFootball
            ? "exatos"
            : "acertos"}
      </div>
    </motion.button>
  );
}
function RankingTable({
  entries,
  isFootball,
  onOpenMember,
}: {
  entries: LeaderboardEntry[];
  isFootball: boolean;
  onOpenMember: (userId: string) => void;
}) {
  return (
    <Card className="mt-5 overflow-hidden p-0">
      <table className="w-full text-left">
        <thead className="bg-mint/20 text-sm">
          <tr>
            <th className="px-5 py-3">Posição</th>
            <th className="px-5 py-3">Usuário</th>
            <th className="px-5 py-3">Pontos</th>
            <th
              className="whitespace-nowrap px-3 py-3"
              title={
                isFootball
                  ? "Placares exatos (1º critério de desempate)"
                  : undefined
              }
            >
              {isFootball ? "🎯 Exatos" : "Acertos"}
            </th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry, index) => (
            <tr key={entry.userId} className="border-t border-mint/20">
              <td className="px-5 py-3">{index + 4}</td>
              <td className="px-5 py-3">
                <button
                  type="button"
                  onClick={() => onOpenMember(entry.userId)}
                  className="font-semibold text-ink underline-offset-4 hover:text-mint-dark hover:underline"
                >
                  {entry.username}
                </button>
              </td>
              <td className="px-5 py-3">{entry.points}</td>
              <td className="whitespace-nowrap px-3 py-3 text-ink-muted">
                {isFootball ? entry.exactScores : entry.correctResults}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  );
}
