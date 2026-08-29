// @ts-nocheck
type AdminPanelProps = Record<string, any>;

export function AdminAuditPanel(props: AdminPanelProps) {
  const {
    tab, setTab, overview, adminUsers, adminPools, adminMatches, selectedMatchAudit,
    selectedUserPools, breakdown, audit, poolReports, settingsDraft, knockoutReleased,
    recalcAll, recalcMatch, setMatchResult, setMatchFinished, blockUser, unblockUser,
    invalidateSessions, triggerPasswordReset, sendPushToUser, sendPushBroadcast,
    addPoolMember, removePoolMember, saveSettings, updatePoolReportStatus,
    selectedMatch, selectedUser, selectedPoolId, selectedPoolMembers, availablePoolUsers,
    selectedUserId, selectedPoolUserToAdd, setSelectedPoolUserToAdd,
    resultHome, setResultHome, resultAway, setResultAway, penHome, setPenHome, penAway, setPenAway,
    overrideExpiry, setOverrideExpiry, overrideReason, setOverrideReason,
    pushTitle, setPushTitle, pushBody, setPushBody, pushUrl, setPushUrl, pushSuccess,
    adminPushPending, runAdminAction, handleSaveResult, handleSendPushToSelectedUser,
    handleSendPushBroadcast, handleReopenPrediction, formatKickoff, formatSelectionLabel,
    MetricCard, TextArea, Button, Card, Label, Select, Input, CheckCircle2, Clock3,
    Flag, Lock, Send, TimerReset, Trophy, Users, Eye, EyeOff, reportCategoryLabels,
    reportStatusLabels, reportStatusOptions,
  } = props;
  return <>
      {tab === "audit" && (
        <Card className="mt-6">
          <h2 className="text-2xl">Auditoria</h2>
          <div className="mt-4 space-y-3">
            {audit.data?.map((entry) => (
              <div key={entry.id} className="rounded-2xl border border-mint/15 bg-card/75 px-4 py-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <p className="font-semibold text-ink">
                    {entry.action} · {entry.actorUsername ?? "Sistema"}
                  </p>
                  <p className="text-xs uppercase tracking-[0.14em] text-ink-muted">
                    {entry.targetType} · {entry.targetId ?? "sem alvo"} · {formatKickoff(entry.createdAt)}
                  </p>
                </div>
                <pre className="mt-3 overflow-x-auto whitespace-pre-wrap break-words text-xs text-ink-muted">
                  {entry.detailsJson}
                </pre>
              </div>
            ))}
          </div>
        </Card>
      )}

  </>;
}
