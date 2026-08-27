import { formatKickoff } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import type {
  AdminMatchRecord,
  AdminPredictionRow,
  AdminUserRecord,
  PoolSummary,
} from "@/types";
import type { Dispatch, SetStateAction } from "react";

type PredictionFilters = {
  matchId: string;
  userId: string;
  poolId: string;
  missingOnly: boolean;
};
type Setter<T> = Dispatch<SetStateAction<T>>;

function TextArea(props: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      {...props}
      className={`min-h-28 w-full rounded-md border-2 border-mint/40 bg-card px-4 py-2.5 text-ink focus:border-mint-dark focus:outline-none focus:shadow-glow ${props.className ?? ""}`}
    />
  );
}

type Props = {
  filters: PredictionFilters;
  setFilters: Setter<PredictionFilters>;
  matches: AdminMatchRecord[] | undefined;
  users: AdminUserRecord[] | undefined;
  pools: PoolSummary[] | undefined;
  predictions: AdminPredictionRow[] | undefined;
  selectedMatchRows: AdminPredictionRow[];
  overrideExpiry: string;
  setOverrideExpiry: Setter<string>;
  overrideReason: string;
  setOverrideReason: Setter<string>;
  onReopenPrediction: (userId: string, matchId: string) => void | Promise<unknown>;
  onRevokeReopen: (overrideId: string) => void | Promise<unknown>;
};

export function AdminPredictionsPanel({
  filters,
  setFilters,
  matches,
  users,
  pools,
  predictions,
  selectedMatchRows,
  overrideExpiry,
  setOverrideExpiry,
  overrideReason,
  setOverrideReason,
  onReopenPrediction,
  onRevokeReopen,
}: Props) {
  const updateFilters = (patch: Partial<PredictionFilters>) =>
    setFilters((current) => ({ ...current, ...patch }));

  return (

        <div className="mt-6 grid gap-5 xl:grid-cols-[1fr_1fr]">
          <Card>
            <div className="grid gap-3 md:grid-cols-4">
              <div>
                <Label>Jogo</Label>
                <Select value={filters.matchId} onChange={(e) => updateFilters({ matchId: e.target.value })}>
                  <option value="">Todos</option>
                  {matches?.map((item) => (
                    <option key={item.matchRecord.id} value={item.matchRecord.id}>
                      {item.matchRecord.homeTeam} x {item.matchRecord.awayTeam}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Usuário</Label>
                <Select value={filters.userId} onChange={(e) => updateFilters({ userId: e.target.value })}>
                  <option value="">Todos</option>
                  {users?.map((item) => (
                    <option key={item.user.id} value={item.user.id}>
                      {item.user.username}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Bolão</Label>
                <Select value={filters.poolId} onChange={(e) => updateFilters({ poolId: e.target.value })}>
                  <option value="">Todos</option>
                  {pools?.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </Select>
              </div>
              <label className="flex items-end gap-2 text-sm font-semibold text-ink">
                <input type="checkbox" checked={filters.missingOnly} onChange={(e) => updateFilters({ missingOnly: e.target.checked })} />
                Só sem palpite
              </label>
            </div>

            <div className="mt-5 space-y-3">
              {predictions?.slice(0, 80).map((row) => (
                <div key={`${row.poolId}-${row.userId}-${row.matchId}`} className="rounded-2xl border border-mint/15 bg-card/70 px-4 py-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <p className="font-semibold text-ink">
                        {row.username} · {row.poolName}
                      </p>
                      <p className="text-sm text-ink-muted">
                        {row.homeTeam} x {row.awayTeam} · {formatKickoff(row.kickoff)}
                      </p>
                    </div>
                    <div className="text-right text-sm">
                      <p className="font-semibold text-ink">
                        {row.prediction ? `${row.prediction.homeScore} x ${row.prediction.awayScore}` : "Sem palpite"}
                      </p>
                      <p className="text-ink-muted">
                        {row.locked ? "Travado" : "Aberto"} · {row.overrideInfo ? "reaberto" : "normal"}
                      </p>
                    </div>
                  </div>
                  {row.overrideInfo && (
                    <p className="mt-2 text-xs text-mint-dark">
                      Reaberto até {formatKickoff(row.overrideInfo.expiresAt)} · motivo: {row.overrideInfo.reason}
                    </p>
                  )}
                  <div className="mt-3 flex flex-wrap gap-2">
                    <Button size="sm" variant="outline" onClick={() => onReopenPrediction(row.userId, row.matchId)}>
                      Reabrir palpite
                    </Button>
                    {row.overrideInfo && (
                      <Button size="sm" variant="outline" onClick={() => onRevokeReopen(row.overrideInfo!.id)}>
                        Revogar reabertura
                      </Button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </Card>

          <Card>
            <h2 className="text-xl">Reabertura controlada</h2>
            <p className="mt-2 text-sm text-ink-muted">
              Use em caso de bug ou suporte. A auditoria registra quem abriu, para quem e até
              quando vale.
            </p>
            <div className="mt-4">
              <Label>Expira em</Label>
              <Input type="datetime-local" value={overrideExpiry} onChange={(e) => setOverrideExpiry(e.target.value)} />
            </div>
            <div className="mt-4">
              <Label>Motivo padrão</Label>
              <TextArea value={overrideReason} onChange={(e) => setOverrideReason(e.target.value)} placeholder="Ex.: falha de travamento indevido após o kickoff" />
            </div>
            <div className="mt-5 rounded-2xl border border-mint/15 bg-card/70 px-4 py-4">
              <p className="font-semibold text-ink">Quem ainda não palpitou no filtro atual</p>
              <div className="mt-3 space-y-2">
                {selectedMatchRows
                  .filter((row) => row.missing)
                  .slice(0, 12)
                  .map((row) => (
                    <div key={`${row.poolId}-${row.userId}-${row.matchId}`} className="flex items-center justify-between gap-3 rounded-xl border border-mint/10 bg-card px-3 py-3">
                      <div>
                        <p className="font-semibold text-ink">{row.username}</p>
                        <p className="text-xs text-ink-muted">{row.poolName}</p>
                      </div>
                      <Button size="sm" onClick={() => onReopenPrediction(row.userId, row.matchId)}>
                        Reabrir
                      </Button>
                    </div>
                  ))}
              </div>
            </div>
          </Card>
        </div>
  );
}
