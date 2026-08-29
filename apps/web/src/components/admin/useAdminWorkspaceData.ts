// @ts-nocheck
import {
  useAddPoolMember, useAdminAudit, useAdminEvents, useAdminMatches, useAdminMatchAudit,
  useAdminOverview, useAdminPoolMembers, useAdminPoolReports, useAdminPools,
  useAdminPredictions, useAdminSendPushBroadcast, useAdminSendPushToUser, useAdminSettings,
  useAdminUsers, useBlockUser, useCreateMatch, useDeleteMatch, useAdminDeleteEvent,
  useFinishEvent, useInvalidateUserSessions, useKnockoutReleased, useReauth,
  useRecalculateAll, useRecalculateMatch, useRemovePoolMember, useReopenPrediction,
  useRevokePredictionReopen, useSaveAdminSettings, useSetKnockoutReleased,
  useSetMatchFinished, useSetMatchResult, useTriggerUserPasswordReset, useUnblockUser,
  useUpdateMatchSchedule, useUpdatePoolReportStatus, useSetEventPoolCreation,
  usePublishEventVersion, useUserBreakdown, useUserPools,
} from "@/hooks/queries";
import { brasiliaDateToIsoDateFilter } from "@/components/admin/fixtureValidation";

export function useAdminWorkspaceData({
  matchFilters, predictionFilters, selectedMatchId, selectedUserId, selectedPoolId,
}: Record<string, any>) {
  const overview = useAdminOverview();
  const reauth = useReauth();
  const adminUsers = useAdminUsers();
  const adminPools = useAdminPools();
  const adminMatches = useAdminMatches({
    phase: matchFilters.phase || undefined,
    groupName: matchFilters.groupName || undefined,
    date: matchFilters.date ? (brasiliaDateToIsoDateFilter(matchFilters.date) ?? undefined) : undefined,
    status: matchFilters.status || undefined,
    origin: matchFilters.origin || undefined,
  });
  const adminPredictions = useAdminPredictions({
    matchId: predictionFilters.matchId || undefined,
    userId: predictionFilters.userId || undefined,
    poolId: predictionFilters.poolId || undefined,
    missingOnly: predictionFilters.missingOnly,
  });
  const selectedMatchAudit = useAdminMatchAudit(selectedMatchId || null);
  const selectedUserPools = useUserPools(selectedUserId || null);
  const breakdown = useUserBreakdown(selectedUserId || null, selectedPoolId || null);
  const audit = useAdminAudit({});
  const poolReports = useAdminPoolReports();
  const settings = useAdminSettings();
  const adminEvents = useAdminEvents();
  // Lista sem filtros, dedicada ao painel do mata-mata: o contador/chaveamento
  // não devem mudar quando o admin filtra a lista de jogos logo abaixo.
  const allMatchesForKnockout = useAdminMatches({});
  const knockoutReleasedQuery = useKnockoutReleased();

  const recalcAll = useRecalculateAll();
  const recalcMatch = useRecalculateMatch();
  const setMatchResult = useSetMatchResult();
  const setMatchFinished = useSetMatchFinished();
  const createMatch = useCreateMatch();
  const setKnockoutReleased = useSetKnockoutReleased();
  const updateMatchSchedule = useUpdateMatchSchedule();
  const deleteMatch = useDeleteMatch();
  const reopenPrediction = useReopenPrediction();
  const revokeReopen = useRevokePredictionReopen();
  const blockUser = useBlockUser();
  const unblockUser = useUnblockUser();
  const invalidateSessions = useInvalidateUserSessions();
  const triggerPasswordReset = useTriggerUserPasswordReset();
  const sendPushToUser = useAdminSendPushToUser();
  const sendPushBroadcast = useAdminSendPushBroadcast();
  const addPoolMember = useAddPoolMember();
  const removePoolMember = useRemovePoolMember();
  const saveSettings = useSaveAdminSettings();
  const finishEvent = useFinishEvent();
  const deleteEvent = useAdminDeleteEvent();
  const setEventPoolCreation = useSetEventPoolCreation();
  const publishEventVersion = usePublishEventVersion();
  const updatePoolReportStatus = useUpdatePoolReportStatus();

  return {
    overview, reauth, adminUsers, adminPools, adminMatches, adminPredictions, selectedMatchAudit, selectedUserPools, breakdown, audit, poolReports, settings, adminEvents, allMatchesForKnockout, knockoutReleasedQuery, recalcAll, recalcMatch, setMatchResult, setMatchFinished, createMatch, setKnockoutReleased, updateMatchSchedule, deleteMatch, reopenPrediction, revokeReopen, blockUser, unblockUser, invalidateSessions, triggerPasswordReset, sendPushToUser, sendPushBroadcast, addPoolMember, removePoolMember, saveSettings, finishEvent, deleteEvent, setEventPoolCreation, publishEventVersion, updatePoolReportStatus,
  };
}
