// @ts-nocheck
type AdminPanelProps = Record<string, any>;

export function AdminUsersPanel(props: AdminPanelProps) {
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
      {tab === "users" && (
        <div className="mt-6 grid gap-5 xl:grid-cols-[0.9fr_1.1fr]">
          <Card>
            <div className="space-y-3">
              {adminUsers.data?.map((item) => (
                <button
                  key={item.user.id}
                  type="button"
                  onClick={() => setSelectedUserId(item.user.id)}
                  className={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedUserId === item.user.id ? "border-mint-dark bg-mint/10" : "border-mint/15 bg-card/70"}`}
                >
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="font-semibold text-ink">{item.user.username}</p>
                      <p className="text-sm text-ink-muted">{item.user.email}</p>
                    </div>
                    <div className="text-right text-xs uppercase tracking-[0.14em] text-ink-muted">
                      <p>{item.poolCount} bolões</p>
                      <p>{item.user.blockedAt ? "bloqueado" : "ativo"}</p>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </Card>

          <Card>
            {selectedUser ? (
              <>
                <h2 className="text-2xl">{selectedUser.user.username}</h2>
                <p className="mt-1 text-sm text-ink-muted">{selectedUser.user.email}</p>
                {selectedUser.user.blockedAt && (
                  <p className="mt-2 text-sm font-semibold text-danger">
                    Bloqueado: {selectedUser.user.blockedReason ?? "sem motivo informado"}
                  </p>
                )}
                <div className="mt-5 flex flex-wrap gap-2">
                  {selectedUser.user.blockedAt ? (
                    <Button variant="outline" onClick={() => runAdminAction(() => unblockUser.mutateAsync(selectedUser.user.id))}>
                      Desbloquear
                    </Button>
                  ) : (
                    <Button variant="outline" onClick={() => {
                      const reason = window.prompt("Motivo do bloqueio do usuário:");
                      if (!reason) return;
                      void runAdminAction(() => blockUser.mutateAsync({ userId: selectedUser.user.id, reason }));
                    }}>
                      Bloquear usuário
                    </Button>
                  )}
                  <Button variant="outline" onClick={() => runAdminAction(() => invalidateSessions.mutateAsync(selectedUser.user.id))}>
                    Invalidar sessões
                  </Button>
                  <Button variant="outline" onClick={() => runAdminAction(() => triggerPasswordReset.mutateAsync(selectedUser.user.id))}>
                    Disparar reset de senha
                  </Button>
                </div>

                <div className="mt-6 rounded-2xl border border-sky/30 bg-sky/10 p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-card/80 p-2 text-mint-dark">
                      <Send className="h-4 w-4" />
                    </div>
                    <div>
                      <h3 className="text-lg">Enviar push</h3>
                      <p className="mt-1 text-sm text-ink-muted">
                        Destinatário: <strong>{selectedUser.user.username}</strong> ({selectedUser.user.email})
                      </p>
                    </div>
                  </div>
                  <div className="mt-4 grid gap-3 md:grid-cols-2">
                    <div>
                      <Label>Título</Label>
                      <Input
                        value={pushTitle}
                        maxLength={80}
                        onChange={(e) => setPushTitle(e.target.value)}
                        placeholder="Presumidos"
                      />
                    </div>
                    <div>
                      <Label>Link ao abrir</Label>
                      <Input
                        value={pushUrl}
                        maxLength={256}
                        onChange={(e) => setPushUrl(e.target.value)}
                        placeholder="/predictions"
                      />
                    </div>
                    <div className="md:col-span-2">
                      <Label>Mensagem</Label>
                      <TextArea
                        value={pushBody}
                        maxLength={240}
                        onChange={(e) => setPushBody(e.target.value)}
                        placeholder="Escreva a mensagem que vai aparecer na notificação."
                      />
                    </div>
                  </div>
                  <div className="mt-4 flex flex-wrap items-center gap-3">
                    <Button
                      onClick={handleSendPushToSelectedUser}
                      disabled={adminPushPending}
                    >
                      {sendPushToUser.isPending ? "Enviando..." : "Enviar para este usuário"}
                    </Button>
                    <Button
                      variant="outline"
                      className="border-yellow-dark/50 text-yellow-dark hover:border-yellow-dark"
                      onClick={handleSendPushBroadcast}
                      disabled={adminPushPending}
                    >
                      {sendPushBroadcast.isPending ? "Enviando em massa..." : "Enviar para todos"}
                    </Button>
                    <span className="text-xs text-ink-muted">
                      O envio em massa alcança usuários com notificações ativadas.
                    </span>
                  </div>
                  {pushSuccess && (
                    <p className="mt-3 flex items-center gap-2 text-sm font-semibold text-mint-dark">
                      <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                      {pushSuccess}
                    </p>
                  )}
                </div>

                <div className="mt-6">
                  <h3 className="text-lg">Pools em que está</h3>
                  <div className="mt-3 space-y-2">
                    {selectedUserPools.data?.map((pool) => (
                      <div key={pool.id} className="rounded-xl border border-mint/15 bg-card/75 px-4 py-3">
                        <p className="font-semibold text-ink">{pool.name}</p>
                        <p className="text-xs text-ink-muted">
                          Convite: {pool.inviteCode} · {pool.joinClosedAt ? "fechado" : "aberto"}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <p className="text-ink-muted">Selecione um usuário.</p>
            )}
          </Card>
        </div>
      )}

  </>;
}
