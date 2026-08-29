import { formatSelectionLabel } from "@/lib/selections";
import { cn } from "@/lib/utils";
import type { KnockoutEntry, MatchPointsSummary } from "@/types";
import { Input } from "./ui/input";
import { Label } from "./ui/field";

// Monta o detalhamento de pontos com os componentes que pontuaram (> 0).
export function pointsBreakdown(points: MatchPointsSummary): string {
  const parts: string[] = [];
  if (points.exactScorePoints > 0) parts.push(`Placar exato ${points.exactScorePoints}`);
  if (points.outcomePoints > 0) parts.push(`Resultado ${points.outcomePoints}`);
  if (points.goalBonusPoints > 0) parts.push(`Bônus de gols ${points.goalBonusPoints}`);
  if (points.qualifierPoints > 0) parts.push(`Classificado ${points.qualifierPoints}`);
  if (points.penaltiesPoints > 0) parts.push(`Pênaltis ${points.penaltiesPoints}`);
  return parts.join(" · ");
}

export function ScoreInputs({ children }: { children: React.ReactNode }) {
  return <div className="flex items-center gap-3">{children}</div>;
}

export function ScoreBox(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <Input
      type="text"
      inputMode="numeric"
      pattern="[0-9]*"
      autoComplete="off"
      className="score-input w-20 text-center text-xl font-heading font-bold"
      {...props}
    />
  );
}

export function PenaltyScorePanel({
  children,
  note,
  className,
}: {
  children: React.ReactNode;
  note?: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "rounded-md border border-mint/30 bg-card/80 p-3 shadow-[inset_0_1px_0_rgb(255_255_255/0.35)]",
        "dark:border-mint/20 dark:bg-bg/35 dark:shadow-none",
        className,
      )}
    >
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <Label className="mb-0">Placar dos pênaltis</Label>
        <ScoreInputs>{children}</ScoreInputs>
      </div>
      {note && <p className="mt-2 text-xs text-ink-muted">{note}</p>}
    </div>
  );
}

export function scoreToField(value: number | null | undefined): string {
  return value === null || value === undefined ? "" : String(value);
}

export function normalizeScoreField(raw: string): string {
  const digits = raw.replace(/\D+/g, "");
  if (!digits) return "";
  return digits.replace(/^0+(?=\d)/, "");
}

export function scoreValue(field: string): number {
  return field === "" ? 0 : parseInt(field, 10) || 0;
}

export function qualifierLabel(
  side: string | null | undefined,
  homeTeam: string,
  awayTeam: string,
): string | null {
  if (side === "home") return formatSelectionLabel(homeTeam);
  if (side === "away") return formatSelectionLabel(awayTeam);
  return null;
}

export function PredictionSummary({
  title,
  homeTeam,
  awayTeam,
  homeScore,
  awayScore,
  qualifier,
  wentToPenalties,
  penaltyHomeScore,
  penaltyAwayScore,
  tone = "default",
}: {
  title: string;
  homeTeam: string;
  awayTeam: string;
  homeScore: number;
  awayScore: number;
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHomeScore: number | null;
  penaltyAwayScore: number | null;
  tone?: "default" | "official";
}) {
  const qualifierName = qualifierLabel(qualifier, homeTeam, awayTeam);

  return (
    <div
      className={cn(
        "rounded-lg border px-4 py-3",
        tone === "official" ? "border-sky/30 bg-sky/10" : "border-mint/25 bg-mint/10",
      )}
    >
      <p className="text-xs font-semibold uppercase tracking-[0.18em] text-ink-muted">{title}</p>
      <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1">
        <span className="text-sm text-ink">{formatSelectionLabel(homeTeam)}</span>
        <span className="font-heading text-lg font-bold text-ink">
          {homeScore} <span className="text-ink-muted">x</span> {awayScore}
        </span>
        <span className="text-sm text-ink">{formatSelectionLabel(awayTeam)}</span>
      </div>
      {qualifierName && (
        <p className="mt-2 text-sm text-mint-dark">
          Classifica: {qualifierName}
          {wentToPenalties && (
            <>
              {" "}· nos pênaltis
              {penaltyHomeScore !== null && penaltyAwayScore !== null && (
                <>
                  {" "}
                  ({penaltyHomeScore}-{penaltyAwayScore})
                </>
              )}
            </>
          )}
        </p>
      )}
    </div>
  );
}

export function buildKnockoutEntry(
  knockout: boolean,
  home: number,
  away: number,
  penaltyHome: string,
  penaltyAway: string,
): KnockoutEntry {
  if (!knockout) return { qualifier: null, wentToPenalties: false, penaltyHome: null, penaltyAway: null };
  const wentToPenalties = home === away;
  return {
    qualifier: null,
    wentToPenalties,
    penaltyHome: wentToPenalties && penaltyHome !== "" ? scoreValue(penaltyHome) : null,
    penaltyAway: wentToPenalties && penaltyAway !== "" ? scoreValue(penaltyAway) : null,
  };
}

export function penaltiesError(
  knockout: boolean,
  home: number,
  away: number,
  penaltyHome: string,
  penaltyAway: string,
): string | null {
  if (!knockout || home !== away) return null;
  if (penaltyHome === "" || penaltyAway === "") return "Empate no tempo normal: informe o placar dos pênaltis dos dois lados.";
  if (scoreValue(penaltyHome) === scoreValue(penaltyAway)) return "O placar dos pênaltis não pode terminar empatado.";
  return null;
}
