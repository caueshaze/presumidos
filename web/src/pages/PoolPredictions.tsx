import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { motion } from "framer-motion";
import { ChevronLeft } from "lucide-react";
import {
  usePools,
  usePoolMemberPredictions,
  usePoolBreakdowns,
  useMatches,
} from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Label, Select, ErrorBanner } from "@/components/ui/field";
import { formatSelectionLabel } from "@/lib/selections";
import { formatKickoff } from "@/lib/utils";
import type {
  MatchRecord,
  MemberPredictions,
  PredictionRecord,
  PredictionScoreBreakdown,
} from "@/types";

function initials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase();
}

/** Linha de placar palpitado, com detalhes de mata-mata quando houver. */
function PredictionDetail({
  prediction,
  game,
  breakdown,
}: {
  prediction: PredictionRecord;
  game: MatchRecord | undefined;
  breakdown: PredictionScoreBreakdown | undefined;
}) {
  if (!game) return null;

  const qualifierName =
    prediction.qualifier === "home"
      ? game.homeTeam
      : prediction.qualifier === "away"
        ? game.awayTeam
        : null;

  const hasOfficial = game.homeScore !== null && game.awayScore !== null;
  // Pontuação que conta neste bolão: 0 quando inelegível (entrou após o kickoff).
  const earned = breakdown && breakdown.eligible ? breakdown.totalPoints : 0;

  return (
    <div className="flex flex-col gap-1 border-t border-mint/20 py-3 first:border-t-0">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0 text-sm text-ink">
          <span className="truncate">{formatSelectionLabel(game.homeTeam)}</span>
          <span className="mx-2 font-heading font-semibold text-ink">
            {prediction.homeScore} <span className="text-ink-muted">x</span> {prediction.awayScore}
          </span>
          <span className="truncate">{formatSelectionLabel(game.awayTeam)}</span>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {hasOfficial && breakdown && (
            <span
              className={
                earned > 0
                  ? "rounded-pill bg-success/15 px-2.5 py-0.5 text-xs font-semibold text-mint-dark ring-1 ring-success/35"
                  : "rounded-pill bg-card px-2.5 py-0.5 text-xs font-semibold text-ink-muted ring-1 ring-mint/25"
              }
              title={
                breakdown.eligible
                  ? "Pontos que este palpite somou no bolão"
                  : "Não conta: entrou no bolão após o jogo começar"
              }
            >
              {earned > 0 ? `+${earned} pts` : breakdown.eligible ? "0 pts" : "não conta"}
            </span>
          )}
          <span className="text-xs text-ink-muted">{formatKickoff(game.kickoff)}</span>
        </div>
      </div>

      {hasOfficial && (
        <div className="text-xs text-ink-muted">
          Resultado oficial:{" "}
          <span className="font-semibold text-ink">
            {game.homeScore} x {game.awayScore}
          </span>
        </div>
      )}

      {qualifierName && (
        <div className="text-xs text-mint-dark">
          Classifica: {formatSelectionLabel(qualifierName)}
          {prediction.wentToPenalties && (
            <>
              {" "}· nos pênaltis
              {prediction.penaltyHomeScore != null && prediction.penaltyAwayScore != null && (
                <>
                  {" "}
                  ({prediction.penaltyHomeScore}-{prediction.penaltyAwayScore})
                </>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}

export function PoolPredictionsPage() {
  const pools = usePools();
  const [searchParams] = useSearchParams();
  const poolIdParam = searchParams.get("poolId");
  const [selectedPool, setSelectedPool] = useState("");
  const [selectedMemberId, setSelectedMemberId] = useState<string | null>(null);

  // Seleciona o bolão indicado na URL (?poolId=) quando existir; senão, o primeiro.
  useEffect(() => {
    if (selectedPool || !pools.data || pools.data.length === 0) return;
    const wanted =
      poolIdParam && pools.data.some((p) => p.id === poolIdParam)
        ? poolIdParam
        : pools.data[0].id;
    setSelectedPool(wanted);
  }, [pools.data, selectedPool, poolIdParam]);

  // Trocar de bolão fecha o perfil aberto.
  useEffect(() => {
    setSelectedMemberId(null);
  }, [selectedPool]);

  const members = usePoolMemberPredictions(selectedPool || null);
  const matches = useMatches();
  const breakdowns = usePoolBreakdowns(selectedPool || null);

  const matchById = useMemo(() => {
    const map = new Map<string, MatchRecord>();
    for (const m of matches.data ?? []) map.set(m.id, m);
    return map;
  }, [matches.data]);

  // Chave userId:matchId → breakdown, para achar os pontos de cada palpite.
  const breakdownByKey = useMemo(() => {
    const map = new Map<string, PredictionScoreBreakdown>();
    for (const b of breakdowns.data ?? []) map.set(`${b.userId}:${b.matchId}`, b);
    return map;
  }, [breakdowns.data]);

  const entries: MemberPredictions[] = members.data ?? [];
  const selectedMember = entries.find((m) => m.userId === selectedMemberId) ?? null;

  return (
    <PageShell>
      <h1 className="text-3xl">Palpites do Bolão</h1>
      <p className="mt-2 max-w-3xl text-sm text-ink-muted">
        Veja os palpites dos outros participantes. Por justiça, o palpite de cada partida só aparece
        depois que o jogo começa.
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
            {members.isLoading || matches.isLoading ? (
              <Card>
                <p className="text-ink-muted">Carregando...</p>
              </Card>
            ) : members.isError ? (
              <ErrorBanner>
                Erro ao carregar palpites: {(members.error as Error).message}
              </ErrorBanner>
            ) : selectedMember ? (
              // ---- Perfil do membro ----
              <div>
                <button
                  type="button"
                  onClick={() => setSelectedMemberId(null)}
                  className="mb-4 inline-flex items-center gap-1 text-sm font-semibold text-ink-muted transition-colors hover:text-ink"
                >
                  <ChevronLeft className="h-4 w-4" /> Voltar
                </button>

                <Card>
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

                  <div className="mt-4">
                    {selectedMember.predictions.length === 0 ? (
                      <p className="text-sm text-ink-muted">
                        Os palpites aparecem aqui assim que os jogos começam.
                      </p>
                    ) : (
                      selectedMember.predictions.map((p) => (
                        <PredictionDetail
                          key={p.matchId}
                          prediction={p}
                          game={matchById.get(p.matchId)}
                          breakdown={breakdownByKey.get(`${selectedMember.userId}:${p.matchId}`)}
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
              // ---- Lista de membros ----
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
                    <div className="min-w-0">
                      <div className="truncate font-heading font-semibold text-ink">
                        {member.username}
                      </div>
                      <div className="text-xs text-ink-muted">
                        {member.predictions.length}{" "}
                        {member.predictions.length === 1 ? "palpite" : "palpites"}
                      </div>
                    </div>
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
