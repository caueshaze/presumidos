import { api, useMutation, useQuery, useQueryClient, type AdminMatchRecord, type AuditLogEntry, type MatchRecord, type SyncStatus } from "./shared";
import { normalizeAdminMatchRecord } from "./match_management";

export function useAdminMatches(filters: {
  phase?: string;
  groupName?: string;
  date?: string;
  status?: string;
  origin?: string;
}) {
  const params = new URLSearchParams();
  if (filters.phase) params.set("phase", filters.phase);
  if (filters.groupName) params.set("groupName", filters.groupName);
  if (filters.date) params.set("date", filters.date);
  if (filters.status) params.set("status", filters.status);
  if (filters.origin) params.set("origin", filters.origin);

  return useQuery({
    queryKey: ["admin-matches", filters],
    queryFn: async () => {
      const data = await api.get<Array<AdminMatchRecord | MatchRecord>>(
        `/admin/matches${params.toString() ? `?${params.toString()}` : ""}`,
      );
      return data.map(normalizeAdminMatchRecord);
    },
  });
}

export function useAdminMatchAudit(matchId: string | null) {
  return useQuery({
    queryKey: ["admin-match-audit", matchId],
    queryFn: () =>
      api.get<AuditLogEntry[]>(
        `/admin/matches/${encodeURIComponent(matchId ?? "")}/audit`,
      ),
    enabled: !!matchId,
  });
}

export function useSyncStatus() {
  return useQuery({
    queryKey: ["admin-sync-status"],
    queryFn: () => api.get<SyncStatus | null>("/admin/sync/status"),
  });
}

export function useRunSyncNow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.post<SyncStatus>("/admin/sync/run-now"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-sync-status"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

export function useRunBackfill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.post<SyncStatus>("/admin/sync/backfill"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-sync-status"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

