// @ts-nocheck
type AdminPanelProps = Record<string, any>;

export function AdminPoolsPanel(props: AdminPanelProps) {
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
      {tab === "pools" && (
        <div className="mt-6 grid gap-5 xl:grid-cols-[0.9fr_1.1fr]">
          <Card>
            <h2 className="text-xl">Bolões</h2>
            <div className="mt-4 space-y-3">
              {adminPools.data?.map((pool) => (
                <button
                  key={pool.id}
                  type="button"
                  onClick={() => setSelectedPoolId(pool.id)}
                  className={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedPoolId === pool.id ? "border-mint-dark bg-mint/10" : "border-mint/15 bg-card/70"}`}
                >
                  <p className="font-semibold text-ink">{pool.name}</p>
                  <p className="mt-1 text-sm text-ink-muted">
                    {pool.memberCount} membro(s) · código {pool.inviteCode}
                  </p>
                  <p className="mt-1 text-xs uppercase tracking-[0.14em] text-ink-muted">
                    {pool.joinClosedAt ? "entrada fechada" : "entrada aberta"}
                  </p>
                </button>
              ))}
            </div>
          </Card>

          <Card>
            <h2 className="text-xl">Membros do bolão</h2>
            <div className="mt-4 flex flex-col gap-3 sm:flex-row">
              <Select value={selectedPoolUserToAdd} onChange={(e) => setSelectedPoolUserToAdd(e.target.value)}>
                <option value="">Selecionar usuário para adicionar</option>
                {availablePoolUsers.map((item) => (
                  <option key={item.user.id} value={item.user.id}>
                    {item.user.username} · {item.user.email}
                  </option>
                ))}
              </Select>
              <Button
                onClick={() => {
                  if (!selectedPoolId || !selectedPoolUserToAdd) return;
                  void runAdminAction(() =>
                    addPoolMember.mutateAsync({ poolId: selectedPoolId, userId: selectedPoolUserToAdd }),
                  );
                  setSelectedPoolUserToAdd("");
                }}
              >
                Adicionar membro
              </Button>
            </div>
            <div className="mt-5 space-y-2">
              {selectedPoolMembers.data?.map((member) => (
                <div key={member.id} className="flex items-center justify-between gap-3 rounded-xl border border-mint/15 bg-card/75 px-4 py-3">
                  <div>
                    <p className="font-semibold text-ink">{member.username}</p>
                    <p className="text-sm text-ink-muted">{member.email}</p>
                  </div>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => runAdminAction(() => removePoolMember.mutateAsync({ poolId: selectedPoolId, userId: member.id }))}
                  >
                    Remover
                  </Button>
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}

  </>;
}
