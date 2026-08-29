// @ts-nocheck
type AdminPanelProps = Record<string, any>;

export function AdminReportsPanel(props: AdminPanelProps) {
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
      {tab === "reports" && (
        <Card className="mt-6">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className="text-2xl">Denúncias de bolões</h2>
              <p className="mt-1 text-sm text-ink-muted">Revise relatos enviados pelos participantes e atualize o andamento de cada caso.</p>
            </div>
            <div className="rounded-pill bg-yellow/25 px-3 py-1 text-sm font-semibold">{poolReports.data?.filter((report) => report.status === "open").length ?? 0} aberta(s)</div>
          </div>
          {poolReports.isLoading ? <p className="mt-5 text-sm text-ink-muted">Carregando denúncias...</p> : poolReports.data?.length ? <div className="mt-5 space-y-3">{poolReports.data.map((report) => <div key={report.id} className="rounded-2xl border border-mint/15 bg-card/75 p-4"><div className="flex flex-wrap items-start justify-between gap-3"><div className="flex items-start gap-3"><div className="rounded-xl bg-yellow/20 p-2 text-yellow-dark"><Flag className="h-5 w-5" /></div><div><p className="font-semibold text-ink">{reportCategoryLabels[report.category]}</p><p className="mt-1 text-sm text-ink-muted">{report.poolName} · código {report.inviteCode}</p><p className="mt-1 text-xs text-ink-muted">Por {report.reporterUsername ?? "usuário removido"} · {formatKickoff(report.createdAt)}</p></div></div><Select className="w-auto min-w-36" value={report.status} aria-label={`Status da denúncia de ${report.poolName}`} onChange={(event) => void runAdminAction(() => updatePoolReportStatus.mutateAsync({ reportId: report.id, status: event.target.value as PoolReportStatus }))}>{reportStatusOptions.map((status) => <option key={status} value={status}>{reportStatusLabels[status]}</option>)}</Select></div>{report.details && <p className="mt-4 whitespace-pre-wrap rounded-xl bg-bg/45 px-3 py-3 text-sm text-ink-muted">{report.details}</p>}</div>)}</div> : <p className="mt-5 rounded-2xl border border-mint/15 bg-bg/35 px-4 py-4 text-sm text-ink-muted">Nenhuma denúncia registrada.</p>}
        </Card>
      )}

  </>;
}
