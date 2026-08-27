import { useEffect, useMemo, useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
  Eye,
  EyeOff,
  Flag,
  Lock,
  RefreshCcw,
  Send,
  ShieldAlert,
  TimerReset,
  Trophy,
  Users,
} from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import {
  useAddPoolMember,
  useAdminAudit,
  useAdminEvents,
  useAdminMatches,
  useAdminMatchAudit,
  useAdminOverview,
  useAdminPoolMembers,
  useAdminPoolReports,
  useAdminPools,
  useAdminPredictions,
  useAdminSendPushBroadcast,
  useAdminSendPushToUser,
  useAdminSettings,
  useAdminUsers,
  useBlockUser,
  useCreateMatch,
  useCheckFixture,
  useDeleteMatch,
  useAdminDeleteEvent,
  useFinishEvent,
  useInvalidateUserSessions,
  useKnockoutReleased,
  useReauth,
  useRecalculateAll,
  useRecalculateMatch,
  useRemovePoolMember,
  useReopenPrediction,
  useRevokePredictionReopen,
  useRunSyncNow,
  useRunBackfill,
  useSaveAdminSettings,
  useSetKnockoutReleased,
  useSetMatchFinished,
  useSetMatchResult,
  useTriggerUserPasswordReset,
  useUnblockUser,
  useUpdateMatchSchedule,
  useUpdatePoolReportStatus,
  useSetMatchFixture,
  useSetEventPoolCreation,
  usePublishEventVersion,
  useUserBreakdown,
  useUserPools,
} from "@/hooks/queries";
import { withAdminReauth } from "@/lib/adminReauth";
import { formatKickoff, isKnockout } from "@/lib/utils";
import { formatSelectionLabel } from "@/lib/selections";
import { brasiliaDateToIsoDateFilter, brasiliaInputToIso, FixtureCheckState, fixtureFingerprint, isoToBrasiliaInput, KNOCKOUT_PHASES, validateFixtureAgainstMatch } from "@/components/admin/fixtureValidation";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { AdminEventsPanel } from "@/components/admin/AdminEventsPanel";
import { AdminMatchesPanel } from "@/components/admin/AdminMatchesPanel";
import { AdminPredictionsPanel } from "@/components/admin/AdminPredictionsPanel";
import { emptyAdminMatchFilters, useAdminMatchFilters } from "@/hooks/useAdminMatchFilters";
import { api } from "@/lib/api";
import type { AdminEventRecord, AdminMatchRecord, AdminSettings, PoolReportStatus } from "@/types";

type AdminTab =
  | "overview"
  | "events"
  | "matches"
  | "predictions"
  | "scoring"
  | "users"
  | "pools"
  | "reports"
  | "audit"
  | "settings";

const tabs: Array<{ id: AdminTab; label: string }> = [
  { id: "overview", label: "Resumo" },
  { id: "events", label: "Edições" },
  { id: "matches", label: "Jogos" },
  { id: "predictions", label: "Palpites" },
  { id: "scoring", label: "Pontuação" },
  { id: "users", label: "Usuários" },
  { id: "pools", label: "Bolões" },
  { id: "reports", label: "Denúncias" },
  { id: "audit", label: "Auditoria" },
  { id: "settings", label: "Configurações" },
];

const reportCategoryLabels = {
  inappropriate_content: "Conteúdo inadequado",
  spam_or_fraud: "Spam ou fraude",
  harassment: "Assédio",
  other: "Outro",
} as const;

