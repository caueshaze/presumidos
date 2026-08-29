// @ts-nocheck
import { CheckCircle2, Clock3, Eye, EyeOff, Flag, Lock, Send, TimerReset, Trophy, Users } from "lucide-react";
import { PageShell } from "@/components/PageShell";
import { formatKickoff } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";
import { AdminEventsPanel } from "@/components/admin/AdminEventsPanel";
import { AdminMatchesPanel } from "@/components/admin/AdminMatchesPanel";
import { AdminPredictionsPanel } from "@/components/admin/AdminPredictionsPanel";
import { AdminAuditPanel } from "@/components/admin/AdminAuditPanel";
import { AdminPoolsPanel } from "@/components/admin/AdminPoolsPanel";
import { AdminReportsPanel } from "@/components/admin/AdminReportsPanel";
import { AdminScoringPanel } from "@/components/admin/AdminScoringPanel";
import { AdminSettingsPanel } from "@/components/admin/AdminSettingsPanel";
import { AdminUsersPanel } from "@/components/admin/AdminUsersPanel";
import { MetricCard, tabs } from "./AdminWorkspacePrimitives";

export function AdminWorkspaceView({ workspace }: { workspace: Record<string, any> }) {
  const {
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
    setOverrideExpiry, overrideReason, setOverrideReason, handleReopenPrediction, revokeReopen
  } = workspace;
  return (
    <PageShell className="max-w-[1280px]">
      <div className="rounded-[28px] border border-mint/20 bg-[radial-gradient(circle_at_top_left,rgba(130,207,255,0.22),transparent_35%),linear-gradient(180deg,rgba(255,255,255,0.96),rgba(248,255,252,0.92))] p-5 shadow-card dark:border-mint/15 dark:bg-[radial-gradient(circle_at_top_left,rgba(79,206,159,0.18),transparent_34%),radial-gradient(circle_at_86%_16%,rgba(95,176,230,0.14),transparent_30%),linear-gradient(180deg,rgba(22,33,30,0.96),rgba(12,20,18,0.92))] sm:p-6">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <p className="text-sm font-semibold uppercase tracking-[0.22em] text-mint-dark">
              Console Admin
            </p>
            <h1 className="mt-2 font-heading text-3xl text-ink sm:text-4xl">
              Operação profissional do bolão
            </h1>
            <p className="mt-2 max-w-3xl text-sm text-ink-muted">
              Jogos, sincronização externa, palpites, pontuação, usuários, bolões, auditoria e
              configuração em uma única superfície.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" onClick={() => runAdminAction(() => recalcAll.mutateAsync())}>
              <Trophy className="h-4 w-4" />
              Recalcular tudo
            </Button>
          </div>
        </div>

        <div className="mt-5 flex flex-wrap gap-2">
          {tabs.map((item) => (
            <Button
              key={item.id}
              variant={tab === item.id ? "primary" : "outline"}
              size="sm"
              onClick={() => setTab(item.id)}
            >
              {item.label}
            </Button>
          ))}
        </div>
      </div>

      {error && (
        <div className="mt-5">
          <ErrorBanner>{error}</ErrorBanner>
        </div>
      )}

      {tab === "overview" && (
        <div className="mt-6 grid gap-4 lg:grid-cols-4">
          <MetricCard icon={<Clock3 className="h-5 w-5" />} label="Agendados" value={overview.data?.scheduledMatches ?? "-"} />
          <MetricCard icon={<Trophy className="h-5 w-5" />} label="Finalizados" value={overview.data?.finalizedMatches ?? "-"} />
          <MetricCard icon={<Users className="h-5 w-5" />} label="Usuários" value={overview.data?.userCount ?? "-"} />
          <MetricCard icon={<Lock className="h-5 w-5" />} label="Bloqueados" value={overview.data?.blockedUserCount ?? "-"} tone="danger" />
          <MetricCard icon={<TimerReset className="h-5 w-5" />} label="Sem Palpite Próximo" value={overview.data?.usersWithoutPredictionsSoon ?? "-"} />

          <Card className="lg:col-span-2">
            <h2 className="text-xl">Feed recente de jogos</h2>
            <div className="mt-3 space-y-3">
              {overview.data?.activityFeed.map((item) => (
                <div key={item.id} className="rounded-lg border border-mint/15 bg-card/75 px-4 py-3">
                  <p className="font-semibold text-ink">{item.label}</p>
                  <p className="mt-1 text-xs uppercase tracking-[0.14em] text-ink-muted">
                    {item.action} · {formatKickoff(item.at)}
                  </p>
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}

      {tab === "events" && (
        <AdminEventsPanel
          events={adminEvents.data}
          isLoading={adminEvents.isLoading}
          isError={adminEvents.isError}
          onApplied={() => void adminEvents.refetch()}
          onDownloadManifest={downloadManifest}
          onDownloadPackage={downloadPackage}
          onOpen={(eventId) => navigate(`/events/${eventId}`)}
          onPublish={(eventId, versionId) => void runAdminAction(() => publishEventVersion.mutateAsync({ eventId, versionId }))}
          publishPending={publishEventVersion.isPending}
          onSetPoolCreation={(eventId, enabled) => void runAdminAction(() => setEventPoolCreation.mutateAsync({ eventId, enabled }))}
          poolCreationPending={setEventPoolCreation.isPending}
          onFinish={handleFinishEvent}
          finishPending={finishEvent.isPending}
          onDelete={handleDeleteEvent}
          deletePending={deleteEvent.isPending}
        />
      )}

      {tab === "matches" && (
        <AdminMatchesPanel
          knockoutMatches={knockoutMatches}
          knockoutReleased={knockoutReleased}
          setKnockoutReleasedPending={setKnockoutReleased.isPending}
          knockoutReleasedLoading={knockoutReleasedQuery.isLoading}
          onToggleKnockout={handleToggleKnockout}
          matchFilters={matchFilters}
          setMatchFilters={setMatchFilters}
          phaseOptions={phaseOptions}
          groupOptions={groupOptions}
          visibleMatches={visibleMatches}
          hasActiveMatchFilters={hasActiveMatchFilters}
          selectedMatchId={selectedMatchId}
          setSelectedMatchId={setSelectedMatchId}
          selectedMatch={selectedMatch}
          auditEntries={selectedMatchAudit.data}
          resultHome={resultHome}
          setResultHome={setResultHome}
          resultAway={resultAway}
          setResultAway={setResultAway}
          penHome={penHome}
          setPenHome={setPenHome}
          penAway={penAway}
          setPenAway={setPenAway}
          newMatchHome={newMatchHome}
          setNewMatchHome={setNewMatchHome}
          newMatchAway={newMatchAway}
          setNewMatchAway={setNewMatchAway}
          newMatchPhase={newMatchPhase}
          setNewMatchPhase={setNewMatchPhase}
          newMatchDate={newMatchDate}
          setNewMatchDate={setNewMatchDate}
          newMatchTime={newMatchTime}
          setNewMatchTime={setNewMatchTime}
          createMatchError={createMatchError}
          createMatchSuccess={createMatchSuccess}
          showCreateMatchForm={showCreateMatchForm}
          setShowCreateMatchForm={setShowCreateMatchForm}
          setCreateMatchError={setCreateMatchError}
          knockoutToggleMsg={knockoutToggleMsg}
          editHome={editHome}
          setEditHome={setEditHome}
          editAway={editAway}
          setEditAway={setEditAway}
          editPhase={editPhase}
          setEditPhase={setEditPhase}
          editMatchDate={editMatchDate}
          setEditMatchDate={setEditMatchDate}
          editMatchTime={editMatchTime}
          setEditMatchTime={setEditMatchTime}
          scheduleError={scheduleError}
          onCreateMatch={handleCreateMatch}
          createMatchPending={createMatch.isPending}
          onToggleFinished={handleToggleFinished}
          onSaveResult={handleSaveResult}
          onRecalculate={() => selectedMatch ? runAdminAction(() => recalcMatch.mutateAsync(selectedMatch.matchRecord.id)) : undefined}
          onUpdateSchedule={handleUpdateSchedule}
          updateSchedulePending={updateMatchSchedule.isPending}
          onDeleteMatch={handleDeleteMatch}
          deleteMatchPending={deleteMatch.isPending}
        />
      )}
      {tab === "predictions" && (
        <AdminPredictionsPanel
          filters={predictionFilters}
          setFilters={setPredictionFilters}
          matches={adminMatches.data}
          users={adminUsers.data}
          pools={adminPools.data}
          predictions={adminPredictions.data}
          selectedMatchRows={selectedMatchRows}
          overrideExpiry={overrideExpiry}
          setOverrideExpiry={setOverrideExpiry}
          overrideReason={overrideReason}
          setOverrideReason={setOverrideReason}
          onReopenPrediction={handleReopenPrediction}
          onRevokeReopen={(overrideId) => runAdminAction(() => revokeReopen.mutateAsync(overrideId))}
        />
      )}
      <AdminScoringPanel {...panelContext} />

      <AdminUsersPanel {...panelContext} />

      <AdminPoolsPanel {...panelContext} />

      <AdminReportsPanel {...panelContext} />

      <AdminAuditPanel {...panelContext} />

      <AdminSettingsPanel {...panelContext} />

    </PageShell>
  );
}
