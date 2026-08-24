import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AdminEventRecord,
  AdminMatchRecord,
  AdminOverview,
  AdminPredictionRow,
  AdminPushRequest,
  AdminPushResult,
  AdminSettings,
  AdminUserRecord,
  AuditLogEntry,
  AuthResult,
  CustomQuestion,
  CustomMemberPredictions,
  EventShowcase,
  FootballScoringConfig,
  FixtureCheckResult,
  KnockoutEntry,
  LeaderboardEntry,
  MatchPointsSummary,
  MatchRecord,
  MemberPredictions,
  NotificationPreference,
  NotificationStatus,
  PointAdjustment,
  PoolSummary,
  PoolDashboardSummary,
  PredictionReopenOverride,
  PredictionScoreBreakdown,
  PredictionRecord,
  ScoringJob,
  SyncStatus,
  UserPublic,
  EventRecord,
} from "@/types";

function normalizeAdminUserRecord(
  input: AdminUserRecord | UserPublic,
): AdminUserRecord {
  if ("user" in input) return input;
  return {
    user: input,
    poolCount: 0,
  };
}

type FlatAdminMatchRecord = MatchRecord & Omit<AdminMatchRecord, "matchRecord">;

function normalizeAdminMatchRecord(
  input: AdminMatchRecord | FlatAdminMatchRecord | MatchRecord,
): AdminMatchRecord {
  if ("matchRecord" in input) return input;

  const matchRecord: MatchRecord = {
    id: input.id,
    homeTeam: input.homeTeam,
    awayTeam: input.awayTeam,
    kickoff: input.kickoff,
    groupName: input.groupName,
    phase: input.phase,
    homeScore: input.homeScore,
    awayScore: input.awayScore,
    qualifier: input.qualifier,
    wentToPenalties: input.wentToPenalties,
    penaltyHomeScore: input.penaltyHomeScore,
    penaltyAwayScore: input.penaltyAwayScore,
    finished: input.finished,
    liveHomeScore: input.liveHomeScore,
    liveAwayScore: input.liveAwayScore,
    liveStatus: input.liveStatus,
    liveElapsed: input.liveElapsed,
    resultSource: input.resultSource,
    resultSyncedAt: input.resultSyncedAt,
    resultExternalRawStatus: input.resultExternalRawStatus,
    liveUpdatedAt: input.liveUpdatedAt,
  };

  if ("adminStatus" in input) {
    return {
      matchRecord,
      adminStatus: input.adminStatus,
      lastAuditAt: input.lastAuditAt,
      externalFixtureId: input.externalFixtureId,
      autoHomeScore: input.autoHomeScore,
      autoAwayScore: input.autoAwayScore,
      autoPenaltyHomeScore: input.autoPenaltyHomeScore,
      autoPenaltyAwayScore: input.autoPenaltyAwayScore,
      autoQualifier: input.autoQualifier,
      autoStatus: input.autoStatus,
      autoDetectedAt: input.autoDetectedAt,
      sourceLastCheckedAt: input.sourceLastCheckedAt,
      sourceLastStatus: input.sourceLastStatus,
    };
  }

  return {
    matchRecord,
    adminStatus:
      input.resultSource === "manual"
        ? "manually_corrected"
        : input.finished && input.resultSource === "api"
          ? "finalized"
          : input.liveStatus && !input.finished
            ? "live"
            : "scheduled",
    lastAuditAt: null,
    externalFixtureId: null,
    autoHomeScore: null,
    autoAwayScore: null,
    autoPenaltyHomeScore: null,
    autoPenaltyAwayScore: null,
    autoQualifier: null,
    autoStatus: null,
    autoDetectedAt: null,
    sourceLastCheckedAt: null,
    sourceLastStatus: null,
  };
}

// ---- Auth mutations -------------------------------------------------------

export function useLogin() {
  return useMutation({
    mutationFn: (vars: { username: string; password: string }) =>
      api.postPublic<AuthResult>("/auth/login", vars),
  });
}

export function useRegisterRequest() {
  return useMutation({
    mutationFn: (vars: { username: string; email: string; password: string }) =>
      api.postPublic<void>("/auth/register", vars),
  });
}

export function useRegisterConfirm() {
  return useMutation({
    mutationFn: (vars: { email: string; code: string }) =>
      api.postPublic<AuthResult>("/auth/register/confirm", vars),
  });
}

