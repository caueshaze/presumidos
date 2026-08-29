import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AdminEventRecord,
  PoolSummary,
  PoolDashboardSummary,
  PoolReport,
  PublicPoolInvitePreview,
  PoolLifecycleState,
  EventRecord,
} from "@/types";

// ---- Pools ----------------------------------------------------------------

export function usePools() {
  return useQuery({
    queryKey: ["pools"],
    queryFn: () => api.get<PoolSummary[]>("/pools"),
  });
}

export function useDashboardPools() {
  return useQuery({
    queryKey: ["pools", "dashboard"],
    queryFn: () => api.get<PoolDashboardSummary[]>("/pools/dashboard"),
  });
}

export type MyEvent = {
  id: string;
  name: string;
  status: "draft" | "active" | "finished";
  startsAt: string | null;
  endsAt: string | null;
};
export function useMyEvents() {
  return useQuery({
    queryKey: ["custom-events", "mine"],
    queryFn: () => api.get<MyEvent[]>("/custom/events/mine"),
  });
}
export function useAvailableEvents() {
  return useQuery({
    queryKey: ["custom-events", "available"],
    queryFn: () => api.get<MyEvent[]>("/custom/events/available"),
  });
}

export function useAdminEvents() {
  return useQuery({
    queryKey: ["admin-events"],
    queryFn: () => api.get<AdminEventRecord[]>("/admin/events"),
  });
}

export function useFinishEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (eventId: string) => api.post<EventRecord>(`/admin/events/${eventId}/finish`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-events"] });
      qc.invalidateQueries({ queryKey: ["pools"] });
    },
  });
}

export function useDeleteEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (eventId: string) =>
      api.post<void>(`/custom/events/${eventId}/delete`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["custom-events"] });
      qc.invalidateQueries({ queryKey: ["admin-events"] });
      qc.invalidateQueries({ queryKey: ["pools"] });
    },
  });
}

export function useAdminDeleteEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (eventId: string) =>
      api.post<void>(`/admin/events/${eventId}/delete`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-events"] });
      qc.invalidateQueries({ queryKey: ["custom-events"] });
      qc.invalidateQueries({ queryKey: ["pools"] });
    },
  });
}

export function useSetEventPoolCreation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { eventId: string; enabled: boolean }) =>
      api.post<void>(`/admin/events/${input.eventId}/pool-creation`, { enabled: input.enabled }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-events"] });
      qc.invalidateQueries({ queryKey: ["custom-events", "available"] });
    },
  });
}

export function usePublishEventVersion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { eventId: string; versionId: string }) =>
      api.post<void>(`/admin/events/${input.eventId}/versions/${input.versionId}/publish`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-events"] });
      qc.invalidateQueries({ queryKey: ["custom-events"] });
    },
  });
}

export function useCreatePool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: string | { name: string; eventId?: string }) =>
      api.post<PoolSummary>(
        "/pools",
        typeof input === "string" ? { name: input } : input,
      ),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["pools"] }),
  });
}

export function useJoinPool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (inviteCode: string) =>
      api.post<PoolSummary>("/pools/join", { inviteCode }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["pools"] }),
  });
}

export function usePublicPoolInvitePreview(inviteCode: string) {
  return useQuery({
    queryKey: ["public-pool-invite", inviteCode],
    queryFn: () =>
      api.get<PublicPoolInvitePreview>(
        `/public/pools/invite/${encodeURIComponent(inviteCode)}`,
      ),
    enabled: inviteCode.length > 0,
    retry: false,
  });
}

export function useDeletePool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<void>(`/pools/${poolId}/delete`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["pools"] }),
  });
}

export function useLeavePool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<void>(`/pools/${poolId}/leave`),
    onSuccess: (_data, poolId) => {
      qc.invalidateQueries({ queryKey: ["pools"] });
      qc.invalidateQueries({ queryKey: ["pools", "dashboard"] });
      qc.invalidateQueries({ queryKey: ["pool-member-predictions", poolId] });
    },
  });
}

function invalidatePoolLifecycle(qc: ReturnType<typeof useQueryClient>, poolId: string) {
  qc.invalidateQueries({ queryKey: ["pools"] });
  qc.invalidateQueries({ queryKey: ["pools", "dashboard"] });
  qc.invalidateQueries({ queryKey: ["pool-member-predictions", poolId] });
  qc.invalidateQueries({ queryKey: ["leaderboard", poolId] });
  qc.invalidateQueries({ queryKey: ["pool-breakdowns", poolId] });
  qc.invalidateQueries({ queryKey: ["custom-questions", poolId] });
}

export function useClosePoolPredictions() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<PoolLifecycleState>(`/pools/${poolId}/close-predictions`),
    onSuccess: (_data, poolId) => invalidatePoolLifecycle(qc, poolId),
  });
}

export function useClosePool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<PoolLifecycleState>(`/pools/${poolId}/close`),
    onSuccess: (_data, poolId) => invalidatePoolLifecycle(qc, poolId),
  });
}

export function useCreatePoolReport() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; category: PoolReport["category"]; details: string }) =>
      api.post<PoolReport>(`/pools/${vars.poolId}/reports`, {
        category: vars.category,
        details: vars.details,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-pool-reports"] });
      qc.invalidateQueries({ queryKey: ["admin-audit"] });
    },
  });
}
