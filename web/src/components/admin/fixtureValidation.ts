import { formatKickoff } from "@/lib/utils";
import { formatSelectionLabel, getSelectionCatalogEntry } from "@/lib/selections";
import type { FixtureCheckResult } from "@/types";

// Fases de mata-mata disponíveis no cadastro manual de jogos.
export const KNOCKOUT_PHASES = [
  "16 avos de final",
  "Oitavas de final",
  "Quartas de final",
  "Semifinal",
  "Disputa de 3º lugar",
  "Final",
];

export type BrasiliaDateTimeInput = {
  date: string;
  time: string;
};

export type FixtureCheckState = {
  eventId: number;
  ok: boolean;
  message: string;
  fingerprint: string;
};

export function formatDateInput(value: string): string {
  const digits = value.replace(/\D/g, "").slice(0, 8);
  if (digits.length <= 2) return digits;
  if (digits.length <= 4) return `${digits.slice(0, 2)}/${digits.slice(2)}`;
  return `${digits.slice(0, 2)}/${digits.slice(2, 4)}/${digits.slice(4)}`;
}

export function formatTimeInput(value: string): string {
  const digits = value.replace(/\D/g, "").slice(0, 4);
  if (digits.length <= 2) return digits;
  return `${digits.slice(0, 2)}:${digits.slice(2)}`;
}

// Converte um ISO (UTC) para campos brasileiros, sempre em horario de Brasilia.
export function isoToBrasiliaInput(iso: string | null | undefined): BrasiliaDateTimeInput {
  if (!iso) return { date: "", time: "" };
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return { date: "", time: "" };
  const parts = new Intl.DateTimeFormat("pt-BR", {
    timeZone: "America/Sao_Paulo",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).formatToParts(date);
  const value = (type: Intl.DateTimeFormatPartTypes) =>
    parts.find((part) => part.type === type)?.value ?? "";
  return {
    date: `${value("day")}/${value("month")}/${value("year")}`,
    time: `${value("hour") === "24" ? "00" : value("hour")}:${value("minute")}`,
  };
}

// No admin, a data/hora digitada e Brasilia. Salvamos em UTC para o backend/poller.
export function brasiliaInputToIso(dateValue: string, timeValue: string): string | null {
  const dateMatch = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(dateValue.trim());
  const timeMatch = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(timeValue.trim());
  if (!dateMatch || !timeMatch) return null;

  const [, dayText, monthText, yearText] = dateMatch;
  const [, hourText, minuteText] = timeMatch;
  const day = Number(dayText);
  const month = Number(monthText);
  const year = Number(yearText);
  const hour = Number(hourText);
  const minute = Number(minuteText);

  const calendarDate = new Date(Date.UTC(year, month - 1, day));
  const validDate =
    calendarDate.getUTCFullYear() === year &&
    calendarDate.getUTCMonth() === month - 1 &&
    calendarDate.getUTCDate() === day;
  if (!validDate) return null;

  const utc = Date.UTC(year, month - 1, day, hour + 3, minute);
  return new Date(utc).toISOString();
}

export function brasiliaDateToIsoDateFilter(dateValue: string): string | null {
  const match = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(dateValue.trim());
  if (!match) return null;
  const [, dayText, monthText, yearText] = match;
  const day = Number(dayText);
  const month = Number(monthText);
  const year = Number(yearText);
  const calendarDate = new Date(Date.UTC(year, month - 1, day));
  const validDate =
    calendarDate.getUTCFullYear() === year &&
    calendarDate.getUTCMonth() === month - 1 &&
    calendarDate.getUTCDate() === day;
  return validDate ? `${yearText}-${monthText}-${dayText}` : null;
}

export function comparableSelectionName(value: string | null | undefined): string {
  if (!value) return "";
  const catalogEntry = getSelectionCatalogEntry(value);
  const canonical = catalogEntry?.name ?? value;
  return canonical
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/[^\p{Letter}\p{Number}]+/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function sameKickoffMinute(a: string | null | undefined, b: string | null | undefined): boolean {
  if (!a || !b) return false;
  const first = new Date(a).getTime();
  const second = new Date(b).getTime();
  if (Number.isNaN(first) || Number.isNaN(second)) return false;
  return Math.floor(first / 60000) === Math.floor(second / 60000);
}

export function fixtureFingerprint(eventId: number, homeTeam: string, awayTeam: string, kickoff: string): string {
  return [
    eventId,
    comparableSelectionName(homeTeam),
    comparableSelectionName(awayTeam),
    Math.floor(new Date(kickoff).getTime() / 60000),
  ].join("|");
}

export function validateFixtureAgainstMatch(
  checked: FixtureCheckResult,
  expected: { homeTeam: string; awayTeam: string; kickoff: string },
): { ok: boolean; message: string } {
  if (!checked.found) {
    return {
      ok: false,
      message: `ID ${checked.eventId}: o provedor respondeu, mas não trouxe detalhes do evento.`,
    };
  }
  if (!checked.homeTeam || !checked.awayTeam || !checked.kickoff) {
    return {
      ok: false,
      message: `ID ${checked.eventId}: faltam time mandante, visitante ou horário no provedor.`,
    };
  }

  const expectedHome = comparableSelectionName(expected.homeTeam);
  const expectedAway = comparableSelectionName(expected.awayTeam);
  const providerHome = comparableSelectionName(checked.homeTeam);
  const providerAway = comparableSelectionName(checked.awayTeam);
  const exactTeams = expectedHome === providerHome && expectedAway === providerAway;
  const swappedTeams = expectedHome === providerAway && expectedAway === providerHome;
  const kickoffMatches = sameKickoffMinute(expected.kickoff, checked.kickoff);

  if (!exactTeams || !kickoffMatches) {
    const issues: string[] = [];
    if (swappedTeams) {
      issues.push("mandante/visitante estão invertidos");
    } else if (!exactTeams) {
      issues.push(
        `confronto esperado ${formatSelectionLabel(expected.homeTeam)} x ${formatSelectionLabel(expected.awayTeam)}, mas o ID retornou ${checked.homeTeam} x ${checked.awayTeam}`,
      );
    }
    if (!kickoffMatches) {
      issues.push(`horário esperado ${formatKickoff(expected.kickoff)}, mas o ID retornou ${formatKickoff(checked.kickoff)}`);
    }
    return {
      ok: false,
      message: `ID ${checked.eventId} não confere: ${issues.join("; ")}.`,
    };
  }

  return {
    ok: true,
    message: `ID correto: ${checked.homeTeam} x ${checked.awayTeam} · ${formatKickoff(checked.kickoff)}`,
  };
}
