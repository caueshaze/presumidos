import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CustomQuestion, CustomMemberPredictions, EventShowcase } from "@/types";

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

