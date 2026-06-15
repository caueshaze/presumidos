import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Radio, Timer } from "lucide-react";
import { formatSelectionLabel } from "@/lib/selections";
import { formatKickoff, formatLiveStatus, isMatchLive } from "@/lib/utils";
import type { MatchRecord, PredictionRecord } from "@/types";
import { Card } from "@/components/ui/card";

const section = {
  initial: { opacity: 0, y: 16 },
  animate: { opacity: 1, y: 0 },
};

/** "Ao vivo" = começou (kickoff passou) e ainda não foi marcado como finalizado.
 *  Mesma regra do badge "AO VIVO" do MatchCard, para consistência com Previsões. */
function isLive(m: MatchRecord): boolean {
  return isMatchLive(m.kickoff, m.finished);
}

/** Placar a exibir no card ao vivo: prioriza o parcial do poller; cai para o
 *  resultado oficial já lançado; por fim 0 x 0. */
function liveScore(m: MatchRecord): { home: number; away: number } {
  if (m.liveStatus) return { home: m.liveHomeScore ?? 0, away: m.liveAwayScore ?? 0 };
  if (m.homeScore !== null && m.awayScore !== null)
    return { home: m.homeScore, away: m.awayScore };
  return { home: 0, away: 0 };
}

/** "Começa em 2d 3h", "3h 12min", "12min 04s" — conforme a proximidade. */
function formatCountdown(msUntil: number): string {
  const total = Math.max(0, Math.floor(msUntil / 1000));
  const days = Math.floor(total / 86400);
  const hours = Math.floor((total % 86400) / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${pad(minutes)}min`;
  return `${minutes}min ${pad(seconds)}s`;
}

function LiveCard({
  game,
  prediction,
  onClick,
}: {
  game: MatchRecord;
  prediction: PredictionRecord | undefined;
  onClick: () => void;
}) {
  return (
    <Card
      className="cursor-pointer border border-danger/20 bg-white/70 hover:shadow-card-hover"
      onClick={onClick}
    >
      <div className="flex items-center justify-between gap-3">
        <span className="inline-flex items-center gap-1.5 rounded-pill bg-danger-bg px-3 py-1 text-xs font-semibold text-danger ring-1 ring-danger/40">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-danger opacity-75" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-danger" />
          </span>
          AO VIVO
        </span>
        <span className="rounded-pill bg-danger-bg px-2 py-0.5 text-xs font-semibold text-danger ring-1 ring-danger/40">
          {formatLiveStatus(game.liveStatus, game.liveElapsed)}
        </span>
      </div>

      <div className="mt-3 flex items-center justify-between gap-3">
        <span className="font-heading text-lg font-semibold">
          {formatSelectionLabel(game.homeTeam)}
        </span>
        <span className="font-heading text-2xl font-bold text-danger">
          {liveScore(game).home} <span className="text-ink-muted">x</span> {liveScore(game).away}
        </span>
        <span className="text-right font-heading text-lg font-semibold">
          {formatSelectionLabel(game.awayTeam)}
        </span>
      </div>

      {prediction && (
        <p className="mt-3 text-sm text-ink-muted">
          Seu palpite:{" "}
          <span className="font-semibold text-ink">
            {prediction.homeScore} x {prediction.awayScore}
          </span>
        </p>
      )}
    </Card>
  );
}

function NextGameCard({ game, onClick }: { game: MatchRecord; onClick: () => void }) {
  const [now, setNow] = useState(() => Date.now());
  const kickoffMs = new Date(game.kickoff).getTime();
  const msUntil = kickoffMs - now;
  const farAway = msUntil > 86_400_000; // mais de 1 dia

  useEffect(() => {
    if (farAway) return; // sem countdown ao vivo quando falta mais de um dia
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [farAway]);

  return (
    <Card
      className="cursor-pointer border border-sky/20 bg-white/70 hover:shadow-card-hover"
      onClick={onClick}
    >
      <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-sky-dark">
        <Timer className="h-4 w-4" />
        Próximo jogo
      </div>
      <div className="mt-2 flex items-center justify-between gap-3">
        <span className="font-heading text-lg font-semibold">
          {formatSelectionLabel(game.homeTeam)}
        </span>
        <span className="text-ink-muted">x</span>
        <span className="text-right font-heading text-lg font-semibold">
          {formatSelectionLabel(game.awayTeam)}
        </span>
      </div>
      <p className="mt-3 text-sm">
        {farAway ? (
          <span className="text-ink-muted">{formatKickoff(game.kickoff)}</span>
        ) : (
          <span>
            <span className="text-ink-muted">Começa em </span>
            <span className="font-heading text-xl font-bold text-sky-dark">
              {formatCountdown(msUntil)}
            </span>
          </span>
        )}
      </p>
    </Card>
  );
}

export function HomeLiveHighlight({
  matches,
  predictions,
}: {
  matches: MatchRecord[] | undefined;
  predictions: PredictionRecord[] | undefined;
}) {
  const navigate = useNavigate();
  const all = matches ?? [];
  const live = all.filter(isLive);

  const now = Date.now();
  const nextGame = all
    .filter((m) => !m.finished && new Date(m.kickoff).getTime() > now)
    .sort((a, b) => new Date(a.kickoff).getTime() - new Date(b.kickoff).getTime())[0];

  if (live.length === 0 && !nextGame) return null;

  const predictionByMatch = new Map((predictions ?? []).map((p) => [p.matchId, p]));

  return (
    <motion.section {...section} transition={{ duration: 0.3 }} className="mt-8">
      <h2 className="mb-3 flex items-center gap-2 text-xl">
        {live.length > 0 && <Radio className="h-5 w-5 text-danger" />}
        {live.length > 0 ? "Ao vivo agora" : "Está chegando"}
      </h2>
      {live.length > 0 ? (
        <div className="grid gap-4 sm:grid-cols-2">
          {live.map((game) => (
            <LiveCard
              key={game.id}
              game={game}
              prediction={predictionByMatch.get(game.id)}
              onClick={() => navigate(`/predictions?matchId=${game.id}`)}
            />
          ))}
        </div>
      ) : (
        nextGame && <NextGameCard game={nextGame} onClick={() => navigate("/predictions")} />
      )}
    </motion.section>
  );
}
