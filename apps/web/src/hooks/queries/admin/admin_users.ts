import { api, useMutation, useQuery, useQueryClient, type AdminPushRequest, type AdminPushResult, type PoolSummary } from "./shared";

export function useUserPools(userId: string | null) {
  return useQuery({
    queryKey: ["admin-user-pools", userId],
    queryFn: () =>
      api.get<PoolSummary[]>(
        `/admin/users/${encodeURIComponent(userId ?? "")}/pools`,
      ),
    enabled: !!userId,
  });
}

export function useBlockUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { userId: string; reason: string }) =>
      api.post<void>(`/admin/users/${vars.userId}/block`, {
        reason: vars.reason,
      }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-users"] }),
  });
}

export function useUnblockUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (userId: string) =>
      api.post<void>(`/admin/users/${userId}/unblock`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-users"] }),
  });
}

export function useInvalidateUserSessions() {
  return useMutation({
    mutationFn: (userId: string) =>
      api.post<void>(`/admin/users/${userId}/invalidate-sessions`),
  });
}

export function useTriggerUserPasswordReset() {
  return useMutation({
    mutationFn: (userId: string) =>
      api.post<void>(`/admin/users/${userId}/password-reset`),
  });
}

export function useAdminSendPushToUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { userId: string; payload: AdminPushRequest }) =>
      api.post<AdminPushResult>(
        `/admin/users/${vars.userId}/push`,
        vars.payload,
      ),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-audit"] });
      qc.invalidateQueries({ queryKey: ["admin-users"] });
    },
  });
}

export function useAdminSendPushBroadcast() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (payload: AdminPushRequest) =>
      api.post<AdminPushResult>("/admin/push/broadcast", payload),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-audit"] });
      qc.invalidateQueries({ queryKey: ["admin-users"] });
    },
  });
}

