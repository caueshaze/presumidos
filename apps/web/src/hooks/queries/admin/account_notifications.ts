import { api, useMutation, useQuery, useQueryClient, type NotificationPreference, type NotificationStatus, type UserPublic } from "./shared";

export function useReauth() {
  return useMutation({
    mutationFn: (password: string) =>
      api.post<void>("/auth/reauth", { password }),
  });
}

export function useChangeUsername() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (username: string) =>
      api.post<UserPublic>("/auth/username", { username }),
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
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["notification-status"] }),
  });
}

export function useReactToPrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      targetUserId: string;
      matchId: string;
      emoji: string;
    }) =>
      api.post<void>(`/pools/${vars.poolId}/prediction-reactions`, {
        targetUserId: vars.targetUserId,
        matchId: vars.matchId,
        emoji: vars.emoji,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({
        queryKey: ["pool-member-predictions", vars.poolId],
      });
    },
  });
}

export function useMarkPredictionReactionsSeen() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) =>
      api.post<void>(
        `/pools/${encodeURIComponent(poolId)}/prediction-reactions/mark-seen`,
      ),
    onSuccess: (_data, poolId) => {
      qc.invalidateQueries({ queryKey: ["pool-member-predictions", poolId] });
    },
  });
}

// ---- Leaderboard ----------------------------------------------------------

