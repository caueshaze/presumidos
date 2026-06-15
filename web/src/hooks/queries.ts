import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AdminMatchRecord,
  AdminOverview,
  AdminPredictionRow,
  AdminSettings,
  AdminUserRecord,
  AuditLogEntry,
  AuthResult,
  KnockoutEntry,
  LeaderboardEntry,
  MatchPointsSummary,
  MatchRecord,
  MemberPredictions,
  NotificationPreference,
  NotificationStatus,
  PointAdjustment,
  PoolSummary,
  PredictionReopenOverride,
  PredictionScoreBreakdown,
  PredictionRecord,
  ScoringJob,
  SyncStatus,
  UserPublic,
} from "@/types";

function normalizeAdminUserRecord(input: AdminUserRecord | UserPublic): AdminUserRecord {
  if ("user" in input) return input;
  return {
    user: input,
    poolCount: 0,
  };
}

function normalizeAdminMatchRecord(input: AdminMatchRecord | MatchRecord): AdminMatchRecord {
  if ("matchRecord" in input) return input;
  return {
    matchRecord: input,
    adminStatus:
      input.resultSource === "manual"
        ? "manually_corrected"
        : input.finished && input.resultSource === "api"
          ? "finalized"
          : input.liveStatus && !input.finished
            ? "live"
            : "scheduled",
    lastAuditAt: null,
  };
}

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

export function useMyMatchPoints() {
  return useQuery({
    queryKey: ["my-match-points"],
    queryFn: () => api.get<MatchPointsSummary[]>("/scoring/my-points"),
    // Acompanha os resultados recém-lançados, no mesmo ritmo dos matches.
    refetchInterval: 60_000,
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
      api.post<void>(`/admin/matches/${vars.matchId}/finished`, { finished: vars.finished }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
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
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
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

export function useDeleteAccount() {
  return useMutation({
    mutationFn: () => api.post<void>("/auth/delete"),
  });
}

export function useNotificationStatus() {
  return useQuery({
    queryKey: ["notification-status"],
    queryFn: () => api.get<NotificationStatus>("/notifications/status"),
  });
}

export function useUpdateNotificationPreference() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: NotificationPreference) =>
      api.post<NotificationPreference>("/notifications/preferences", vars),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["notification-status"] }),
  });
}

// ---- Leaderboard ----------------------------------------------------------

export function useLeaderboard(poolId: string | null) {
  return useQuery({
    queryKey: ["leaderboard", poolId],
    queryFn: () =>
      api.get<LeaderboardEntry[]>(`/leaderboard?poolId=${encodeURIComponent(poolId ?? "")}`),
    enabled: !!poolId,
    // Revalida sozinho para refletir a pontuação ao vivo (provisória) durante os jogos.
    refetchInterval: 60_000,
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

export function usePoolBreakdowns(poolId: string | null) {
  return useQuery({
    queryKey: ["pool-breakdowns", poolId],
    queryFn: () =>
      api.get<PredictionScoreBreakdown[]>(
        `/pools/${encodeURIComponent(poolId ?? "")}/breakdowns`,
      ),
    enabled: !!poolId,
    // Acompanha resultados recém-lançados durante os jogos.
    refetchInterval: 60_000,
  });
}

// ---- Admin: gestão de membros de bolões -----------------------------------

export function useAdminPools() {
  return useQuery({ queryKey: ["admin-pools"], queryFn: () => api.get<PoolSummary[]>("/admin/pools") });
}

export function useAdminUsers() {
  return useQuery({
    queryKey: ["admin-users"],
    queryFn: async () => {
      const data = await api.get<Array<AdminUserRecord | UserPublic>>("/admin/users");
      return data.map(normalizeAdminUserRecord);
    },
  });
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
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
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
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useAdminOverview() {
  return useQuery({
    queryKey: ["admin-overview"],
    queryFn: () => api.get<AdminOverview>("/admin/overview"),
  });
}

export function useAdminMatches(filters: {
  phase?: string;
  groupName?: string;
  date?: string;
  status?: string;
  origin?: string;
}) {
  const params = new URLSearchParams();
  if (filters.phase) params.set("phase", filters.phase);
  if (filters.groupName) params.set("groupName", filters.groupName);
  if (filters.date) params.set("date", filters.date);
  if (filters.status) params.set("status", filters.status);
  if (filters.origin) params.set("origin", filters.origin);

  return useQuery({
    queryKey: ["admin-matches", filters],
    queryFn: async () => {
      const data = await api.get<Array<AdminMatchRecord | MatchRecord>>(
        `/admin/matches${params.toString() ? `?${params.toString()}` : ""}`,
      );
      return data.map(normalizeAdminMatchRecord);
    },
  });
}

export function useAdminMatchAudit(matchId: string | null) {
  return useQuery({
    queryKey: ["admin-match-audit", matchId],
    queryFn: () => api.get<AuditLogEntry[]>(`/admin/matches/${encodeURIComponent(matchId ?? "")}/audit`),
    enabled: !!matchId,
  });
}

export function useSyncStatus() {
  return useQuery({
    queryKey: ["admin-sync-status"],
    queryFn: () => api.get<SyncStatus | null>("/admin/sync/status"),
  });
}

export function useRunSyncNow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.post<SyncStatus>("/admin/sync/run-now"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-sync-status"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

export function useAdminPredictions(filters: {
  matchId?: string;
  userId?: string;
  poolId?: string;
  missingOnly?: boolean;
}) {
  const params = new URLSearchParams();
  if (filters.matchId) params.set("matchId", filters.matchId);
  if (filters.userId) params.set("userId", filters.userId);
  if (filters.poolId) params.set("poolId", filters.poolId);
  if (filters.missingOnly) params.set("missingOnly", "true");

  return useQuery({
    queryKey: ["admin-predictions", filters],
    queryFn: () =>
      api.get<AdminPredictionRow[]>(
        `/admin/predictions${params.toString() ? `?${params.toString()}` : ""}`,
      ),
  });
}

export function useReopenPrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      userId: string;
      reason: string;
      expiresAt: string;
    }) => api.post<PredictionReopenOverride>("/admin/predictions/reopen", vars),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-predictions"] }),
  });
}

