import { api, useMutation, useQueryClient, type AdminMatchRecord, type KnockoutEntry, type MatchRecord } from "./shared";

type FlatAdminMatchRecord = MatchRecord & Omit<AdminMatchRecord, "matchRecord">;

export function normalizeAdminMatchRecord(
  input: AdminMatchRecord | FlatAdminMatchRecord | MatchRecord,
): AdminMatchRecord {
  if ("matchRecord" in input) return input;

  const matchRecord: MatchRecord = {
    id: input.id,
    homeTeam: input.homeTeam,
    awayTeam: input.awayTeam,
    kickoff: input.kickoff,
    groupName: input.groupName,
    phase: input.phase,
    homeScore: input.homeScore,
    awayScore: input.awayScore,
    qualifier: input.qualifier,
    wentToPenalties: input.wentToPenalties,
    penaltyHomeScore: input.penaltyHomeScore,
    penaltyAwayScore: input.penaltyAwayScore,
    finished: input.finished,
  };

  if ("adminStatus" in input) {
    return {
      matchRecord,
      adminStatus: input.adminStatus,
      lastAuditAt: input.lastAuditAt,
    };
  }

  return {
    matchRecord,
    adminStatus: input.finished ? "finalized" : "scheduled",
    lastAuditAt: null,
  };
}

// ---- Admin ----------------------------------------------------------------

export function useSetKnockoutReleased() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (released: boolean) =>
      api.post<void>("/admin/knockout-released", { released }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["knockout-released"] });
      qc.invalidateQueries({ queryKey: ["matches"] });
      // Mantém o rascunho de configurações em sincronia para que um "Salvar
      // configurações" posterior não reverta a liberação feita aqui.
      qc.invalidateQueries({ queryKey: ["admin-settings"] });
    },
  });
}

export function useSetMatchResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      homeScore: number;
      awayScore: number;
      knockout: KnockoutEntry;
    }) =>
      api.post<MatchRecord>(`/admin/matches/${vars.matchId}/result`, {
        homeScore: vars.homeScore,
        awayScore: vars.awayScore,
        knockout: vars.knockout,
      }),
    onSuccess: () => {
      // Placar conta no ranking na hora → invalida partidas e leaderboard.
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
    },
  });
}

export function useSetMatchFinished() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { matchId: string; finished: boolean }) =>
      api.post<void>(`/admin/matches/${vars.matchId}/finished`, {
        finished: vars.finished,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

export function useUpdateMatchTeams() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      homeTeam: string;
      awayTeam: string;
    }) =>
      api.post<void>(`/admin/matches/${vars.matchId}/teams`, {
        homeTeam: vars.homeTeam,
        awayTeam: vars.awayTeam,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

// Cadastro manual de jogos de mata-mata pelo admin (times + fase + horário).
export function useCreateMatch() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      homeTeam: string;
      awayTeam: string;
      phase: string;
      kickoff: string;
    }) => api.post<MatchRecord>("/admin/matches", vars),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useUpdateMatchSchedule() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      homeTeam: string;
      awayTeam: string;
      phase: string;
      kickoff: string;
    }) =>
      api.post<MatchRecord>(`/admin/matches/${vars.matchId}/schedule`, {
        homeTeam: vars.homeTeam,
        awayTeam: vars.awayTeam,
        phase: vars.phase,
        kickoff: vars.kickoff,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

export function useDeleteMatch() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (matchId: string) =>
      api.post<void>(`/admin/matches/${matchId}/delete`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
    },
  });
}

