// @ts-nocheck
import { useEffect, useMemo, useState } from "react";
import { withAdminReauth } from "@/lib/adminReauth";
import { formatKickoff } from "@/lib/utils";
import { formatSelectionLabel } from "@/lib/selections";
import { isoToBrasiliaInput, KNOCKOUT_PHASES } from "@/components/admin/fixtureValidation";
import { createAdminWorkspaceActions } from "@/components/admin/AdminWorkspaceActions";
import { emptyAdminMatchFilters, useAdminMatchFilters } from "@/hooks/useAdminMatchFilters";
import { useAdminPoolMembers } from "@/hooks/queries/admin/pools";
import { api } from "@/lib/api";
import type { AdminSettings } from "@/types";
import { MetricCard, TextArea, parseScore, scoreField, reportCategoryLabels, reportStatusLabels, reportStatusOptions } from "./AdminWorkspacePrimitives";
import { useAdminWorkspaceData } from "./useAdminWorkspaceData";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { CheckCircle2, Clock3, Eye, EyeOff, Flag, Lock, Send, TimerReset, Trophy, Users } from "lucide-react";
export function useAdminWorkspace({ navigate }: Record<string, any>) {
  const [tab, setTab] = useState<AdminTab>("overview");
  const [error, setError] = useState("");
  const [matchFilters, setMatchFilters] = useState(emptyAdminMatchFilters);
  const [predictionFilters, setPredictionFilters] = useState({
    matchId: "",
    userId: "",
    poolId: "",
    missingOnly: false,
  });
  const [selectedMatchId, setSelectedMatchId] = useState("");
  const [selectedUserId, setSelectedUserId] = useState("");
  const [selectedPoolId, setSelectedPoolId] = useState("");
  const [settingsDraft, setSettingsDraft] = useState<AdminSettings | null>(null);
  const {
    overview,
    reauth,
    adminUsers,
    adminPools,
    adminMatches,
    adminPredictions,
    selectedMatchAudit,
    selectedUserPools,
    breakdown,
    audit,
    poolReports,
    settings,
    adminEvents,
    allMatchesForKnockout,
    knockoutReleasedQuery,
    recalcAll,
    recalcMatch,
    setMatchResult,
    setMatchFinished,
    createMatch,
    setKnockoutReleased,
    updateMatchSchedule,
    deleteMatch,
    reopenPrediction,
    revokeReopen,
    blockUser,
    unblockUser,
    invalidateSessions,
    triggerPasswordReset,
    sendPushToUser,
    sendPushBroadcast,
    addPoolMember,
    removePoolMember,
    saveSettings,
    finishEvent,
    deleteEvent,
    setEventPoolCreation,
    publishEventVersion,
    updatePoolReportStatus,
  } = useAdminWorkspaceData({
    matchFilters, predictionFilters, selectedMatchId, selectedUserId, selectedPoolId,
  });
  useEffect(() => {
    if (!selectedUserId && adminUsers.data?.length) {
      const firstUserId = adminUsers.data[0]?.user?.id;
      if (firstUserId) setSelectedUserId(firstUserId);
    }
  }, [adminUsers.data, selectedUserId]);
  useEffect(() => {
    if (!selectedPoolId && adminPools.data?.length) {
      setSelectedPoolId(adminPools.data[0].id);
    }
  }, [adminPools.data, selectedPoolId]);
  useEffect(() => {
    if (settings.data) setSettingsDraft(settings.data);
  }, [settings.data]);

  const selectedMatch = useMemo(
    () =>
      adminMatches.data?.find(
        (item) => item.matchRecord && item.matchRecord.id === selectedMatchId,
      ) ?? null,
    [adminMatches.data, selectedMatchId],
  );
  const selectedPoolMembers = useAdminPoolMembers(selectedPoolId || null);
  const selectedUser =
    adminUsers.data?.find((item) => item.user?.id === selectedUserId) ?? null;
  const availablePoolUsers = useMemo(() => {
    const members = new Set((selectedPoolMembers.data ?? []).map((member) => member.id));
    return (adminUsers.data ?? []).filter((record) => {
      const userId = record.user?.id;
      return !!userId && !members.has(userId);
    });
  }, [adminUsers.data, selectedPoolMembers.data]);

  const [resultHome, setResultHome] = useState("");
  const [resultAway, setResultAway] = useState("");
  const [penHome, setPenHome] = useState("");
  const [penAway, setPenAway] = useState("");
  const [overrideExpiry, setOverrideExpiry] = useState("");
  const [overrideReason, setOverrideReason] = useState("");
  const [selectedPoolUserToAdd, setSelectedPoolUserToAdd] = useState("");
  const [pushTitle, setPushTitle] = useState("Presumidos");
  const [pushBody, setPushBody] = useState("");
  const [pushUrl, setPushUrl] = useState("/");
  const [pushSuccess, setPushSuccess] = useState("");

  // Cadastro manual de jogo de mata-mata.
  const [newMatchHome, setNewMatchHome] = useState("");
  const [newMatchAway, setNewMatchAway] = useState("");
  const [newMatchPhase, setNewMatchPhase] = useState(KNOCKOUT_PHASES[0]);
  const [newMatchDate, setNewMatchDate] = useState("");
  const [newMatchTime, setNewMatchTime] = useState("");
  const [createMatchError, setCreateMatchError] = useState("");
  const [createMatchSuccess, setCreateMatchSuccess] = useState("");
  const [showCreateMatchForm, setShowCreateMatchForm] = useState(false);
  const [knockoutToggleMsg, setKnockoutToggleMsg] = useState("");

  const knockoutReleased = knockoutReleasedQuery.data?.released ?? false;
  const {
    knockoutMatches,
    phaseOptions,
    groupOptions,
    visibleMatches,
    hasActiveFilters: hasActiveMatchFilters,
  } = useAdminMatchFilters(
    adminMatches.data,
    allMatchesForKnockout.data,
    matchFilters,
  );

  // Edição de confronto/fase/horário do jogo selecionado.
  const [editHome, setEditHome] = useState("");
  const [editAway, setEditAway] = useState("");
  const [editPhase, setEditPhase] = useState(KNOCKOUT_PHASES[0]);
  const [editMatchDate, setEditMatchDate] = useState("");
  const [editMatchTime, setEditMatchTime] = useState("");
  const [scheduleError, setScheduleError] = useState("");

  useEffect(() => {
    if (!selectedMatch) return;
    setResultHome(scoreField(selectedMatch.matchRecord.homeScore));
    setResultAway(scoreField(selectedMatch.matchRecord.awayScore));
    setPenHome(scoreField(selectedMatch.matchRecord.penaltyHomeScore));
    setPenAway(scoreField(selectedMatch.matchRecord.penaltyAwayScore));
    setEditHome(selectedMatch.matchRecord.homeTeam);
    setEditAway(selectedMatch.matchRecord.awayTeam);
    setEditPhase(selectedMatch.matchRecord.phase ?? KNOCKOUT_PHASES[0]);
    const kickoffInput = isoToBrasiliaInput(selectedMatch.matchRecord.kickoff);
    setEditMatchDate(kickoffInput.date);
    setEditMatchTime(kickoffInput.time);
    setScheduleError("");
  }, [selectedMatch]);

  useEffect(() => {
    if (!visibleMatches.length) {
      if (selectedMatchId) setSelectedMatchId("");
      return;
    }
    if (!visibleMatches.some((item) => item.matchRecord.id === selectedMatchId)) {
      setSelectedMatchId(visibleMatches[0].matchRecord.id);
    }
  }, [selectedMatchId, visibleMatches]);

  // As confirmações de "criado"/"liberado" somem sozinhas depois de alguns segundos.
  useEffect(() => {
    if (!createMatchSuccess) return;
    const timer = window.setTimeout(() => setCreateMatchSuccess(""), 5000);
    return () => window.clearTimeout(timer);
  }, [createMatchSuccess]);

  useEffect(() => {
    if (!knockoutToggleMsg) return;
    const timer = window.setTimeout(() => setKnockoutToggleMsg(""), 5000);
    return () => window.clearTimeout(timer);
  }, [knockoutToggleMsg]);

  useEffect(() => {
    if (!pushSuccess) return;
    const timer = window.setTimeout(() => setPushSuccess(""), 7000);
    return () => window.clearTimeout(timer);
  }, [pushSuccess]);

  const runAdminAction = async <T,>(action: () => Promise<T>) => {
    setError("");
    try {
      return await withAdminReauth(action, (password) => reauth.mutateAsync(password));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Falha ao executar ação admin.";
      setError(message);
      throw err;
    }
  };

  const downloadManifest = async (eventId: string, slug: string) => {
    setError("");
    try {
      const download = await api.download(`/admin/events/${eventId}/manifest`);
      const url = URL.createObjectURL(download.blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = download.filename ?? `${slug}.json`;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Não foi possível exportar o manifesto.");
    }
  };

  const downloadPackage = async (eventId: string, slug: string) => {
    setError("");
    try {
      const download = await api.download(`/admin/events/${eventId}/package`);
      const url = URL.createObjectURL(download.blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = download.filename ?? `${slug}.zip`;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Não foi possível exportar o pacote.");
    }
  };


  const actions = createAdminWorkspaceActions({
    selectedUser, setPushSuccess, setError, runAdminAction, sendPushToUser, sendPushBroadcast,
    selectedMatch, resultHome, resultAway, penHome, penAway, setMatchResult,
    setCreateMatchError, setCreateMatchSuccess, newMatchHome, newMatchAway, newMatchDate,
    newMatchTime, newMatchPhase, createMatch, setNewMatchHome, setNewMatchAway, setNewMatchDate,
    setNewMatchTime, setShowCreateMatchForm, knockoutReleased, setKnockoutToggleMsg,
    setKnockoutReleased, finishEvent, deleteEvent, setScheduleError, editHome, editAway,
    editMatchDate, editMatchTime, updateMatchSchedule, editPhase, deleteMatch, selectedMatchId,
    setSelectedMatchId, setMatchFinished, overrideExpiry, overrideReason, reopenPrediction,
    parseScore,
  });
  const {
    adminPushPending, handleSendPushToSelectedUser, handleSendPushBroadcast, handleSaveResult,
    handleCreateMatch, handleToggleKnockout, handleFinishEvent, handleDeleteEvent,
    handleUpdateSchedule, handleDeleteMatch, handleToggleFinished, handleReopenPrediction,
  } = actions;

  const selectedMatchRows =
    adminPredictions.data?.filter((row) => row.matchId === (predictionFilters.matchId || selectedMatchId)) ?? [];

  const panelContext = {
    
    tab, setTab, overview, adminUsers, adminPools, adminMatches, selectedMatchAudit,
    selectedUserPools, breakdown, audit, poolReports, settingsDraft, setSettingsDraft, knockoutReleased,
    recalcAll, recalcMatch, setMatchResult, setMatchFinished, blockUser, unblockUser,
    invalidateSessions, triggerPasswordReset, sendPushToUser, sendPushBroadcast,
    addPoolMember, removePoolMember, saveSettings, updatePoolReportStatus,
    selectedMatch, selectedUser, selectedPoolId, setSelectedPoolId, selectedPoolMembers, availablePoolUsers,
    selectedUserId, setSelectedUserId, selectedPoolUserToAdd, setSelectedPoolUserToAdd,
    resultHome, setResultHome, resultAway, setResultAway, penHome, setPenHome, penAway, setPenAway,
    overrideExpiry, setOverrideExpiry, overrideReason, setOverrideReason,
    pushTitle, setPushTitle, pushBody, setPushBody, pushUrl, setPushUrl, pushSuccess,
    adminPushPending, runAdminAction, handleSaveResult, handleSendPushToSelectedUser,
    handleSendPushBroadcast, handleReopenPrediction, formatKickoff, formatSelectionLabel,
    MetricCard, TextArea, Button, Card, Label, Select, Input, CheckCircle2, Clock3,
    Flag, Lock, Send, TimerReset, Trophy, Users, Eye, EyeOff, reportCategoryLabels,
    reportStatusLabels, reportStatusOptions,
    
    
  };
  return {
    panelContext, navigate, tab, setTab, error, runAdminAction, recalcAll, overview,
    adminEvents, downloadManifest, downloadPackage, publishEventVersion, setEventPoolCreation,
    handleFinishEvent, finishEvent, handleDeleteEvent, deleteEvent, knockoutMatches,
    knockoutReleased, setKnockoutReleased, knockoutReleasedQuery, handleToggleKnockout,
    matchFilters, setMatchFilters, phaseOptions, groupOptions, visibleMatches,
    hasActiveMatchFilters, selectedMatchId, setSelectedMatchId, selectedMatch,
    selectedMatchAudit, resultHome, setResultHome, resultAway, setResultAway, penHome,
    setPenHome, penAway, setPenAway, newMatchHome, setNewMatchHome, newMatchAway,
    setNewMatchAway, newMatchPhase, setNewMatchPhase, newMatchDate, setNewMatchDate,
    newMatchTime, setNewMatchTime, createMatchError, createMatchSuccess, showCreateMatchForm,
    setShowCreateMatchForm, setCreateMatchError, knockoutToggleMsg, editHome, setEditHome,
    editAway, setEditAway, editPhase, setEditPhase, editMatchDate, setEditMatchDate,
    editMatchTime, setEditMatchTime, scheduleError, handleCreateMatch, createMatch,
    handleToggleFinished, handleSaveResult, recalcMatch, handleUpdateSchedule,
    updateMatchSchedule, handleDeleteMatch, deleteMatch, predictionFilters, setPredictionFilters,
    adminMatches, adminUsers, adminPools, adminPredictions, selectedMatchRows, overrideExpiry,
    setOverrideExpiry, overrideReason, setOverrideReason, handleReopenPrediction, revokeReopen,
  };
}
