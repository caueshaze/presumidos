// @ts-nocheck
type AdminPanelProps = Record<string, any>;

export function AdminScoringPanel(props: AdminPanelProps) {
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
      {tab === "scoring" && (
        <div className="mt-6 grid gap-5 xl:grid-cols-[0.9fr_1.1fr]">
          <Card>
            <div className="grid gap-3">
              <div>
                <Label>Usuário</Label>
                <Select value={selectedUserId} onChange={(e) => setSelectedUserId(e.target.value)}>
                  {adminUsers.data?.map((item) => (
                    <option key={item.user.id} value={item.user.id}>
                      {item.user.username}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Bolão</Label>
                <Select value={selectedPoolId} onChange={(e) => setSelectedPoolId(e.target.value)}>
                  {selectedUserPools.data?.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  )) ?? adminPools.data?.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </Select>
              </div>
            </div>

            <div className="mt-5 flex flex-wrap gap-2">
              <Button variant="outline" onClick={() => selectedMatch && runAdminAction(() => recalcMatch.mutateAsync(selectedMatch.matchRecord.id))}>
                Recalcular jogo selecionado
              </Button>
              <Button onClick={() => runAdminAction(() => recalcAll.mutateAsync())}>Recalcular tudo</Button>
            </div>

            {selectedUser && (
              <div className="mt-5 rounded-2xl border border-mint/15 bg-card/75 px-4 py-4">
                <p className="font-semibold text-ink">{selectedUser.user.username}</p>
                <p className="text-sm text-ink-muted">{selectedUser.user.email}</p>
                <p className="mt-1 text-xs uppercase tracking-[0.14em] text-ink-muted">
                  {selectedUser.poolCount} bolão(ões)
                </p>
              </div>
            )}
          </Card>

          <Card>
            <h2 className="text-xl">Breakdown por usuário</h2>
            <div className="mt-4 overflow-x-auto">
              <table className="min-w-full text-sm">
                <thead className="text-left text-ink-muted">
                  <tr>
                    <th className="pb-2 pr-3">Jogo</th>
                    <th className="pb-2 pr-3">Placar</th>
                    <th className="pb-2 pr-3">Resultado</th>
                    <th className="pb-2 pr-3">Gols</th>
                    <th className="pb-2 pr-3">Classificado</th>
                    <th className="pb-2 pr-3">Pênaltis</th>
                    <th className="pb-2 pr-3">Total</th>
                    <th className="pb-2 pr-3">Elegível</th>
                  </tr>
                </thead>
                <tbody>
                  {breakdown.data?.map((row) => (
                    <tr key={`${row.poolId}-${row.matchId}`} className="border-t border-mint/10">
                      <td className="py-3 pr-3">{row.homeTeam} x {row.awayTeam}</td>
                      <td className="py-3 pr-3">{row.exactScorePoints}</td>
                      <td className="py-3 pr-3">{row.outcomePoints}</td>
                      <td className="py-3 pr-3">{row.goalBonusPoints}</td>
                      <td className="py-3 pr-3">{row.qualifierPoints}</td>
                      <td className="py-3 pr-3">{row.penaltiesPoints}</td>
                      <td className="py-3 pr-3 font-semibold text-ink">{row.totalPoints}</td>
                      <td className="py-3 pr-3">{row.eligible ? "Sim" : row.eligibilityReason}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        </div>
      )}

  </>;
}
