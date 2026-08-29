import { api, useMutation, useQuery, useQueryClient, type AdminSettings, type AuditLogEntry } from "./shared";

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
      api.get<AuditLogEntry[]>(
        `/admin/audit${params.toString() ? `?${params.toString()}` : ""}`,
      ),
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
    staleTime: 30_000,
    // A edição da final é uma chave operacional: visitantes já abertos devem
    // recebê-la sem precisar recarregar a página.
    refetchInterval: 30_000,
  });
}

export function useSaveAdminSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (settings: AdminSettings) =>
      api.post<AdminSettings>("/admin/settings", settings),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-settings"] });
      qc.invalidateQueries({ queryKey: ["public-settings"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}
