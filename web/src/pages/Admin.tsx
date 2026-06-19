import { useEffect, useMemo, useState } from "react";
import { Navigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  Clock3,
  Lock,
  RefreshCcw,
  ShieldAlert,
  TimerReset,
  Trophy,
  Users,
} from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import {
  useAddPoolMember,
  useAdminAudit,
  useAdminMatches,
  useAdminMatchAudit,
  useAdminOverview,
  useAdminPoolMembers,
  useAdminPools,
  useAdminPredictions,
  useAdminSettings,
  useAdminUsers,
  useBlockUser,
  useInvalidateUserSessions,
  useReauth,
  useRecalculateAll,
  useRecalculateMatch,
  useRemovePoolMember,
  useReopenPrediction,
  useRevokePredictionReopen,
  useRunSyncNow,
  useSaveAdminSettings,
  useSetMatchFinished,
  useSetMatchResult,
  useTriggerUserPasswordReset,
  useUnblockUser,
  useUserBreakdown,
  useUserPools,
} from "@/hooks/queries";
import { withAdminReauth } from "@/lib/adminReauth";
import { formatKickoff, isKnockout } from "@/lib/utils";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import type { AdminSettings } from "@/types";

type AdminTab =
  | "overview"
  | "matches"
  | "predictions"
  | "scoring"
  | "users"
  | "pools"
  | "audit"
  | "settings";

const tabs: Array<{ id: AdminTab; label: string }> = [
  { id: "overview", label: "Resumo" },
  { id: "matches", label: "Jogos" },
  { id: "predictions", label: "Palpites" },
  { id: "scoring", label: "Pontuação" },
  { id: "users", label: "Usuários" },
  { id: "pools", label: "Bolões" },
  { id: "audit", label: "Auditoria" },
  { id: "settings", label: "Configurações" },
];

function MetricCard({
  icon,
  label,
  value,
  tone = "default",
}: {
  icon: React.ReactNode;
  label: string;
  value: string | number;
  tone?: "default" | "danger" | "highlight";
}) {
  const toneClass =
    tone === "danger"
      ? "border-danger/30 bg-danger-bg"
      : tone === "highlight"
        ? "border-sky/40 bg-sky/15"
        : "border-mint/20 bg-card/80";

  return (
    <Card className={`border ${toneClass} p-4`}>
      <div className="flex items-center gap-3">
        <div className="rounded-full bg-card/80 p-2 text-mint-dark">{icon}</div>
        <div>
          <p className="text-xs uppercase tracking-[0.18em] text-ink-muted">{label}</p>
          <p className="mt-1 font-heading text-2xl font-semibold text-ink">{value}</p>
        </div>
      </div>
    </Card>
  );
}

function TextArea(props: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      {...props}
      className={`min-h-28 w-full rounded-md border-2 border-mint/40 bg-card px-4 py-2.5 text-ink focus:border-mint-dark focus:outline-none focus:shadow-glow ${props.className ?? ""}`}
    />
  );
}

function scoreField(value: number | null | undefined) {
  return value === null || value === undefined ? "" : String(value);
}

function parseScore(value: string) {
  return value.trim() === "" ? 0 : Number.parseInt(value, 10) || 0;
}

