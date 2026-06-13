import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AuthResult,
  KnockoutEntry,
  LeaderboardEntry,
  MatchRecord,
  PoolSummary,
  PredictionRecord,
} from "@/types";

// ---- Auth mutations -------------------------------------------------------

export function useLogin() {
  return useMutation({
    mutationFn: (vars: { username: string; password: string }) =>
      api.postPublic<AuthResult>("/auth/login", vars),
  });
}

export function useRegisterRequest() {
  return useMutation({
    mutationFn: (vars: { username: string; email: string; password: string }) =>
      api.postPublic<void>("/auth/register", vars),
  });
}

export function useRegisterConfirm() {
  return useMutation({
    mutationFn: (vars: { email: string; code: string }) =>
      api.postPublic<AuthResult>("/auth/register/confirm", vars),
  });
}

export function usePasswordResetRequest() {
  return useMutation({
    mutationFn: (vars: { email: string }) => api.postPublic<void>("/auth/password-reset", vars),
  });
}

export function usePasswordResetConfirm() {
  return useMutation({
    mutationFn: (vars: { email: string; code: string; newPassword: string }) =>
      api.postPublic<void>("/auth/password-reset/confirm", vars),
  });
}

// ---- Pools ----------------------------------------------------------------

export function usePools() {
  return useQuery({ queryKey: ["pools"], queryFn: () => api.get<PoolSummary[]>("/pools") });
}

export function useCreatePool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => api.post<PoolSummary>("/pools", { name }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["pools"] }),
  });
}

export function useJoinPool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (inviteCode: string) => api.post<PoolSummary>("/pools/join", { inviteCode }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["pools"] }),
  });
}

// ---- Matches / predictions ------------------------------------------------

export function useMatches() {
  return useQuery({ queryKey: ["matches"], queryFn: () => api.get<MatchRecord[]>("/matches") });
}

export function useMyPredictions() {
  return useQuery({
    queryKey: ["predictions"],
    queryFn: () => api.get<PredictionRecord[]>("/predictions"),
  });
}

export function useKnockoutReleased() {
  return useQuery({
    queryKey: ["knockout-released"],
    queryFn: () => api.get<{ released: boolean }>("/matches/knockout-released"),
  });
}

export interface PredictionInput {
  matchId: string;
  homeScore: number;
  awayScore: number;
  knockout: KnockoutEntry;
}

export function useSubmitPrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: PredictionInput) => api.post<void>("/predictions", input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["predictions"] }),
  });
}

// ---- Admin ----------------------------------------------------------------

export function useSetKnockoutReleased() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (released: boolean) => api.post<void>("/admin/knockout-released", { released }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["knockout-released"] });
      qc.invalidateQueries({ queryKey: ["matches"] });
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
      api.post<void>(`/admin/matches/${vars.matchId}/result`, {
        homeScore: vars.homeScore,
        awayScore: vars.awayScore,
        knockout: vars.knockout,
      }),
    onSuccess: () => {
      // Placar conta no ranking na hora → invalida partidas e leaderboard.
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
    },
  });
}

export function useSetMatchFinished() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { matchId: string; finished: boolean }) =>
      api.post<void>(`/admin/matches/${vars.matchId}/finished`, { finished: vars.finished }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["matches"] }),
  });
}

export function useUpdateMatchTeams() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { matchId: string; homeTeam: string; awayTeam: string }) =>
      api.post<void>(`/admin/matches/${vars.matchId}/teams`, {
        homeTeam: vars.homeTeam,
        awayTeam: vars.awayTeam,
      }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["matches"] }),
  });
}

export function useReauth() {
  return useMutation({
    mutationFn: (password: string) => api.post<void>("/auth/reauth", { password }),
  });
}

// ---- Leaderboard ----------------------------------------------------------

export function useLeaderboard(poolId: string | null) {
  return useQuery({
    queryKey: ["leaderboard", poolId],
    queryFn: () =>
      api.get<LeaderboardEntry[]>(`/leaderboard?poolId=${encodeURIComponent(poolId ?? "")}`),
    enabled: !!poolId,
  });
}
