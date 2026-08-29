import { api, useMutation, useQuery, useQueryClient, type AdminOverview, type AdminUserRecord, type PoolSummary, type PoolReport, type PoolReportStatus, type UserPublic } from "./shared";

function normalizeAdminUserRecord(
  input: AdminUserRecord | UserPublic,
): AdminUserRecord {
  if ("user" in input) return input;
  return {
    user: input,
    poolCount: 0,
  };
}

export function useAdminPools() {
  return useQuery({
    queryKey: ["admin-pools"],
    queryFn: () => api.get<PoolSummary[]>("/admin/pools"),
  });
}

export function useAdminUsers() {
  return useQuery({
    queryKey: ["admin-users"],
    queryFn: async () => {
      const data =
        await api.get<Array<AdminUserRecord | UserPublic>>("/admin/users");
      return data.map(normalizeAdminUserRecord);
    },
  });
}

export function useAdminPoolMembers(poolId: string | null) {
  return useQuery({
    queryKey: ["admin-pool-members", poolId],
    queryFn: () =>
      api.get<UserPublic[]>(
        `/admin/pools/${encodeURIComponent(poolId ?? "")}/members`,
      ),
    enabled: !!poolId,
  });
}

export function useAddPoolMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; userId: string }) =>
      api.post<void>(`/admin/pools/${vars.poolId}/members`, {
        userId: vars.userId,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["admin-pool-members", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["admin-pools"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useRemovePoolMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; userId: string }) =>
      api.post<void>(`/admin/pools/${vars.poolId}/members/remove`, {
        userId: vars.userId,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["admin-pool-members", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["admin-pools"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useAdminPoolReports(status?: PoolReportStatus) {
  return useQuery({
    queryKey: ["admin-pool-reports", status ?? "all"],
    queryFn: () =>
      api.get<PoolReport[]>(
        `/admin/pool-reports${status ? `?status=${encodeURIComponent(status)}` : ""}`,
      ),
  });
}

export function useUpdatePoolReportStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { reportId: string; status: PoolReportStatus }) =>
      api.post<PoolReport>(`/admin/pool-reports/${vars.reportId}/status`, {
        status: vars.status,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["admin-pool-reports"] });
      qc.invalidateQueries({ queryKey: ["admin-audit"] });
    },
  });
}

export function useAdminOverview() {
  return useQuery({
    queryKey: ["admin-overview"],
    queryFn: () => api.get<AdminOverview>("/admin/overview"),
  });
}

