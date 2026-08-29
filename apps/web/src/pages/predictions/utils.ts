import { formatSelectionLabel } from "@/lib/selections";
import type { MatchRecord } from "@/types";

const PHASE_ORDER = ["Fase de grupos", "Oitavas de final", "Quartas de final", "Semifinal", "Disputa de terceiro", "Final"];

export function phaseRank(phase: string): number {
  const index = PHASE_ORDER.indexOf(phase);
  return index === -1 ? PHASE_ORDER.length : index;
}

function normalize(text: string): string {
  return text.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase().trim();
}

export function matchesSearch(game: MatchRecord, query: string): boolean {
  if (!query) return true;
  const haystack = normalize(`${game.homeTeam} ${game.awayTeam} ${formatSelectionLabel(game.homeTeam)} ${formatSelectionLabel(game.awayTeam)}`);
  return haystack.includes(normalize(query));
}
