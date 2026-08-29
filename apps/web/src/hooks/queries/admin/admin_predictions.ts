import { api, useMutation, useQuery, useQueryClient, type AdminPredictionRow, type PredictionReopenOverride, type PredictionScoreBreakdown, type ScoringJob } from "./shared";

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

