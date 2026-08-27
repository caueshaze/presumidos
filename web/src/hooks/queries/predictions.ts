import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  CustomQuestion,
  CustomMemberPredictions,
  EventShowcase,
  FootballScoringConfig,
  KnockoutEntry,
  MatchPointsSummary,
  MatchRecord,
  PredictionReopenOverride,
  PredictionRecord,
  PredictionReuseResult,
  PredictionReuseSuggestion,
} from "@/types";

// ---- Matches / predictions ------------------------------------------------

export function useMatches() {
  return useQuery({
    queryKey: ["matches"],
    queryFn: () => api.get<MatchRecord[]>("/matches"),
    // Revalida sozinho para refletir o placar ao vivo entre os ciclos do poller.
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

export function useCustomQuestions(poolId: string | null) {
  return useQuery({
    queryKey: ["custom-questions", poolId],
    queryFn: () =>
      api.get<CustomQuestion[]>(
        `/custom/questions?poolId=${encodeURIComponent(poolId ?? "")}`,
      ),
    enabled: !!poolId,
  });
}
export function useEventShowcase(poolId: string | null) {
  return useQuery({ queryKey: ["event-showcase", poolId], queryFn: () => api.get<EventShowcase>(`/custom/event-showcase?poolId=${encodeURIComponent(poolId ?? "")}`), enabled: !!poolId });
}
export function useUpdateOptionMediaProgress() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; optionId: string; seen: boolean }) => api.post<void>("/custom/media-progress", vars),
    onSuccess: (_data, vars) => qc.setQueryData<CustomQuestion[]>(["custom-questions", vars.poolId], (questions) => questions?.map((question) => ({ ...question, options: question.options.map((option) => option.id === vars.optionId ? { ...option, mediaSeen: vars.seen } : option) }))),
  });
}
export function useCustomMemberPredictions(poolId: string | null) {
  return useQuery({
    queryKey: ["custom-member-predictions", poolId],
    queryFn: () =>
      api.get<CustomMemberPredictions[]>(
        `/pools/${encodeURIComponent(poolId ?? "")}/custom-member-predictions`,
      ),
    enabled: !!poolId,
  });
}

function updateCurrentCustomPrediction(
  qc: ReturnType<typeof useQueryClient>,
  poolId: string,
  itemId: string,
  patch: Partial<
    Pick<
      CustomQuestion,
      "currentOptionId" | "currentOptionIds" | "currentValue"
    >
  >,
) {
  qc.setQueryData<CustomQuestion[]>(["custom-questions", poolId], (questions) =>
    questions?.map((question) =>
      question.itemId === itemId ? { ...question, ...patch } : question,
    ),
  );
}

export function useSubmitCustomPrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; itemId: string; optionId: string }) =>
      api.post<void>("/custom/predictions", vars),
    onSuccess: (_data, vars) => {
      updateCurrentCustomPrediction(qc, vars.poolId, vars.itemId, {
        currentOptionId: vars.optionId,
      });
    },
  });
}
export function useSubmitNumericPrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; itemId: string; value: string }) =>
      api.post<void>("/custom/numeric-predictions", vars),
    onSuccess: (_data, vars) => {
      updateCurrentCustomPrediction(qc, vars.poolId, vars.itemId, {
        currentValue: vars.value,
      });
    },
  });
}
export function useSubmitMultipleChoicePrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      itemId: string;
      optionIds: string[];
    }) => api.post<void>("/custom/multiple-choice-predictions", vars),
    onSuccess: (_data, vars) =>
      updateCurrentCustomPrediction(qc, vars.poolId, vars.itemId, {
        currentOptionIds: vars.optionIds,
      }),
  });
}

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
