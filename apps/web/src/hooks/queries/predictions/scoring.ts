import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { FootballScoringConfig } from "@/types";

export function useFootballScoring(poolId: string | null) {
  return useQuery({
    queryKey: ["pool-scoring", poolId, "football"],
    queryFn: () =>
      api.get<FootballScoringConfig>(
        `/pools/${encodeURIComponent(poolId ?? "")}/scoring/football`,
      ),
    enabled: !!poolId,
  });
}

export function useUpdateFootballScoring() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string } & FootballScoringConfig) =>
      api.post<void>(`/pools/${vars.poolId}/scoring/football`, vars),
    onSuccess: (_data, vars) =>
      qc.invalidateQueries({ queryKey: ["pool-scoring", vars.poolId] }),
  });
}

export function useUpdateCustomScoring() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      itemId: string;
      correctPoints: number;
      incorrectPoints: number;
    }) =>
      api.post<void>(
        `/pools/${vars.poolId}/scoring/items/${vars.itemId}`,
        vars,
      ),
    onSuccess: (_data, vars) =>
      qc.invalidateQueries({ queryKey: ["custom-questions", vars.poolId] }),
  });
}

export function useSetCustomResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { itemId: string; optionId: string; poolId: string }) =>
      api.post<void>(`/admin/custom/questions/${vars.itemId}/result`, {
        optionId: vars.optionId,
        poolId: vars.poolId,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["custom-questions", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}
export function useUpdateNumericScoring() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      itemId: string;
      exactPoints: number;
      tolerance: string;
      withinTolerancePoints: number;
      incorrectPoints: number;
    }) =>
      api.post<void>(
        `/pools/${vars.poolId}/scoring/numeric/${vars.itemId}`,
        vars,
      ),
    onSuccess: (_d, vars) =>
      qc.invalidateQueries({ queryKey: ["custom-questions", vars.poolId] }),
  });
}
export function useSetNumericResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; itemId: string; value: string }) =>
      api.post<void>(`/admin/custom/numeric/${vars.itemId}/result`, {
        value: vars.value,
        poolId: vars.poolId,
      }),
    onSuccess: (_d, vars) => {
      qc.invalidateQueries({ queryKey: ["custom-questions", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}
export function useUpdateMultipleChoiceScoring() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      itemId: string;
      exactPoints: number;
      partialPoints: number;
      incorrectPoints: number;
    }) =>
      api.post<void>(
        `/pools/${vars.poolId}/scoring/multiple-choice/${vars.itemId}`,
        vars,
      ),
    onSuccess: (_data, vars) =>
      qc.invalidateQueries({ queryKey: ["custom-questions", vars.poolId] }),
  });
}
export function useSetMultipleChoiceResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      itemId: string;
      optionIds: string[];
    }) =>
      api.post<void>(`/admin/custom/multiple-choice/${vars.itemId}/result`, {
        optionIds: vars.optionIds,
        poolId: vars.poolId,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["custom-questions", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}
