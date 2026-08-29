import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { KnockoutEntry, MatchPointsSummary, MatchRecord, PredictionReopenOverride, PredictionRecord, PredictionReuseResult, PredictionReuseSuggestion } from "@/types";

// ---- Matches / predictions ------------------------------------------------

export function useMatches() {
  return useQuery({
    queryKey: ["matches"],
    queryFn: () => api.get<MatchRecord[]>("/matches"),
    // Atualização periódica normal da lista de partidas.
    refetchInterval: 60_000,
  });
}

export function useMyPredictions(poolId: string | null) {
  return useQuery({
    queryKey: ["predictions", poolId],
    queryFn: () => api.get<PredictionRecord[]>(`/predictions?poolId=${encodeURIComponent(poolId ?? "")}`),
    enabled: !!poolId,
  });
}

/** Reaberturas administrativas ativas para o usuário logado (libera palpite mesmo travado). */
export function useMyPredictionOverrides() {
  return useQuery({
    queryKey: ["predictions", "reopened"],
    queryFn: () => api.get<PredictionReopenOverride[]>("/predictions/reopened"),
    refetchInterval: 60_000,
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
  poolId: string;
  matchId: string;
  homeScore: number;
  awayScore: number;
  knockout: KnockoutEntry;
}

export function useSubmitPrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: PredictionInput) =>
      api.post<void>("/predictions", input),
    onSuccess: (_data, vars) => qc.invalidateQueries({ queryKey: ["predictions", vars.poolId] }),
  });
}

export function usePredictionReuseSuggestion(poolId: string | null) {
  return useQuery({
    queryKey: ["prediction-reuse", poolId],
    queryFn: () => api.get<PredictionReuseSuggestion>(`/pools/${encodeURIComponent(poolId ?? "")}/prediction-reuse`),
    enabled: false,
    retry: false,
  });
}

export function useCopyPredictionsReuse() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<PredictionReuseResult>(`/pools/${encodeURIComponent(poolId)}/prediction-reuse/copy`),
    onSuccess: (_data, poolId) => {
      qc.invalidateQueries({ queryKey: ["prediction-reuse", poolId] });
      qc.invalidateQueries({ queryKey: ["custom-questions", poolId] });
      qc.invalidateQueries({ queryKey: ["predictions", poolId] });
      qc.invalidateQueries({ queryKey: ["pools", "dashboard"] });
    },
  });
}

export function useStartPredictionsEmpty() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<PredictionReuseResult>(`/pools/${encodeURIComponent(poolId)}/prediction-reuse/start-empty`),
    onSuccess: (_data, poolId) => qc.invalidateQueries({ queryKey: ["prediction-reuse", poolId] }),
  });
}

