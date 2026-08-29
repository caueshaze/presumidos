import { api, useMutation, useQuery, useQueryClient, type LeaderboardEntry, type MemberPredictions, type PointAdjustment, type PredictionScoreBreakdown } from "./shared";

export type PoolTieBreakPriority = { itemId: string; title: string; kind: string; priority: number };
export type PoolTieBreakConfig = { mode: "inherit" | "custom" | "disabled"; effectivePriorities: PoolTieBreakPriority[]; customPriorities: PoolTieBreakPriority[]; canEdit: boolean };
export function usePoolTieBreak(poolId: string | null) {
  return useQuery({ queryKey: ["pool-tie-break", poolId], queryFn: () => api.get<PoolTieBreakConfig>(`/pools/${encodeURIComponent(poolId ?? "")}/tie-break`), enabled: !!poolId });
}
export function useUpdatePoolTieBreak() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (vars: { poolId: string; mode: PoolTieBreakConfig["mode"]; itemIds?: string[] }) => api.post<PoolTieBreakConfig>(`/pools/${vars.poolId}/tie-break`, vars), onSuccess: (_data, vars) => { qc.invalidateQueries({ queryKey: ["pool-tie-break", vars.poolId] }); qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] }); } });
}

export function useLeaderboard(poolId: string | null) {
  return useQuery({
    queryKey: ["leaderboard", poolId],
    queryFn: () =>
      api.get<LeaderboardEntry[]>(
        `/leaderboard?poolId=${encodeURIComponent(poolId ?? "")}`,
      ),
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
      api.get<PointAdjustment[]>(
        `/pools/${encodeURIComponent(poolId ?? "")}/adjustments`,
      ),
    enabled: !!poolId,
  });
}

export function useAddAdjustment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      userId: string;
      delta: number;
      reason: string;
    }) =>
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
