import { AnimatePresence, motion } from "framer-motion";
import { SmilePlus } from "lucide-react";
import { formatSelectionLabel } from "@/lib/selections";
import { formatKickoff } from "@/lib/utils";
import type { MatchRecord, PoolPredictionRecord, PredictionScoreBreakdown } from "@/types";

const REACTION_EMOJIS = ["🔥", "👏", "😂", "😮", "😅", "😭"] as const;

export function initials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase();
}

function ReactionBar({
  poolId,
  targetUserId,
  prediction,
  disabled,
  isPending,
  isPickerOpen,
  onTogglePicker,
  onReact,
}: {
  poolId: string;
  targetUserId: string;
  prediction: PoolPredictionRecord;
  disabled: boolean;
  isPending: boolean;
  isPickerOpen: boolean;
  onTogglePicker: (matchId: string) => void;
  onReact: (vars: {
    poolId: string;
    targetUserId: string;
    matchId: string;
    emoji: string;
  }) => void;
}) {
  return (
    <div className="mt-3 flex flex-wrap items-center gap-2">
      {prediction.reactions.map((reaction) => (
        <motion.span
          key={reaction.emoji}
          layout
          initial={{ opacity: 0, y: 4, scale: 0.92 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
          className={
            reaction.reactedByViewer
              ? "rounded-pill bg-mint/25 px-2.5 py-1 text-xs font-semibold text-mint-dark ring-1 ring-mint/40"
              : "rounded-pill bg-card px-2.5 py-1 text-xs font-semibold text-ink-muted ring-1 ring-mint/20"
          }
        >
          {reaction.emoji} {reaction.count}
        </motion.span>
      ))}

      {!disabled && (
        <div className="relative">
          <motion.button
            type="button"
            disabled={isPending}
            onClick={() => onTogglePicker(prediction.matchId)}
            whileTap={{ scale: 0.94 }}
            animate={{
              scale: isPickerOpen ? 1.04 : 1,
              rotate: isPickerOpen ? 6 : 0,
            }}
            transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
            className={
              prediction.viewerReaction
                ? "inline-flex h-8 w-8 items-center justify-center rounded-full bg-mint/20 text-mint-dark ring-1 ring-mint/35 transition hover:bg-mint/25"
                : "inline-flex h-8 w-8 items-center justify-center rounded-full bg-card text-ink-muted ring-1 ring-mint/20 transition hover:bg-mint/10"
            }
            aria-expanded={isPickerOpen}
            aria-label={
              prediction.viewerReaction
                ? `Trocar reacao ${prediction.viewerReaction}`
                : "Abrir menu de reacoes"
            }
          >
            <SmilePlus className="h-4 w-4" />
          </motion.button>

          <AnimatePresence>
            {isPickerOpen && (
              <motion.div
                initial={{ opacity: 0, y: -6, scale: 0.96 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, y: -4, scale: 0.98 }}
                transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
                className="absolute left-0 top-full z-10 mt-2 flex min-w-max flex-wrap gap-1.5 rounded-2xl border border-mint/20 bg-card p-2 shadow-card"
              >
                {REACTION_EMOJIS.map((emoji, index) => {
                  const active = prediction.viewerReaction === emoji;
                  return (
                    <motion.button
                      key={emoji}
                      type="button"
                      disabled={isPending}
                      initial={{ opacity: 0, y: 6, scale: 0.92 }}
                      animate={{ opacity: 1, y: 0, scale: 1 }}
                      exit={{ opacity: 0, y: 4, scale: 0.92 }}
                      transition={{
                        delay: index * 0.02,
                        duration: 0.16,
                        ease: [0.22, 1, 0.36, 1],
                      }}
                      whileHover={{ y: -1, scale: 1.04 }}
                      whileTap={{ scale: 0.92 }}
                      onClick={() => {
                        onReact({
                          poolId,
                          targetUserId,
                          matchId: prediction.matchId,
                          emoji,
                        });
                        onTogglePicker(prediction.matchId);
                      }}
                      className={
                        active
                          ? "rounded-full bg-mint/30 px-2.5 py-1 text-sm ring-1 ring-mint/50"
                          : "rounded-full bg-card px-2.5 py-1 text-sm ring-1 ring-mint/20 transition hover:bg-mint/10"
                      }
                      aria-label={`Reagir com ${emoji}`}
                    >
                      {emoji}
                    </motion.button>
                  );
                })}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
}

export function PredictionDetail({
  poolId,
  targetUserId,
  prediction,
  game,
  breakdown,
  highlight,
  canReact,
  reactPending,
  isPickerOpen,
  onTogglePicker,
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
  isPickerOpen: boolean;
  onTogglePicker: (matchId: string) => void;
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
        isPickerOpen={isPickerOpen}
        onTogglePicker={onTogglePicker}
        onReact={onReact}
      />
    </div>
  );
}