export function usePasswordResetRequest() {
  return useMutation({
    mutationFn: (vars: { email: string }) =>
      api.postPublic<void>("/auth/password-reset", vars),
  });
}

export function usePasswordResetConfirm() {
  return useMutation({
    mutationFn: (vars: { email: string; code: string; newPassword: string }) =>
      api.postPublic<void>("/auth/password-reset/confirm", vars),
  });
}

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

export function useDeletePool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (poolId: string) => api.post<void>(`/pools/${poolId}/delete`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["pools"] }),
  });
}

// ---- Matches / predictions ------------------------------------------------

export function useMatches() {
  return useQuery({
    queryKey: ["matches"],
    queryFn: () => api.get<MatchRecord[]>("/matches"),
    // Revalida sozinho para refletir o placar ao vivo entre os ciclos do poller.
    refetchInterval: 60_000,
  });
}

export function useMyPredictions() {
  return useQuery({
    queryKey: ["predictions"],
    queryFn: () => api.get<PredictionRecord[]>("/predictions"),
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
    onSuccess: () => qc.invalidateQueries({ queryKey: ["predictions"] }),
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
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["custom-questions", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}

// ---- Admin ----------------------------------------------------------------

export function useSetKnockoutReleased() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (released: boolean) =>
      api.post<void>("/admin/knockout-released", { released }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["knockout-released"] });
      qc.invalidateQueries({ queryKey: ["matches"] });
      // Mantém o rascunho de configurações em sincronia para que um "Salvar
      // configurações" posterior não reverta a liberação feita aqui.
      qc.invalidateQueries({ queryKey: ["admin-settings"] });
    },
  });
}

export function useSetMatchResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      homeScore: number;
      awayScore: number;
      knockout: KnockoutEntry;
    }) =>
      api.post<MatchRecord>(`/admin/matches/${vars.matchId}/result`, {
        homeScore: vars.homeScore,
        awayScore: vars.awayScore,
        knockout: vars.knockout,
      }),
    onSuccess: () => {
      // Placar conta no ranking na hora → invalida partidas e leaderboard.
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
    },
  });
}

export function useSetMatchFinished() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { matchId: string; finished: boolean }) =>
      api.post<void>(`/admin/matches/${vars.matchId}/finished`, {
        finished: vars.finished,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

export function useUpdateMatchTeams() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      homeTeam: string;
      awayTeam: string;
    }) =>
      api.post<void>(`/admin/matches/${vars.matchId}/teams`, {
        homeTeam: vars.homeTeam,
        awayTeam: vars.awayTeam,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

// Cadastro manual de jogos de mata-mata pelo admin (times + fase + horário).
export function useCreateMatch() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      homeTeam: string;
      awayTeam: string;
      phase: string;
      kickoff: string;
    }) => api.post<MatchRecord>("/admin/matches", vars),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useUpdateMatchSchedule() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      homeTeam: string;
      awayTeam: string;
      phase: string;
      kickoff: string;
    }) =>
      api.post<MatchRecord>(`/admin/matches/${vars.matchId}/schedule`, {
        homeTeam: vars.homeTeam,
        awayTeam: vars.awayTeam,
        phase: vars.phase,
        kickoff: vars.kickoff,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

export function useSetMatchFixture() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { matchId: string; externalFixtureId: number | null }) =>
      api.post<MatchRecord>(`/admin/matches/${vars.matchId}/fixture`, {
        externalFixtureId: vars.externalFixtureId,
      }),
    onSuccess: (_updated, vars) => {
      qc.setQueriesData<AdminMatchRecord[]>(
        { queryKey: ["admin-matches"] },
        (old) =>
          old?.map((item) =>
            item.matchRecord.id === vars.matchId
              ? { ...item, externalFixtureId: vars.externalFixtureId }
              : item,
          ),
      );
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
    },
  });
}

export function useCheckFixture() {
  return useMutation({
    mutationFn: (externalFixtureId: number) =>
      api.post<FixtureCheckResult>("/admin/fixtures/check", {
        externalFixtureId,
      }),
  });
}

export function useDeleteMatch() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (matchId: string) =>
      api.post<void>(`/admin/matches/${matchId}/delete`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["matches"] });
      qc.invalidateQueries({ queryKey: ["admin-matches"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
    },
  });
}

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