export function AdminPage() {
  const { isAdmin, loading } = useAuth();
  const [tab, setTab] = useState<AdminTab>("overview");
  const [error, setError] = useState("");
  const [matchFilters, setMatchFilters] = useState({
    phase: "",
    groupName: "",
    date: "",
    status: "",
    origin: "",
  });
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

  const overview = useAdminOverview();
  const reauth = useReauth();
  const adminUsers = useAdminUsers();
  const adminPools = useAdminPools();
  const adminMatches = useAdminMatches({
    phase: matchFilters.phase || undefined,
    groupName: matchFilters.groupName || undefined,
    date: matchFilters.date || undefined,
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
  const settings = useAdminSettings();

  const runSyncNow = useRunSyncNow();
  const recalcAll = useRecalculateAll();
  const recalcMatch = useRecalculateMatch();
  const setMatchResult = useSetMatchResult();
  const setMatchFinished = useSetMatchFinished();
  const reopenPrediction = useReopenPrediction();
  const revokeReopen = useRevokePredictionReopen();
  const blockUser = useBlockUser();
  const unblockUser = useUnblockUser();
  const invalidateSessions = useInvalidateUserSessions();
  const triggerPasswordReset = useTriggerUserPasswordReset();
  const addPoolMember = useAddPoolMember();
  const removePoolMember = useRemovePoolMember();
  const saveSettings = useSaveAdminSettings();

  useEffect(() => {
    if (!selectedMatchId && adminMatches.data?.length) {
      setSelectedMatchId(adminMatches.data[0].matchRecord.id);
    }
  }, [adminMatches.data, selectedMatchId]);

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
  const [resultQualifier, setResultQualifier] = useState<"home" | "away">("home");
  const [wentPens, setWentPens] = useState(false);
  const [penHome, setPenHome] = useState("");
  const [penAway, setPenAway] = useState("");
  const [overrideExpiry, setOverrideExpiry] = useState("");
  const [overrideReason, setOverrideReason] = useState("");
  const [selectedPoolUserToAdd, setSelectedPoolUserToAdd] = useState("");

  useEffect(() => {
    if (!selectedMatch) return;
    setResultHome(scoreField(selectedMatch.matchRecord.homeScore));
    setResultAway(scoreField(selectedMatch.matchRecord.awayScore));
    setResultQualifier(selectedMatch.matchRecord.qualifier === "away" ? "away" : "home");
    setWentPens(selectedMatch.matchRecord.wentToPenalties);
    setPenHome(scoreField(selectedMatch.matchRecord.penaltyHomeScore));
    setPenAway(scoreField(selectedMatch.matchRecord.penaltyAwayScore));
  }, [selectedMatch]);

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

  if (!loading && !isAdmin) return <Navigate to="/" replace />;

  const handleSaveResult = async () => {
    if (!selectedMatch) return;
    const fromApi = selectedMatch.matchRecord.resultSource === "api";
    if (
      fromApi &&
      !window.confirm(
        "Esse jogo veio da fonte externa. Confirmar correção manual e preservar auditoria?",
      )
    ) {
      return;
    }

    await runAdminAction(() =>
      setMatchResult.mutateAsync({
        matchId: selectedMatch.matchRecord.id,
        homeScore: parseScore(resultHome),
        awayScore: parseScore(resultAway),
        knockout: {
          qualifier: isKnockout(selectedMatch.matchRecord.phase) ? resultQualifier : null,
          wentToPenalties: wentPens,
          penaltyHome: wentPens ? parseScore(penHome) : null,
          penaltyAway: wentPens ? parseScore(penAway) : null,
        },
      }),
    );
  };

  const handleToggleFinished = async () => {
    if (!selectedMatch) return;
    await runAdminAction(() =>
      setMatchFinished.mutateAsync({
        matchId: selectedMatch.matchRecord.id,
        finished: !selectedMatch.matchRecord.finished,
      }),
    );
  };

  const handleReopenPrediction = async (userId: string, matchId: string) => {
    const expiresAt =
      overrideExpiry ||
      new Date(Date.now() + 60 * 60 * 1000).toISOString().slice(0, 16);
    await runAdminAction(() =>
      reopenPrediction.mutateAsync({
        matchId,
        userId,
        reason: overrideReason || "Reabertura administrativa por suporte",
        expiresAt: expiresAt.includes("T") ? `${expiresAt}:00Z` : expiresAt,
      }),
    );
  };

  const selectedMatchRows =
    adminPredictions.data?.filter((row) => row.matchId === (predictionFilters.matchId || selectedMatchId)) ?? [];

  return (
    <PageShell className="max-w-[1280px]">
      <div className="rounded-[28px] border border-mint/20 bg-[radial-gradient(circle_at_top_left,rgba(130,207,255,0.22),transparent_35%),linear-gradient(180deg,rgba(255,255,255,0.96),rgba(248,255,252,0.92))] p-5 shadow-card sm:p-6">
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
            <Button onClick={() => runAdminAction(() => runSyncNow.mutateAsync())}>
              <RefreshCcw className="h-4 w-4" />
              Sincronizar agora
            </Button>
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
          <MetricCard icon={<RefreshCcw className="h-5 w-5" />} label="Ao Vivo" value={overview.data?.liveMatches ?? "-"} tone="highlight" />
          <MetricCard icon={<ShieldAlert className="h-5 w-5" />} label="Corrigidos Manualmente" value={overview.data?.manuallyCorrectedMatches ?? "-"} />
          <MetricCard icon={<AlertTriangle className="h-5 w-5" />} label="Conflitos de API" value={overview.data?.apiConflicts ?? "-"} tone="danger" />
          <MetricCard icon={<Trophy className="h-5 w-5" />} label="Finalizados por API" value={overview.data?.finalizedMatches ?? "-"} />
          <MetricCard icon={<Users className="h-5 w-5" />} label="Usuários" value={overview.data?.userCount ?? "-"} />
          <MetricCard icon={<Lock className="h-5 w-5" />} label="Bloqueados" value={overview.data?.blockedUserCount ?? "-"} tone="danger" />
          <MetricCard icon={<TimerReset className="h-5 w-5" />} label="Sem Palpite Próximo" value={overview.data?.usersWithoutPredictionsSoon ?? "-"} />

          <Card className="lg:col-span-2">
            <h2 className="text-xl">Status da sincronização</h2>
            <p className="mt-2 text-sm text-ink-muted">
              {overview.data?.lastSync
                ? `Última execução: ${overview.data.lastSync.status} em ${formatKickoff(overview.data.lastSync.startedAt)}`
                : "Ainda não houve execução manual registrada."}
            </p>
            <p className="mt-2 text-sm text-ink-muted">
              Automação: {overview.data?.syncEnabled ? "ligada" : "desligada"}
            </p>
          </Card>

          <Card className="lg:col-span-2">
            <h2 className="text-xl">Feed recente de jogos</h2>
            <div className="mt-3 space-y-3">
              {overview.data?.activityFeed.map((item) => (
                <div key={`${item.action}-${item.at}-${item.targetId ?? "none"}`} className="rounded-lg border border-mint/15 bg-card/75 px-4 py-3">
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

      {tab === "matches" && (
        <div className="mt-6 grid gap-5 xl:grid-cols-[1.1fr_0.9fr] [&>*]:min-w-0">
          <Card>
            <div className="grid gap-3 md:grid-cols-5">
              <div>
                <Label>Fase</Label>
                <Input value={matchFilters.phase} onChange={(e) => setMatchFilters((v) => ({ ...v, phase: e.target.value }))} placeholder="Fase" />
              </div>
              <div>
                <Label>Grupo</Label>
                <Input value={matchFilters.groupName} onChange={(e) => setMatchFilters((v) => ({ ...v, groupName: e.target.value }))} placeholder="Grupo" />
              </div>
              <div>
                <Label>Data</Label>
                <Input type="date" value={matchFilters.date} onChange={(e) => setMatchFilters((v) => ({ ...v, date: e.target.value }))} />
              </div>
              <div>
                <Label>Status</Label>
                <Select value={matchFilters.status} onChange={(e) => setMatchFilters((v) => ({ ...v, status: e.target.value }))}>
                  <option value="">Todos</option>
                  <option value="scheduled">Agendado</option>
                  <option value="live">Ao vivo</option>
                  <option value="finalized">Finalizado</option>
                </Select>
              </div>
              <div>
                <Label>Origem</Label>
                <Select value={matchFilters.origin} onChange={(e) => setMatchFilters((v) => ({ ...v, origin: e.target.value }))}>
                  <option value="">Todas</option>
                  <option value="api">Fonte externa</option>
                  <option value="manual">Manual</option>
                </Select>
              </div>
            </div>

            <div className="mt-5 space-y-3">
              {adminMatches.data?.map((item, index) => (
                <motion.button
                  key={item.matchRecord.id}
                  type="button"
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: Math.min(index * 0.02, 0.2) }}
                  onClick={() => setSelectedMatchId(item.matchRecord.id)}
                  className={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedMatchId === item.matchRecord.id ? "border-mint-dark bg-mint/10 shadow-glow" : "border-mint/15 bg-card/70"}`}
                >
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                      <p className="font-heading text-lg text-ink">
                        {item.matchRecord.homeTeam} x {item.matchRecord.awayTeam}
                      </p>
                      <p className="mt-1 text-sm text-ink-muted">
                        {formatKickoff(item.matchRecord.kickoff)} · {item.matchRecord.phase ?? "Sem fase"} · {item.adminStatus}
                      </p>
                    </div>
                    <div className="text-right text-sm">
                      <p className="font-semibold text-ink">
                        {item.matchRecord.homeScore ?? "-"} x {item.matchRecord.awayScore ?? "-"}
                      </p>
                      <p className="text-ink-muted">
                        {item.matchRecord.resultSource === "api"
                          ? "Fonte externa"
                          : item.matchRecord.resultSource === "manual"
                            ? "Manual"
                            : "Sem origem"}
                      </p>
                    </div>
                  </div>
                </motion.button>
              ))}
            </div>
          </Card>

          <Card>
            {selectedMatch ? (
              <>
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h2 className="text-2xl">
                      {selectedMatch.matchRecord.homeTeam} x {selectedMatch.matchRecord.awayTeam}
                    </h2>
                    <p className="mt-1 text-sm text-ink-muted">
                      {selectedMatch.matchRecord.groupName ?? "Sem grupo"} · {formatKickoff(selectedMatch.matchRecord.kickoff)}
                    </p>
                  </div>
                  <Button variant={selectedMatch.matchRecord.finished ? "secondary" : "outline"} onClick={handleToggleFinished}>
                    {selectedMatch.matchRecord.finished ? "Marcar como não finalizado" : "Marcar finalizado"}
                  </Button>
                </div>

                <div className="mt-4 grid gap-3 sm:grid-cols-2">
                  <div>
                    <Label>Placar mandante</Label>
                    <Input value={resultHome} onChange={(e) => setResultHome(e.target.value.replace(/\D+/g, ""))} />
                  </div>
                  <div>
                    <Label>Placar visitante</Label>
                    <Input value={resultAway} onChange={(e) => setResultAway(e.target.value.replace(/\D+/g, ""))} />
                  </div>
                </div>

                {isKnockout(selectedMatch.matchRecord.phase) && (
                  <div className="mt-4 space-y-3">
                    <div>
                      <Label>Classificado</Label>
                      <Select value={resultQualifier} onChange={(e) => setResultQualifier(e.target.value as "home" | "away")}>
                        <option value="home">{selectedMatch.matchRecord.homeTeam}</option>
                        <option value="away">{selectedMatch.matchRecord.awayTeam}</option>
                      </Select>
                    </div>
                    <label className="flex items-center gap-2 text-sm font-semibold text-ink">
                      <input type="checkbox" checked={wentPens} onChange={(e) => setWentPens(e.target.checked)} />
                      Houve pênaltis
                    </label>
                    {wentPens && (
                      <div className="grid gap-3 sm:grid-cols-2">
                        <div>
                          <Label>Pênaltis mandante</Label>
                          <Input value={penHome} onChange={(e) => setPenHome(e.target.value.replace(/\D+/g, ""))} />
                        </div>
                        <div>
                          <Label>Pênaltis visitante</Label>
                          <Input value={penAway} onChange={(e) => setPenAway(e.target.value.replace(/\D+/g, ""))} />
                        </div>
                      </div>
                    )}
                  </div>
                )}

                <div className="mt-5 flex flex-wrap gap-2">
                  <Button onClick={handleSaveResult}>Salvar resultado</Button>
                  <Button variant="outline" onClick={() => runAdminAction(() => recalcMatch.mutateAsync(selectedMatch.matchRecord.id))}>
                    Recalcular este jogo
                  </Button>
                </div>

                <div className="mt-6">
                  <h3 className="text-lg">Auditoria deste jogo</h3>
                  <div className="mt-3 space-y-2">
                    {selectedMatchAudit.data?.map((entry) => (
                      <div key={entry.id} className="rounded-xl border border-mint/15 bg-card/75 px-4 py-3">
                        <p className="font-semibold text-ink">
                          {entry.action} · {entry.actorUsername ?? "Sistema"}
                        </p>
                        <p className="mt-1 text-xs text-ink-muted">{formatKickoff(entry.createdAt)}</p>
                        <pre className="mt-2 overflow-x-auto whitespace-pre-wrap break-words text-xs text-ink-muted">
                          {entry.detailsJson}
                        </pre>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <p className="text-ink-muted">Selecione um jogo para editar.</p>
            )}
          </Card>
        </div>
      )}

      {tab === "predictions" && (
        <div className="mt-6 grid gap-5 xl:grid-cols-[1fr_1fr]">
          <Card>
            <div className="grid gap-3 md:grid-cols-4">
              <div>
                <Label>Jogo</Label>
                <Select value={predictionFilters.matchId} onChange={(e) => setPredictionFilters((v) => ({ ...v, matchId: e.target.value }))}>
                  <option value="">Todos</option>
                  {adminMatches.data?.map((item) => (
                    <option key={item.matchRecord.id} value={item.matchRecord.id}>
                      {item.matchRecord.homeTeam} x {item.matchRecord.awayTeam}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Usuário</Label>
                <Select value={predictionFilters.userId} onChange={(e) => setPredictionFilters((v) => ({ ...v, userId: e.target.value }))}>
                  <option value="">Todos</option>
                  {adminUsers.data?.map((item) => (
                    <option key={item.user.id} value={item.user.id}>
                      {item.user.username}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Bolão</Label>
                <Select value={predictionFilters.poolId} onChange={(e) => setPredictionFilters((v) => ({ ...v, poolId: e.target.value }))}>
                  <option value="">Todos</option>
                  {adminPools.data?.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </Select>
              </div>
              <label className="flex items-end gap-2 text-sm font-semibold text-ink">
                <input type="checkbox" checked={predictionFilters.missingOnly} onChange={(e) => setPredictionFilters((v) => ({ ...v, missingOnly: e.target.checked }))} />
                Só sem palpite
              </label>
            </div>

            <div className="mt-5 space-y-3">
              {adminPredictions.data?.slice(0, 80).map((row) => (
                <div key={`${row.poolId}-${row.userId}-${row.matchId}`} className="rounded-2xl border border-mint/15 bg-card/70 px-4 py-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <p className="font-semibold text-ink">
                        {row.username} · {row.poolName}
                      </p>
                      <p className="text-sm text-ink-muted">
                        {row.homeTeam} x {row.awayTeam} · {formatKickoff(row.kickoff)}
                      </p>
                    </div>
                    <div className="text-right text-sm">
                      <p className="font-semibold text-ink">
                        {row.prediction ? `${row.prediction.homeScore} x ${row.prediction.awayScore}` : "Sem palpite"}
                      </p>
                      <p className="text-ink-muted">
                        {row.locked ? "Travado" : "Aberto"} · {row.overrideInfo ? "reaberto" : "normal"}
                      </p>
                    </div>
                  </div>
                  {row.overrideInfo && (
                    <p className="mt-2 text-xs text-mint-dark">
                      Reaberto até {formatKickoff(row.overrideInfo.expiresAt)} · motivo: {row.overrideInfo.reason}
                    </p>
                  )}
                  <div className="mt-3 flex flex-wrap gap-2">
                    <Button size="sm" variant="outline" onClick={() => handleReopenPrediction(row.userId, row.matchId)}>
                      Reabrir palpite
                    </Button>
                    {row.overrideInfo && (
                      <Button size="sm" variant="outline" onClick={() => runAdminAction(() => revokeReopen.mutateAsync(row.overrideInfo!.id))}>
                        Revogar reabertura
                      </Button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </Card>

          <Card>
            <h2 className="text-xl">Reabertura controlada</h2>
            <p className="mt-2 text-sm text-ink-muted">
              Use em caso de bug ou suporte. A auditoria registra quem abriu, para quem e até
              quando vale.
            </p>
            <div className="mt-4">
              <Label>Expira em</Label>
              <Input type="datetime-local" value={overrideExpiry} onChange={(e) => setOverrideExpiry(e.target.value)} />
            </div>
            <div className="mt-4">
              <Label>Motivo padrão</Label>
              <TextArea value={overrideReason} onChange={(e) => setOverrideReason(e.target.value)} placeholder="Ex.: falha de travamento indevido após o kickoff" />
            </div>
            <div className="mt-5 rounded-2xl border border-mint/15 bg-card/70 px-4 py-4">
              <p className="font-semibold text-ink">Quem ainda não palpitou no filtro atual</p>
              <div className="mt-3 space-y-2">
                {selectedMatchRows
                  .filter((row) => row.missing)
                  .slice(0, 12)
                  .map((row) => (
                    <div key={`${row.userId}-${row.matchId}`} className="flex items-center justify-between gap-3 rounded-xl border border-mint/10 bg-card px-3 py-3">
                      <div>
                        <p className="font-semibold text-ink">{row.username}</p>
                        <p className="text-xs text-ink-muted">{row.poolName}</p>
                      </div>
                      <Button size="sm" onClick={() => handleReopenPrediction(row.userId, row.matchId)}>
                        Reabrir
                      </Button>
                    </div>
                  ))}
              </div>
            </div>
          </Card>
        </div>
      )}

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

      {tab === "settings" && settingsDraft && (
        <Card className="mt-6">
          <h2 className="text-2xl">Configurações operacionais</h2>
          <div className="mt-5 grid gap-4 md:grid-cols-2">
            <label className="flex items-center gap-2 text-sm font-semibold text-ink">
              <input
                type="checkbox"
                checked={settingsDraft.knockoutReleased}
                onChange={(e) => setSettingsDraft((v) => (v ? { ...v, knockoutReleased: e.target.checked } : v))}
              />
              Mata-mata liberado
            </label>
            <label className="flex items-center gap-2 text-sm font-semibold text-ink">
              <input
                type="checkbox"
                checked={settingsDraft.autoSyncEnabled}
                onChange={(e) => setSettingsDraft((v) => (v ? { ...v, autoSyncEnabled: e.target.checked } : v))}
              />
              Atualização automática ligada
            </label>
            <div>
              <Label>Sincronização em minutos</Label>
              <Input
                value={String(settingsDraft.syncIntervalMinutes)}
                onChange={(e) => setSettingsDraft((v) => (v ? { ...v, syncIntervalMinutes: Number(e.target.value) || 0 } : v))}
              />
            </div>
            <div>
              <Label>Fechar palpites antes do jogo (min)</Label>
              <Input
                value={String(settingsDraft.predictionLockMinutes)}
                onChange={(e) => setSettingsDraft((v) => (v ? { ...v, predictionLockMinutes: Number(e.target.value) || 0 } : v))}
              />
            </div>
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
    </PageShell>
  );
}