export function useRevokePredictionReopen() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (overrideId: string) =>
      api.post<void>("/admin/predictions/reopen/revoke", { overrideId }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-predictions"] }),
  });
}

export function useRecalculateMatch() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (matchId: string) =>
      api.post<ScoringJob>("/admin/scoring/recalculate-match", { matchId }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useRecalculateAll() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.post<ScoringJob>("/admin/scoring/recalculate-all"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useUserBreakdown(userId: string | null, poolId: string | null) {
  return useQuery({
    queryKey: ["admin-user-breakdown", userId, poolId],
    queryFn: () =>
      api.get<PredictionScoreBreakdown[]>(
        `/admin/scoring/users/${encodeURIComponent(userId ?? "")}/breakdown?poolId=${encodeURIComponent(poolId ?? "")}`,
      ),
    enabled: !!userId && !!poolId,
  });
}

export function useUserPools(userId: string | null) {
  return useQuery({
    queryKey: ["admin-user-pools", userId],
    queryFn: () => api.get<PoolSummary[]>(`/admin/users/${encodeURIComponent(userId ?? "")}/pools`),
    enabled: !!userId,
  });
}

export function useBlockUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { userId: string; reason: string }) =>
      api.post<void>(`/admin/users/${vars.userId}/block`, { reason: vars.reason }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-users"] }),
  });
}

export function useUnblockUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (userId: string) => api.post<void>(`/admin/users/${userId}/unblock`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-users"] }),
  });
}

export function useInvalidateUserSessions() {
  return useMutation({
    mutationFn: (userId: string) => api.post<void>(`/admin/users/${userId}/invalidate-sessions`),
  });
}

export function useTriggerUserPasswordReset() {
  return useMutation({
    mutationFn: (userId: string) => api.post<void>(`/admin/users/${userId}/password-reset`),
  });
}

export function useAdminAudit(filters: {
  action?: string;
  actorUserId?: string;
  targetType?: string;
  targetId?: string;
}) {
  const params = new URLSearchParams();
  if (filters.action) params.set("action", filters.action);
  if (filters.actorUserId) params.set("actorUserId", filters.actorUserId);
  if (filters.targetType) params.set("targetType", filters.targetType);
  if (filters.targetId) params.set("targetId", filters.targetId);

  return useQuery({
    queryKey: ["admin-audit", filters],
    queryFn: () =>
      api.get<AuditLogEntry[]>(`/admin/audit${params.toString() ? `?${params.toString()}` : ""}`),
  });
}

export function useAdminSettings() {
  return useQuery({
    queryKey: ["admin-settings"],
    queryFn: () => api.get<AdminSettings>("/admin/settings"),
  });
}

export function usePublicSettings() {
  return useQuery({
    queryKey: ["public-settings"],
    queryFn: () => api.get<AdminSettings>("/settings/public"),
    staleTime: 60_000,
  });
}

export function useSaveAdminSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (settings: AdminSettings) => api.post<AdminSettings>("/admin/settings", settings),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-settings"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}
