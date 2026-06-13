import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** "2026-06-12T18:00:00Z" -> "12/06 18:00" (horário local). */
export function formatKickoff(kickoff: string): string {
  const date = new Date(kickoff);
  if (Number.isNaN(date.getTime())) return kickoff;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(date.getDate())}/${pad(date.getMonth() + 1)} ${pad(date.getHours())}:${pad(
    date.getMinutes(),
  )}`;
}

export function isMatchLocked(kickoff: string): boolean {
  const date = new Date(kickoff);
  if (Number.isNaN(date.getTime())) return false;
  return date.getTime() <= Date.now();
}

/** "Ao vivo" = já começou (kickoff passou) e ainda não foi marcado como finalizado. */
export function isMatchLive(kickoff: string, finished: boolean): boolean {
  return !finished && isMatchLocked(kickoff);
}

/** Lado vencedor do tempo normal ("home"/"away") ou null em empate. */
export function winnerSide(home: number, away: number): "home" | "away" | null {
  if (home > away) return "home";
  if (home < away) return "away";
  return null;
}

/** Mata-mata = qualquer fase que não seja a de grupos. */
export function isKnockout(phase: string | null): boolean {
  if (!phase) return false;
  const p = phase.trim().toLowerCase();
  return !(p.startsWith("fase de grupos") || p === "group" || p === "group stage");
}
