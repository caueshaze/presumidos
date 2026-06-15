import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { motion } from "framer-motion";
import { ChevronLeft, SmilePlus } from "lucide-react";
import {
  useMarkPredictionReactionsSeen,
  useMatches,
  usePoolBreakdowns,
  usePoolMemberPredictions,
  usePools,
  useReactToPrediction,
} from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Label, Select, ErrorBanner } from "@/components/ui/field";
import { formatSelectionLabel } from "@/lib/selections";
import { formatKickoff } from "@/lib/utils";
import type {
  MatchRecord,
  MemberPredictions,
  PoolPredictionRecord,
  PredictionScoreBreakdown,
} from "@/types";

const REACTION_EMOJIS = ["🔥", "👏", "😂", "😮", "😅", "😭"] as const;

function initials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase();
}

function ReactionBar({
  poolId,
  targetUserId,
  prediction,
  disabled,
  isPending,
  onReact,
}: {
  poolId: string;
  targetUserId: string;
  prediction: PoolPredictionRecord;
  disabled: boolean;
  isPending: boolean;
  onReact: (vars: {
    poolId: string;
    targetUserId: string;
    matchId: string;
    emoji: string;
  }) => void;
}) {
  const [isPickerOpen, setIsPickerOpen] = useState(false);

  return (
    <div className="mt-3 flex flex-wrap items-center gap-2">
      {prediction.reactions.map((reaction) => (
        <span
          key={reaction.emoji}
          className={
            reaction.reactedByViewer
              ? "rounded-pill bg-mint/25 px-2.5 py-1 text-xs font-semibold text-mint-dark ring-1 ring-mint/40"
              : "rounded-pill bg-card px-2.5 py-1 text-xs font-semibold text-ink-muted ring-1 ring-mint/20"
          }
        >
          {reaction.emoji} {reaction.count}
        </span>
      ))}

      {!disabled && (
        <div className="relative">
          <button
            type="button"
            disabled={isPending}
            onClick={() => setIsPickerOpen((current) => !current)}
            className={
              prediction.viewerReaction
                ? "inline-flex h-8 min-w-8 items-center justify-center rounded-full bg-mint/20 px-2 text-base text-mint-dark ring-1 ring-mint/35 transition hover:bg-mint/25"
                : "inline-flex h-8 w-8 items-center justify-center rounded-full bg-white text-ink-muted ring-1 ring-mint/20 transition hover:bg-mint/10"
            }
            aria-expanded={isPickerOpen}
            aria-label={
              prediction.viewerReaction
                ? `Trocar reacao ${prediction.viewerReaction}`
                : "Abrir menu de reacoes"
            }
          >
            {prediction.viewerReaction ? (
              <span aria-hidden="true">{prediction.viewerReaction}</span>
            ) : (
              <SmilePlus className="h-4 w-4" />
            )}
          </button>

          {isPickerOpen && (
            <div className="absolute left-0 top-full z-10 mt-2 flex min-w-max flex-wrap gap-1.5 rounded-2xl border border-mint/20 bg-white p-2 shadow-card">
              {REACTION_EMOJIS.map((emoji) => {
                const active = prediction.viewerReaction === emoji;
                return (
                  <button
                    key={emoji}
                    type="button"
                    disabled={isPending}
                    onClick={() => {
                      onReact({
                        poolId,
                        targetUserId,
                        matchId: prediction.matchId,
                        emoji,
                      });
                      setIsPickerOpen(false);
                    }}
                    className={
                      active
                        ? "rounded-full bg-mint/30 px-2.5 py-1 text-sm ring-1 ring-mint/50"
                        : "rounded-full bg-card px-2.5 py-1 text-sm ring-1 ring-mint/20 transition hover:bg-mint/10"
                    }
                    aria-label={`Reagir com ${emoji}`}
                  >
                    {emoji}
                  </button>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function PredictionDetail({
  poolId,
  targetUserId,
  prediction,
  game,
  breakdown,
  highlight,
  canReact,
  reactPending,
  onReact,
}: {
  poolId: string;
  targetUserId: string;
  prediction: PoolPredictionRecord;
  game: MatchRecord | undefined;
  breakdown: PredictionScoreBreakdown | undefined;
  highlight: boolean;
  canReact: boolean;
  reactPending: boolean;
  onReact: (vars: {
    poolId: string;
    targetUserId: string;
    matchId: string;
    emoji: string;
  }) => void;
}) {
  if (!game) return null;

  const qualifierName =
    prediction.qualifier === "home"
      ? game.homeTeam
      : prediction.qualifier === "away"
        ? game.awayTeam
        : null;

  const hasOfficial = game.homeScore !== null && game.awayScore !== null;
  const earned = breakdown && breakdown.eligible ? breakdown.totalPoints : 0;

  return (
    <div
      className={
        highlight
          ? "flex flex-col gap-1 rounded-2xl border border-sky/45 bg-sky/10 px-3 py-3"
          : "flex flex-col gap-1 border-t border-mint/20 py-3 first:border-t-0"
      }
    >
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0 text-sm text-ink">
          <span className="truncate">{formatSelectionLabel(game.homeTeam)}</span>
          <span className="mx-2 font-heading font-semibold text-ink">
            {prediction.homeScore} <span className="text-ink-muted">x</span> {prediction.awayScore}
          </span>
          <span className="truncate">{formatSelectionLabel(game.awayTeam)}</span>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {prediction.unreadReactionCount > 0 && (
            <span className="rounded-pill bg-sky/20 px-2.5 py-0.5 text-xs font-semibold text-sky-dark ring-1 ring-sky/35">
              {prediction.unreadReactionCount} nova
              {prediction.unreadReactionCount > 1 ? "s" : ""} reacao
            </span>
          )}
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

      <ReactionBar
        poolId={poolId}
        targetUserId={targetUserId}
        prediction={prediction}
        disabled={!canReact}
        isPending={reactPending}
        onReact={onReact}
      />
    </div>
  );
}

export function PoolPredictionsPage() {
  const pools = usePools();
  const { user } = useAuth();
  const [searchParams] = useSearchParams();
  const poolIdParam = searchParams.get("poolId");
  const memberIdParam = searchParams.get("memberId");
  const matchIdParam = searchParams.get("matchId");
  const [selectedPool, setSelectedPool] = useState("");
  const [selectedMemberId, setSelectedMemberId] = useState<string | null>(null);
  const [lastSeenKey, setLastSeenKey] = useState("");

  useEffect(() => {
    if (selectedPool || !pools.data || pools.data.length === 0) return;
    const wanted =
      poolIdParam && pools.data.some((p) => p.id === poolIdParam) ? poolIdParam : pools.data[0].id;
    setSelectedPool(wanted);
  }, [pools.data, selectedPool, poolIdParam]);

  useEffect(() => {
    setSelectedMemberId(null);
    setLastSeenKey("");
  }, [selectedPool]);

  const members = usePoolMemberPredictions(selectedPool || null);
  const matches = useMatches();
  const breakdowns = usePoolBreakdowns(selectedPool || null);
  const reactToPrediction = useReactToPrediction();
  const markSeen = useMarkPredictionReactionsSeen();

  const matchById = useMemo(() => {
    const map = new Map<string, MatchRecord>();
    for (const m of matches.data ?? []) map.set(m.id, m);
    return map;
  }, [matches.data]);

  const breakdownByKey = useMemo(() => {
    const map = new Map<string, PredictionScoreBreakdown>();
    for (const b of breakdowns.data ?? []) map.set(`${b.userId}:${b.matchId}`, b);
    return map;
  }, [breakdowns.data]);

  const entries: MemberPredictions[] = members.data ?? [];

  useEffect(() => {
    if (selectedMemberId || !memberIdParam || entries.length === 0) return;
    if (entries.some((entry) => entry.userId === memberIdParam)) {
      setSelectedMemberId(memberIdParam);
    }
  }, [entries, memberIdParam, selectedMemberId]);

  const selectedMember = entries.find((m) => m.userId === selectedMemberId) ?? null;

  useEffect(() => {
    if (!selectedPool || !selectedMember || !user) return;
    if (selectedMember.userId !== user.id) return;
    if (selectedMember.unreadReactionCount <= 0) return;
    const seenKey = `${selectedPool}:${selectedMember.userId}:${selectedMember.unreadReactionCount}`;
    if (lastSeenKey === seenKey) return;
    if (markSeen.isPending) return;
    setLastSeenKey(seenKey);
    markSeen.mutate(selectedPool);
  }, [lastSeenKey, markSeen, selectedMember, selectedPool, user]);

  return (
    <PageShell>
      <h1 className="text-3xl">Palpites do Bolão</h1>
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
              <div>
                <button
                  type="button"
                  onClick={() => setSelectedMemberId(null)}
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