export function useLeaderboard(poolId: string | null) {
  return useQuery({
    queryKey: ["leaderboard", poolId],
    queryFn: () =>
      api.get<LeaderboardEntry[]>(
        `/leaderboard?poolId=${encodeURIComponent(poolId ?? "")}`,
      ),
    enabled: !!poolId,
    // Revalida sozinho para refletir a pontuação ao vivo (provisória) durante os jogos.
    refetchInterval: 60_000,
  });
}

// ---- Ajustes manuais de pontos --------------------------------------------

export function usePoolAdjustments(poolId: string | null) {
  return useQuery({
    queryKey: ["pool-adjustments", poolId],
    queryFn: () =>
      api.get<PointAdjustment[]>(
        `/pools/${encodeURIComponent(poolId ?? "")}/adjustments`,
      ),
    enabled: !!poolId,
  });
}

export function useAddAdjustment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      poolId: string;
      userId: string;
      delta: number;
      reason: string;
    }) =>
      api.post<void>(`/pools/${vars.poolId}/adjustments`, {
        userId: vars.userId,
        delta: vars.delta,
        reason: vars.reason,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["pool-adjustments", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}

export function useRemoveAdjustment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { poolId: string; adjustmentId: string }) =>
      api.post<void>(`/pools/${vars.poolId}/adjustments/remove`, {
        adjustmentId: vars.adjustmentId,
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["pool-adjustments", vars.poolId] });
      qc.invalidateQueries({ queryKey: ["leaderboard", vars.poolId] });
    },
  });
}

// ---- Palpites do bolão ----------------------------------------------------

export function usePoolMemberPredictions(poolId: string | null) {
  return useQuery({
    queryKey: ["pool-member-predictions", poolId],
    queryFn: () =>
      api.get<MemberPredictions[]>(
        `/pools/${encodeURIComponent(poolId ?? "")}/member-predictions`,
      ),
    enabled: !!poolId,
  });
}

export function usePoolBreakdowns(poolId: string | null) {
  return useQuery({
    queryKey: ["pool-breakdowns", poolId],
    queryFn: () =>
      api.get<PredictionScoreBreakdown[]>(
        `/pools/${encodeURIComponent(poolId ?? "")}/breakdowns`,
      ),
    enabled: !!poolId,
    // Acompanha resultados recém-lançados durante os jogos.
    refetchInterval: 60_000,
  });
}

// ---- Admin: gestão de membros de bolões -----------------------------------

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

export function useAdminOverview() {
  return useQuery({
    queryKey: ["admin-overview"],
    queryFn: () => api.get<AdminOverview>("/admin/overview"),
  });
}

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

export function useAdminPredictions(filters: {
  matchId?: string;
  userId?: string;
  poolId?: string;
  missingOnly?: boolean;
}) {
  const params = new URLSearchParams();
  if (filters.matchId) params.set("matchId", filters.matchId);
  if (filters.userId) params.set("userId", filters.userId);
  if (filters.poolId) params.set("poolId", filters.poolId);
  if (filters.missingOnly) params.set("missingOnly", "true");

  return useQuery({
    queryKey: ["admin-predictions", filters],
    queryFn: () =>
      api.get<AdminPredictionRow[]>(
        `/admin/predictions${params.toString() ? `?${params.toString()}` : ""}`,
      ),
  });
}

export function useReopenPrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: {
      matchId: string;
      userId: string;
      reason: string;
      expiresAt: string;
    }) => api.post<PredictionReopenOverride>("/admin/predictions/reopen", vars),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-predictions"] }),
  });
}

export function useRevokePredictionReopen() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (overrideId: string) =>
      api.post<void>("/admin/predictions/reopen/revoke", { overrideId }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-predictions"] }),
  });
}

export function useRecalculateMatch() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (matchId: string) =>
      api.post<ScoringJob>("/admin/scoring/recalculate-match", { matchId }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useRecalculateAll() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.post<ScoringJob>("/admin/scoring/recalculate-all"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["leaderboard"] });
      qc.invalidateQueries({ queryKey: ["admin-overview"] });
    },
  });
}

export function useUserBreakdown(userId: string | null, poolId: string | null) {
  return useQuery({
    queryKey: ["admin-user-breakdown", userId, poolId],
    queryFn: () =>
      api.get<PredictionScoreBreakdown[]>(
        `/admin/scoring/users/${encodeURIComponent(userId ?? "")}/breakdown?poolId=${encodeURIComponent(poolId ?? "")}`,
      ),
    enabled: !!userId && !!poolId,
  });
}

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