const reportStatusLabels: Record<PoolReportStatus, string> = {
  open: "Aberta",
  reviewing: "Em análise",
  resolved: "Resolvida",
  dismissed: "Arquivada",
};
const reportStatusOptions: PoolReportStatus[] = ["open", "reviewing", "resolved", "dismissed"];

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
  const navigate = useNavigate();
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

  const runSyncNow = useRunSyncNow();
  const runBackfill = useRunBackfill();
  const recalcAll = useRecalculateAll();
  const recalcMatch = useRecalculateMatch();
  const setMatchResult = useSetMatchResult();
  const setMatchFinished = useSetMatchFinished();
  const createMatch = useCreateMatch();
  const setKnockoutReleased = useSetKnockoutReleased();
  const updateMatchSchedule = useUpdateMatchSchedule();
  const setMatchFixture = useSetMatchFixture();
  const checkFixture = useCheckFixture();
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
  const [editFixtureId, setEditFixtureId] = useState("");
  const [fixtureError, setFixtureError] = useState("");
  const [fixtureSuccess, setFixtureSuccess] = useState("");
  const [fixtureCheckState, setFixtureCheckState] = useState<FixtureCheckState | null>(null);

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
    setEditFixtureId(
      selectedMatch.externalFixtureId != null ? String(selectedMatch.externalFixtureId) : "",
    );
    setFixtureError("");
    setFixtureSuccess("");
    setFixtureCheckState(null);
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
    if (!fixtureSuccess) return;
    const timer = window.setTimeout(() => setFixtureSuccess(""), 5000);
    return () => window.clearTimeout(timer);
  }, [fixtureSuccess]);

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

  if (!loading && !isAdmin) return <Navigate to="/" replace />;

  const adminPushPending = sendPushToUser.isPending || sendPushBroadcast.isPending;

  const buildAdminPushPayload = () => {
    const title = pushTitle.trim();
    const body = pushBody.trim();
    const url = pushUrl.trim() || "/";
    setPushSuccess("");

    if (!title || !body) {
      setError("Preencha titulo e mensagem do push.");
      return null;
    }
    if (!url.startsWith("/") || url.startsWith("//")) {
      setError("O link do push deve ser um caminho interno, por exemplo /predictions.");
      return null;
    }

    return { title, body, url };
  };

  const handleSendPushToSelectedUser = async () => {
    if (!selectedUser) return;
    const payload = buildAdminPushPayload();
    if (!payload) return;

    try {
      const result = await runAdminAction(() =>
        sendPushToUser.mutateAsync({
          userId: selectedUser.user.id,
          payload,
        }),
      );
      if (!result) return;
      if (result.successfulCount > 0) {
        setPushSuccess(
          `Push enviado para ${result.successfulCount} dispositivo(s).` +
            (result.failedCount > 0 ? ` Falha em ${result.failedCount}.` : ""),
        );
      } else if (result.activeSubscriptionCount === 0) {
        setPushSuccess("Nenhum dispositivo elegivel: o usuario precisa ativar notificacoes nesta conta.");
      } else {
        setPushSuccess("Nenhum push foi entregue. Confira se os dispositivos ainda estao validos.");
      }
    } catch {
      // Erro ja exibido por runAdminAction.
    }
  };

  const handleSendPushBroadcast = async () => {
    const payload = buildAdminPushPayload();
    if (!payload) return;
    if (
      !window.confirm(
        "Enviar este push para todos os usuarios com notificacoes ativadas? Essa acao nao pode ser desfeita.",
      )
    ) {
      return;
    }

    try {
      const result = await runAdminAction(() => sendPushBroadcast.mutateAsync(payload));
      if (!result) return;
      if (result.successfulCount > 0) {
        setPushSuccess(
          `Push em massa enviado para ${result.successfulCount} dispositivo(s) de ${result.targetUserCount} usuario(s).` +
            (result.failedCount > 0 ? ` Falha em ${result.failedCount}.` : ""),
        );
      } else if (result.activeSubscriptionCount === 0) {
        setPushSuccess("Nenhum usuario elegivel: ninguem tem notificacoes ativadas no momento.");
      } else {
        setPushSuccess("Nenhum push foi entregue. Confira se os dispositivos ainda estao validos.");
      }
    } catch {
      // Erro ja exibido por runAdminAction.
    }
  };

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

    const knockoutMatch = isKnockout(selectedMatch.matchRecord.phase);
    const home = parseScore(resultHome);
    const away = parseScore(resultAway);
    const draw = knockoutMatch && home === away;
    if (draw) {
      if (penHome === "" || penAway === "") {
        setError("Empate no tempo normal: informe o placar dos pênaltis dos dois lados.");
        return;
      }
      if (parseScore(penHome) === parseScore(penAway)) {
        setError("O placar dos pênaltis não pode terminar empatado.");
        return;
      }
    }

    await runAdminAction(() =>
      setMatchResult.mutateAsync({
        matchId: selectedMatch.matchRecord.id,
        homeScore: home,
        awayScore: away,
        knockout: {
          qualifier: null,
          wentToPenalties: draw,
          penaltyHome: draw ? parseScore(penHome) : null,
          penaltyAway: draw ? parseScore(penAway) : null,
        },
      }),
    );
  };

  const handleCreateMatch = async () => {
    setCreateMatchError("");
    setCreateMatchSuccess("");
    if (!newMatchHome.trim() || !newMatchAway.trim()) {
      setCreateMatchError("Escolha as duas seleções.");
      return;
    }
    if (newMatchHome.trim() === newMatchAway.trim()) {
      setCreateMatchError("Mandante e visitante não podem ser a mesma seleção.");
      return;
    }
    if (!newMatchDate || !newMatchTime) {
      setCreateMatchError("Informe a data e o horário do jogo.");
      return;
    }
    const kickoff = brasiliaInputToIso(newMatchDate, newMatchTime);
    if (!kickoff) {
      setCreateMatchError("Informe data no formato DD/MM/AAAA e horário no formato HH:mm.");
      return;
    }
    const homeLabel = formatSelectionLabel(newMatchHome.trim());
    const awayLabel = formatSelectionLabel(newMatchAway.trim());
    try {
      await runAdminAction(() =>
        createMatch.mutateAsync({
          homeTeam: newMatchHome.trim(),
          awayTeam: newMatchAway.trim(),
          phase: newMatchPhase,
          kickoff,
        }),
      );
      setNewMatchHome("");
      setNewMatchAway("");
      setNewMatchDate("");
      setNewMatchTime("");
      setShowCreateMatchForm(false);
      setCreateMatchSuccess(`${homeLabel} x ${awayLabel} adicionado ao mata-mata (${newMatchPhase}).`);
    } catch {
      // erro já exibido por runAdminAction
    }
  };

  const handleToggleKnockout = async () => {
    setKnockoutToggleMsg("");
    const next = !knockoutReleased;
    try {
      await runAdminAction(() => setKnockoutReleased.mutateAsync(next));
      setKnockoutToggleMsg(
        next
          ? "Mata-mata liberado — agora visível para todos os participantes."
          : "Mata-mata ocultado — só você (admin) vê os confrontos.",
      );
    } catch {
      // erro já exibido por runAdminAction
    }
  };

  const handleFinishEvent = async (eventId: string, name: string) => {
    if (!window.confirm(`Encerrar a edição “${name}”? Os dados permanecem consultáveis e não serão recalculados.`)) return;
    try {
      await runAdminAction(() => finishEvent.mutateAsync(eventId));
    } catch {
      // erro já exibido por runAdminAction
    }
  };

  const handleDeleteEvent = async (event: AdminEventRecord) => {
    const willArchive = event.status !== "draft" || event.poolCount > 0;
    const confirmation = willArchive
      ? `Arquivar o evento "${event.name}"? Ele sairá dos catálogos, mas os ${event.poolCount} bolão(ões) existentes continuarão preservados.`
      : `Excluir definitivamente o rascunho "${event.name}"? Esta ação não pode ser desfeita.`;
    if (!window.confirm(confirmation)) return;
    try {
      await runAdminAction(() => deleteEvent.mutateAsync(event.id));
    } catch {
      // erro já exibido por runAdminAction
    }
  };

  const handleUpdateSchedule = async () => {
    if (!selectedMatch) return;
    setScheduleError("");
    if (!editHome.trim() || !editAway.trim()) {
      setScheduleError("Informe os dois times.");
      return;
    }
    if (!editMatchDate || !editMatchTime) {
      setScheduleError("Informe a data e o horário do jogo.");
      return;
    }
    const kickoff = brasiliaInputToIso(editMatchDate, editMatchTime);
    if (!kickoff) {
      setScheduleError("Informe data no formato DD/MM/AAAA e horário no formato HH:mm.");
      return;
    }
    await runAdminAction(() =>
      updateMatchSchedule.mutateAsync({
        matchId: selectedMatch.matchRecord.id,
        homeTeam: editHome.trim(),
        awayTeam: editAway.trim(),
        phase: editPhase,
        kickoff,
      }),
    );
  };

  const handleSaveFixture = async () => {
    if (!selectedMatch) return;
    setFixtureError("");
    setFixtureSuccess("");
    const trimmed = editFixtureId.trim();
    let fixtureId: number | null = null;
    if (trimmed !== "") {
      const parsed = Number(trimmed);
      if (!Number.isInteger(parsed) || parsed <= 0) {
        setFixtureError("Informe um ID numérico positivo, ou deixe vazio para remover.");
        return;
      }
      fixtureId = parsed;
    }
    if (fixtureId != null) {
      const kickoff = brasiliaInputToIso(editMatchDate, editMatchTime);
      if (!kickoff) {
        setFixtureError("Salve uma data/horário válidos antes de mapear o ID do evento.");
        return;
      }
      const fingerprint = fixtureFingerprint(fixtureId, editHome.trim(), editAway.trim(), kickoff);
      if (!fixtureCheckState || !fixtureCheckState.ok || fixtureCheckState.fingerprint !== fingerprint) {
        setFixtureError("Cheque o ID e confirme confronto + horário antes de salvar o mapeamento.");
        return;
      }
    }
    try {
      await runAdminAction(() =>
        setMatchFixture.mutateAsync({
          matchId: selectedMatch.matchRecord.id,
          externalFixtureId: fixtureId,
        }),
      );
      setFixtureSuccess(
        fixtureId == null ? "Mapeamento removido." : `ID ${fixtureId} salvo com sucesso.`,
      );
    } catch (err) {
      setFixtureError(err instanceof Error ? err.message : "Falha ao salvar o ID do evento.");
    }
  };

  const handleCheckFixture = async () => {
    setFixtureError("");
    setFixtureCheckState(null);
    const trimmed = editFixtureId.trim();
    const parsed = Number(trimmed);
    if (!trimmed || !Number.isInteger(parsed) || parsed <= 0) {
      setFixtureError("Informe um ID numérico positivo para checar.");
      return;
    }
    const kickoff = brasiliaInputToIso(editMatchDate, editMatchTime);
    if (!kickoff || !editHome.trim() || !editAway.trim()) {
      setFixtureError("Informe confronto, data e horário válidos antes de checar o ID.");
      return;
    }

    try {
      const checked = await runAdminAction(() => checkFixture.mutateAsync(parsed));
      const validation = validateFixtureAgainstMatch(checked, {
        homeTeam: editHome.trim(),
        awayTeam: editAway.trim(),
        kickoff,
      });
      setFixtureCheckState({
        eventId: parsed,
        ok: validation.ok,
        message: validation.message,
        fingerprint: fixtureFingerprint(parsed, editHome.trim(), editAway.trim(), kickoff),
      });
    } catch (err) {
      setFixtureError(err instanceof Error ? err.message : "Falha ao checar o ID no provedor.");
    }
  };

  const applySuggestion = () => {
    if (!selectedMatch) return;
    const m = selectedMatch;
    if (m.autoHomeScore == null || m.autoAwayScore == null) return;
    setResultHome(String(m.autoHomeScore));
    setResultAway(String(m.autoAwayScore));
    setPenHome(m.autoPenaltyHomeScore != null ? String(m.autoPenaltyHomeScore) : "");
    setPenAway(m.autoPenaltyAwayScore != null ? String(m.autoPenaltyAwayScore) : "");
  };

  const handleDeleteMatch = async (target?: AdminMatchRecord) => {
    const match = target ?? selectedMatch;
    if (!match) return;
    if (
      !window.confirm(
        `Excluir o jogo ${match.matchRecord.homeTeam} x ${match.matchRecord.awayTeam}? Os palpites desse jogo serão removidos.`,
      )
    ) {
      return;
    }
    await runAdminAction(() => deleteMatch.mutateAsync(match.matchRecord.id));
    if (selectedMatchId === match.matchRecord.id) setSelectedMatchId("");
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
            <Button onClick={() => runAdminAction(() => runSyncNow.mutateAsync())}>
              <RefreshCcw className="h-4 w-4" />
              Sincronizar agora
            </Button>
            <Button variant="outline" onClick={() => runAdminAction(() => runBackfill.mutateAsync())}>
              <RefreshCcw className="h-4 w-4" />
              Sincronizar histórico
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
          editFixtureId={editFixtureId}
          setEditFixtureId={setEditFixtureId}
          fixtureError={fixtureError}
          fixtureSuccess={fixtureSuccess}
          fixtureCheckState={fixtureCheckState}
          setFixtureCheckState={setFixtureCheckState}
          onCreateMatch={handleCreateMatch}
          createMatchPending={createMatch.isPending}
          onToggleFinished={handleToggleFinished}
          onApplySuggestion={applySuggestion}
          onSaveResult={handleSaveResult}
          onRecalculate={() => selectedMatch ? runAdminAction(() => recalcMatch.mutateAsync(selectedMatch.matchRecord.id)) : undefined}
          onUpdateSchedule={handleUpdateSchedule}
          updateSchedulePending={updateMatchSchedule.isPending}
          onDeleteMatch={handleDeleteMatch}
          deleteMatchPending={deleteMatch.isPending}
          onSaveFixture={handleSaveFixture}
          setFixturePending={setMatchFixture.isPending}
          onCheckFixture={handleCheckFixture}
          checkFixturePending={checkFixture.isPending}
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
    </PageShell>
  );
}
