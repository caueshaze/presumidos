import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AuthResult,
  KnockoutEntry,
  LeaderboardEntry,
  MatchRecord,
  MemberPredictions,
  PointAdjustment,
  PoolSummary,
  PredictionRecord,
  UserPublic,
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

export function useDeletePool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<void>(`/pools/${poolId}/delete`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["pools"] }),
  });
}

// ---- Matches / predictions ------------------------------------------------

export function useMatches() {
  return useQuery({
    queryKey: ["matches"],
    queryFn: () => api.get<MatchRecord[]>("/matches"),
    // Revalida sozinho para refletir o placar ao vivo entre os ciclos do poller.
    refetchInterval: 60_000,
  });
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

export function useChangeUsername() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (username: string) => api.post<UserPublic>("/auth/username", { username }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["current-user"] }),
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

// ---- Ajustes manuais de pontos --------------------------------------------

export function usePoolAdjustments(poolId: string | null) {
  return useQuery({
    queryKey: ["pool-adjustments", poolId],
    queryFn: () =>
      api.get<PointAdjustment[]>(`/pools/${encodeURIComponent(poolId ?? "")}/adjustments`),
    enabled: !!poolId,
  });
}

export function useAddAdjustment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; userId: string; delta: number; reason: string }) =>
      api.post<void>(`/pools/${vars.poolId}/adjustments`, {
        userId: vars.userId,
        delta: vars.delta,
        reason: vars.reason,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["pool-adjustments", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}

export function useRemoveAdjustment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; adjustmentId: string }) =>
      api.post<void>(`/pools/${vars.poolId}/adjustments/remove`, {
        adjustmentId: vars.adjustmentId,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["pool-adjustments", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}

// ---- Palpites do bolão ----------------------------------------------------

export function usePoolMemberPredictions(poolId: string | null) {
  return useQuery({
    queryKey: ["pool-member-predictions", poolId],
    queryFn: () =>
      api.get<MemberPredictions[]>(
        `/pools/${encodeURIComponent(poolId ?? "")}/member-predictions`,
      ),
    enabled: !!poolId,
  });
}

// ---- Admin: gestão de membros de bolões -----------------------------------

export function useAdminPools() {
  return useQuery({ queryKey: ["admin-pools"], queryFn: () => api.get<PoolSummary[]>("/admin/pools") });
}

export function useAdminUsers() {
  return useQuery({ queryKey: ["admin-users"], queryFn: () => api.get<UserPublic[]>("/admin/users") });
}

export function useAdminPoolMembers(poolId: string | null) {
  return useQuery({
    queryKey: ["admin-pool-members", poolId],
    queryFn: () =>
      api.get<UserPublic[]>(`/admin/pools/${encodeURIComponent(poolId ?? "")}/members`),
    enabled: !!poolId,
  });
}

export function useAddPoolMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; userId: string }) =>
      api.post<void>(`/admin/pools/${vars.poolId}/members`, { userId: vars.userId }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["admin-pool-members", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["admin-pools"] });
    },
  });
}

export function useRemovePoolMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; userId: string }) =>
      api.post<void>(`/admin/pools/${vars.poolId}/members/remove`, { userId: vars.userId }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["admin-pool-members", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["admin-pools"] });
    },
  });
}
