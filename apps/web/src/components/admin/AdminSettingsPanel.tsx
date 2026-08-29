// @ts-nocheck
type AdminPanelProps = Record<string, any>;

export function AdminSettingsPanel(props: AdminPanelProps) {
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
      {tab === "settings" && settingsDraft && (
        <Card className="mt-6">
          <h2 className="text-2xl">Configurações operacionais</h2>
          <div className="mt-5 grid gap-4 md:grid-cols-2">
            <div className="flex items-center justify-between gap-2 rounded-md border border-mint/20 bg-card/60 px-3 py-2 text-sm">
              <span className="flex items-center gap-2 font-semibold text-ink">
                {knockoutReleased ? (
                  <Eye className="h-4 w-4 text-mint-dark" />
                ) : (
                  <EyeOff className="h-4 w-4 text-yellow-dark" />
                )}
                Mata-mata {knockoutReleased ? "liberado" : "oculto"}
              </span>
              <button
                type="button"
                onClick={() => setTab("matches")}
                className="font-semibold text-mint-dark underline-offset-4 hover:underline"
              >
                Gerenciar em Jogos
              </button>
            </div>
            <div>
              <Label>Fechar palpites antes do jogo (min)</Label>
              <Input
                value={String(settingsDraft.predictionLockMinutes)}
                onChange={(e) => setSettingsDraft((v) => (v ? { ...v, predictionLockMinutes: Number(e.target.value) || 0 } : v))}
              />
            </div>
            <label className="flex items-start gap-3 rounded-2xl border border-yellow-dark/35 bg-yellow/15 px-4 py-3 text-sm text-ink md:col-span-2">
              <input
                type="checkbox"
                checked={settingsDraft.finalThemeEnabled}
                onChange={(e) =>
                  setSettingsDraft((v) => (v ? { ...v, finalThemeEnabled: e.target.checked } : v))
                }
                className="mt-0.5"
              />
              <span><span className="block font-heading text-base font-semibold">Ativar tema do bolão em destaque</span><span className="mt-1 block text-ink-muted">Aplica uma edição visual neutra quando houver um bolão em destaque configurado.</span></span>
            </label>
            <label className="flex items-start gap-3 rounded-2xl border border-mint-dark/25 bg-mint/10 px-4 py-3 text-sm text-ink md:col-span-2">
              <input
                type="checkbox"
                checked={settingsDraft.closingScreenEnabled}
                onChange={(e) =>
                  setSettingsDraft((v) => (v ? { ...v, closingScreenEnabled: e.target.checked } : v))
                }
                className="mt-0.5"
              />
              <span><span className="block font-heading text-base font-semibold">Destacar encerramento da edição</span><span className="mt-1 block text-ink-muted">Usa o bolão em destaque como referência visual, independente do tipo de evento.</span></span>
            </label>
            <div className="md:col-span-2"><Label htmlFor="featured-pool-id">ID do bolão em destaque</Label><Input id="featured-pool-id" value={settingsDraft.featuredPoolId ?? ""} onChange={(e) => setSettingsDraft((v) => v ? { ...v, featuredPoolId: e.target.value || null } : v)} placeholder="Opcional — um bolão explicitamente divulgado pelo admin" /><p className="mt-1 text-xs text-ink-muted">O destaque mostra contexto do bolão mesmo para quem ainda não participa; regras de entrada continuam valendo.</p></div>
            <label className="flex items-center gap-2 text-sm font-semibold text-ink md:col-span-2">
              <input
                type="checkbox"
                checked={settingsDraft.globalBannerEnabled}
                onChange={(e) => setSettingsDraft((v) => (v ? { ...v, globalBannerEnabled: e.target.checked } : v))}
              />
              Exibir mensagem global para usuários
            </label>
            <div className="md:col-span-2">
              <Label>Mensagem global</Label>
              <TextArea
                value={settingsDraft.globalBannerText}
                onChange={(e) => setSettingsDraft((v) => (v ? { ...v, globalBannerText: e.target.value } : v))}
                placeholder="Ex.: sincronização externa pausada durante manutenção"
              />
            </div>
          </div>
          <div className="mt-5">
            <Button onClick={() => runAdminAction(() => saveSettings.mutateAsync(settingsDraft))}>
              Salvar configurações
            </Button>
          </div>
        </Card>
      )}
  </>;
}
