import { useEffect, useMemo, useState } from "react";
import { Navigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
  Eye,
  EyeOff,
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
  useCreateMatch,
  useCheckFixture,
  useDeleteMatch,
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
  useSetMatchFixture,
  useUserBreakdown,
  useUserPools,
} from "@/hooks/queries";
import { withAdminReauth } from "@/lib/adminReauth";
import { formatKickoff, isKnockout } from "@/lib/utils";
import { formatSelectionLabel, getSelectionGroups, isKnownSelection } from "@/lib/selections";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import type { AdminMatchRecord, AdminSettings } from "@/types";

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

// Seleção de seleções (com bandeira) para montar confrontos do mata-mata.
// Mantém o valor atual como opção mesmo se não estiver no catálogo, para não
// perder confrontos já cadastrados com nomes legados.
function TeamSelect({
  value,
  onChange,
  ariaLabel,
}: {
  value: string;
  onChange: (value: string) => void;
  ariaLabel?: string;
}) {
  const groups = getSelectionGroups();
  const unknown = value !== "" && !isKnownSelection(value);
  return (
    <Select value={value} onChange={(e) => onChange(e.target.value)} aria-label={ariaLabel}>
      <option value="">Selecione a seleção</option>
      {unknown && <option value={value}>{formatSelectionLabel(value)}</option>}
      <optgroup label="Seleções">
        {groups.teams.map((selection) => (
          <option key={selection.key} value={selection.name}>
            {formatSelectionLabel(selection.name)}
          </option>
        ))}
      </optgroup>
      {groups.placeholders.length > 0 && (
        <optgroup label="Chaves do mata-mata">
          {groups.placeholders.map((selection) => (
            <option key={selection.key} value={selection.name}>
              {formatSelectionLabel(selection.name)}
            </option>
          ))}
        </optgroup>
      )}
    </Select>
  );
}

function scoreField(value: number | null | undefined) {
  return value === null || value === undefined ? "" : String(value);
}

function parseScore(value: string) {
  return value.trim() === "" ? 0 : Number.parseInt(value, 10) || 0;
}

function adminStatusLabel(status: string): string {
  switch (status) {
    case "scheduled":
      return "agendado";
    case "live":
      return "ao vivo";
    case "finished_pending":
      return "pendente de confirmação";
    case "finalized":
      return "finalizado";
    default:
      return status;
  }
}

// Fases de mata-mata disponíveis no cadastro manual de jogos.
const KNOCKOUT_PHASES = [
  "16 avos de final",
  "Oitavas de final",
  "Quartas de final",
  "Semifinal",
  "Disputa de 3º lugar",
  "Final",
];

// Converte um ISO (UTC) para o input datetime-local em horario de Brasilia.
function isoToLocalInput(iso: string | null | undefined): string {
  if (!iso) return "";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const parts = new Intl.DateTimeFormat("pt-BR", {
    timeZone: "America/Sao_Paulo",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).formatToParts(date);
  const value = (type: Intl.DateTimeFormatPartTypes) =>
    parts.find((part) => part.type === type)?.value ?? "";
  return `${value("year")}-${value("month")}-${value("day")}T${value("hour")}:${value("minute")}`;
}

// O input datetime-local entrega "YYYY-MM-DDTHH:mm"; no admin, esse horário é
// digitado como Brasília. Salvamos em UTC para o backend/poller.
function localInputToIso(value: string): string {
  const trimmed = value.trim();
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(trimmed);
  if (!match) return trimmed;
  const [, year, month, day, hour, minute] = match;
  const utc = Date.UTC(
    Number(year),
    Number(month) - 1,
    Number(day),
    Number(hour) + 3,
    Number(minute),
  );
  return new Date(utc).toISOString();
}

export function AdminPage() {
  const { isAdmin, loading } = useAuth();
  const [tab, setTab] = useState<AdminTab>("overview");
  const [error, setError] = useState("");
  const emptyMatchFilters = { phase: "", groupName: "", date: "", status: "", origin: "", team: "" };
  const [matchFilters, setMatchFilters] = useState(emptyMatchFilters);
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
  const [penHome, setPenHome] = useState("");
  const [penAway, setPenAway] = useState("");
  const [overrideExpiry, setOverrideExpiry] = useState("");
  const [overrideReason, setOverrideReason] = useState("");
  const [selectedPoolUserToAdd, setSelectedPoolUserToAdd] = useState("");

  // Cadastro manual de jogo de mata-mata.
  const [newMatchHome, setNewMatchHome] = useState("");
  const [newMatchAway, setNewMatchAway] = useState("");
  const [newMatchPhase, setNewMatchPhase] = useState(KNOCKOUT_PHASES[0]);
  const [newMatchKickoff, setNewMatchKickoff] = useState("");
  const [createMatchError, setCreateMatchError] = useState("");
  const [createMatchSuccess, setCreateMatchSuccess] = useState("");
  const [knockoutToggleMsg, setKnockoutToggleMsg] = useState("");

  const knockoutReleased = knockoutReleasedQuery.data?.released ?? false;
  const knockoutMatches = useMemo(
    () =>
      (allMatchesForKnockout.data ?? []).filter((item) =>
        isKnockout(item.matchRecord.phase),
      ),
    [allMatchesForKnockout.data],
  );

  // Opções de filtro derivadas dos jogos existentes (fase e grupo de verdade,
  // em vez de digitar o texto exato à mão).
  const phaseOptions = useMemo(() => {
    const set = new Set<string>();
    for (const item of allMatchesForKnockout.data ?? []) {
      if (item.matchRecord.phase) set.add(item.matchRecord.phase);
    }
    return Array.from(set).sort();
  }, [allMatchesForKnockout.data]);

  const groupOptions = useMemo(() => {
    const set = new Set<string>();
    for (const item of allMatchesForKnockout.data ?? []) {
      if (item.matchRecord.groupName) set.add(item.matchRecord.groupName);
    }
    return Array.from(set).sort();
  }, [allMatchesForKnockout.data]);

  // Busca por time é client-side, sobre o que o backend já filtrou.
  const visibleMatches = useMemo(() => {
    const term = matchFilters.team.trim().toLowerCase();
    const list = adminMatches.data ?? [];
    if (!term) return list;
    return list.filter((item) => {
      const home = formatSelectionLabel(item.matchRecord.homeTeam).toLowerCase();
      const away = formatSelectionLabel(item.matchRecord.awayTeam).toLowerCase();
      return home.includes(term) || away.includes(term);
    });
  }, [adminMatches.data, matchFilters.team]);

  const hasActiveMatchFilters =
    matchFilters.phase !== "" ||
    matchFilters.groupName !== "" ||
    matchFilters.date !== "" ||
    matchFilters.status !== "" ||
    matchFilters.origin !== "" ||
    matchFilters.team !== "";

  // Edição de confronto/fase/horário do jogo selecionado.
  const [editHome, setEditHome] = useState("");
  const [editAway, setEditAway] = useState("");
  const [editPhase, setEditPhase] = useState(KNOCKOUT_PHASES[0]);
  const [editKickoff, setEditKickoff] = useState("");
  const [scheduleError, setScheduleError] = useState("");
  const [editFixtureId, setEditFixtureId] = useState("");
  const [fixtureError, setFixtureError] = useState("");
  const [fixtureSuccess, setFixtureSuccess] = useState("");
  const [fixtureCheckMsg, setFixtureCheckMsg] = useState("");

  useEffect(() => {
    if (!selectedMatch) return;
    setResultHome(scoreField(selectedMatch.matchRecord.homeScore));
    setResultAway(scoreField(selectedMatch.matchRecord.awayScore));
    setPenHome(scoreField(selectedMatch.matchRecord.penaltyHomeScore));
    setPenAway(scoreField(selectedMatch.matchRecord.penaltyAwayScore));
    setEditHome(selectedMatch.matchRecord.homeTeam);
    setEditAway(selectedMatch.matchRecord.awayTeam);
    setEditPhase(selectedMatch.matchRecord.phase ?? KNOCKOUT_PHASES[0]);
    setEditKickoff(isoToLocalInput(selectedMatch.matchRecord.kickoff));
    setScheduleError("");
    setEditFixtureId(
      selectedMatch.externalFixtureId != null ? String(selectedMatch.externalFixtureId) : "",
    );
    setFixtureError("");
    setFixtureSuccess("");
    setFixtureCheckMsg("");
  }, [selectedMatch]);

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
    if (!newMatchKickoff) {
      setCreateMatchError("Informe a data e o horário do jogo.");
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
          kickoff: localInputToIso(newMatchKickoff),
        }),
      );
      setNewMatchHome("");
      setNewMatchAway("");
      setNewMatchKickoff("");
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

  const handleUpdateSchedule = async () => {
    if (!selectedMatch) return;
    setScheduleError("");
    if (!editHome.trim() || !editAway.trim()) {
      setScheduleError("Informe os dois times.");
      return;
    }
    if (!editKickoff) {
      setScheduleError("Informe a data e o horário do jogo.");
      return;
    }
    await runAdminAction(() =>
      updateMatchSchedule.mutateAsync({
        matchId: selectedMatch.matchRecord.id,
        homeTeam: editHome.trim(),
        awayTeam: editAway.trim(),
        phase: editPhase,
        kickoff: localInputToIso(editKickoff),
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
    setFixtureCheckMsg("");
    const trimmed = editFixtureId.trim();
    const parsed = Number(trimmed);
    if (!trimmed || !Number.isInteger(parsed) || parsed <= 0) {
      setFixtureError("Informe um ID numérico positivo para checar.");
      return;
    }

    try {
      const checked = await runAdminAction(() => checkFixture.mutateAsync(parsed));
      if (!checked.found) {
        setFixtureCheckMsg(`ID ${checked.eventId}: o provedor respondeu, mas não trouxe detalhes do evento.`);
        return;
      }
      const kickoff = checked.kickoff ? ` · ${formatKickoff(checked.kickoff)}` : "";
      setFixtureCheckMsg(`ID correto: ${checked.label}${kickoff}`);
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

  // Seleciona o jogo e rola até o painel de edição (usado nas listas de cima).
  const handleEditMatch = (matchId: string) => {
    setSelectedMatchId(matchId);
    requestAnimationFrame(() => {
      document.getElementById("match-edit-panel")?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
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

      {tab === "matches" && (
        <div className="mt-6 space-y-5">
          <Card className="border-l-4 border-yellow-dark">
            {/* Cabeçalho + status de liberação */}
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="flex items-center gap-2">
                <Trophy className="h-5 w-5 text-yellow-dark" />
                <h2 className="text-xl">Mata-mata</h2>
              </div>
              <span
                className={`inline-flex items-center gap-1.5 rounded-pill px-3 py-1 text-xs font-semibold ring-1 ${
                  knockoutReleased
                    ? "bg-success/15 text-mint-dark ring-success/40"
                    : "bg-yellow/15 text-yellow-dark ring-yellow-dark/40"
                }`}
              >
                {knockoutReleased ? <Eye className="h-3.5 w-3.5" /> : <EyeOff className="h-3.5 w-3.5" />}
                {knockoutReleased ? "Liberado" : "Oculto"}
              </span>
            </div>

            <div
              className={`mt-4 rounded-xl border p-4 ${
                knockoutReleased
                  ? "border-success/40 bg-success/10"
                  : "border-yellow-dark/30 bg-yellow/10"
              }`}
            >
              <p className="text-sm text-ink">
                {knockoutReleased
                  ? "Os confrontos do mata-mata estão visíveis para todos os participantes."
                  : "Os confrontos estão ocultos. Só você (admin) os vê para montar o chaveamento — libere quando a fase de grupos terminar."}
              </p>
              <div className="mt-3 flex flex-wrap items-center gap-3">
                <Button
                  variant={knockoutReleased ? "outline" : "primary"}
                  disabled={setKnockoutReleased.isPending || knockoutReleasedQuery.isLoading}
                  onClick={handleToggleKnockout}
                >
                  {setKnockoutReleased.isPending
                    ? "Salvando..."
                    : knockoutReleased
                      ? "Ocultar mata-mata"
                      : "Liberar mata-mata"}
                </Button>
                <span className="text-sm text-ink-muted">
                  {knockoutMatches.length} confronto(s) cadastrado(s)
                </span>
              </div>
              {knockoutToggleMsg && (
                <p className="mt-3 flex items-center gap-2 text-sm font-semibold text-mint-dark">
                  <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                  {knockoutToggleMsg}
                </p>
              )}
            </div>

            {/* Adicionar confronto */}
            <div className="mt-5 border-t border-mint/15 pt-5">
              <h3 className="text-lg">Adicionar confronto</h3>
              <p className="mt-1 text-sm text-ink-muted">
                Escolha as seleções, a fase e o horário. O confronto entra direto no chaveamento do mata-mata.
              </p>
              <div className="mt-4 grid gap-3 md:grid-cols-4">
                <div>
                  <Label>Mandante</Label>
                  <TeamSelect value={newMatchHome} onChange={setNewMatchHome} ariaLabel="Seleção mandante" />
                </div>
                <div>
                  <Label>Visitante</Label>
                  <TeamSelect value={newMatchAway} onChange={setNewMatchAway} ariaLabel="Seleção visitante" />
                </div>
                <div>
                  <Label>Fase</Label>
                  <Select value={newMatchPhase} onChange={(e) => setNewMatchPhase(e.target.value)}>
                    {KNOCKOUT_PHASES.map((phase) => (
                      <option key={phase} value={phase}>
                        {phase}
                      </option>
                    ))}
                  </Select>
                </div>
                <div>
                  <Label>Data e horário</Label>
                  <Input type="datetime-local" value={newMatchKickoff} onChange={(e) => setNewMatchKickoff(e.target.value)} />
                </div>
              </div>
              {createMatchError && <div className="mt-3"><ErrorBanner>{createMatchError}</ErrorBanner></div>}
              <div className="mt-4 flex flex-wrap items-center gap-3">
                <Button onClick={handleCreateMatch} disabled={createMatch.isPending}>
                  {createMatch.isPending ? "Criando..." : "Adicionar ao mata-mata"}
                </Button>
                {createMatchSuccess && (
                  <span className="flex items-center gap-2 text-sm font-semibold text-mint-dark">
                    <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                    {createMatchSuccess}
                  </span>
                )}
              </div>
            </div>

            {/* Chaveamento atual — confirma o que está realmente no mata-mata */}
            <div className="mt-5 border-t border-mint/15 pt-5">
              <h3 className="text-lg">Confrontos no mata-mata</h3>
              <p className="mt-1 text-sm text-ink-muted">
                Use <strong>Editar</strong> para ajustar times/fase/horário e mapear o ID do evento, ou <strong>Excluir</strong> para remover o confronto.
              </p>
              {knockoutMatches.length === 0 ? (
                <p className="mt-2 text-sm text-ink-muted">
                  Nenhum confronto de mata-mata ainda. Adicione um acima para começar o chaveamento.
                </p>
              ) : (
                <div className="mt-3 space-y-2">
                  {knockoutMatches.map((item) => (
                    <div
                      key={item.matchRecord.id}
                      className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-mint/15 bg-card/70 px-4 py-3"
                    >
                      <div>
                        <p className="font-heading text-ink">
                          {formatSelectionLabel(item.matchRecord.homeTeam)}{" "}
                          <span className="text-ink-muted">x</span>{" "}
                          {formatSelectionLabel(item.matchRecord.awayTeam)}
                        </p>
                        <p className="mt-0.5 text-xs text-ink-muted">
                          {formatKickoff(item.matchRecord.kickoff)}
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="rounded-pill bg-yellow/20 px-3 py-1 text-xs font-semibold text-yellow-dark">
                          {item.matchRecord.phase ?? "Sem fase"}
                        </span>
                        <Button size="sm" variant="outline" onClick={() => handleEditMatch(item.matchRecord.id)}>
                          Editar
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          className="border-danger/50 text-danger hover:border-danger"
                          onClick={() => handleDeleteMatch(item)}
                          disabled={deleteMatch.isPending}
                        >
                          Excluir
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </Card>

        <div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr] [&>*]:min-w-0">
          <Card>
            <div className="grid gap-3 md:grid-cols-3">
              <div>
                <Label>Time</Label>
                <Input
                  value={matchFilters.team}
                  onChange={(e) => setMatchFilters((v) => ({ ...v, team: e.target.value }))}
                  placeholder="Buscar seleção..."
                />
              </div>
              <div>
                <Label>Fase</Label>
                <Select value={matchFilters.phase} onChange={(e) => setMatchFilters((v) => ({ ...v, phase: e.target.value }))}>
                  <option value="">Todas</option>
                  {phaseOptions.map((phase) => (
                    <option key={phase} value={phase}>
                      {phase}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Grupo</Label>
                <Select value={matchFilters.groupName} onChange={(e) => setMatchFilters((v) => ({ ...v, groupName: e.target.value }))}>
                  <option value="">Todos</option>
                  {groupOptions.map((group) => (
                    <option key={group} value={group}>
                      {group}
                    </option>
                  ))}
                </Select>
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
                  <option value="finished_pending">Pendente (sugestão)</option>
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

            <div className="mt-4 flex flex-wrap items-center justify-between gap-3">
              <span className="text-sm text-ink-muted">
                {visibleMatches.length} jogo(s)
                {hasActiveMatchFilters ? " (filtrado)" : ""}
              </span>
              {hasActiveMatchFilters && (
                <Button size="sm" variant="outline" onClick={() => setMatchFilters(emptyMatchFilters)}>
                  Limpar filtros
                </Button>
              )}
            </div>

            <div className="mt-3 space-y-3">
              {visibleMatches.length === 0 && (
                <p className="rounded-xl border border-mint/15 bg-card/70 px-4 py-6 text-center text-sm text-ink-muted">
                  Nenhum jogo encontrado com esses filtros.
                </p>
              )}
              {visibleMatches.map((item, index) => (
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
                        {formatKickoff(item.matchRecord.kickoff)} · {item.matchRecord.phase ?? "Sem fase"} · {adminStatusLabel(item.adminStatus)}
                      </p>
                      {item.adminStatus === "finished_pending" && (
                        <span className="mt-1 inline-block rounded-pill bg-yellow/20 px-3 py-0.5 text-xs font-semibold text-yellow-dark">
                          Sugestão pendente
                        </span>
                      )}
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

          <Card id="match-edit-panel">
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

                {selectedMatch.autoDetectedAt && selectedMatch.autoHomeScore != null && !selectedMatch.matchRecord.finished && (
                  <div className="mt-4 space-y-2 rounded-xl border border-yellow/40 bg-yellow/10 p-4">
                    <p className="text-sm font-semibold text-yellow-dark">
                      Sugestão da fonte externa (aguardando sua confirmação)
                    </p>
                    <p className="text-sm text-ink">
                      {selectedMatch.matchRecord.homeTeam} {selectedMatch.autoHomeScore} ×{" "}
                      {selectedMatch.autoAwayScore} {selectedMatch.matchRecord.awayTeam}
                      {selectedMatch.autoPenaltyHomeScore != null && (
                        <>
                          {" "}· pênaltis {selectedMatch.autoPenaltyHomeScore}×{selectedMatch.autoPenaltyAwayScore}
                        </>
                      )}
                      {selectedMatch.autoQualifier ? (
                        <>
                          {" "}· classificado:{" "}
                          {selectedMatch.autoQualifier === "home"
                            ? selectedMatch.matchRecord.homeTeam
                            : selectedMatch.matchRecord.awayTeam}
                        </>
                      ) : (
                        <> · classificado: indefinido (confira os pênaltis)</>
                      )}
                    </p>
                    <p className="text-xs text-ink-muted">
                      Status: {selectedMatch.autoStatus ?? "—"}. Revise e clique em Salvar resultado para oficializar e recalcular o ranking.
                    </p>
                    <Button size="sm" variant="outline" onClick={applySuggestion}>
                      Aplicar sugestão ao formulário
                    </Button>
                  </div>
                )}

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

                {isKnockout(selectedMatch.matchRecord.phase) &&
                  resultHome !== "" &&
                  resultAway !== "" &&
                  parseScore(resultHome) === parseScore(resultAway) && (
                    <div className="mt-4 space-y-2">
                      <p className="text-sm text-ink-muted">
                        Empate no tempo normal → decidido nos pênaltis (quem fizer mais se classifica).
                      </p>
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
                    </div>
                  )}

                <div className="mt-5 flex flex-wrap gap-2">
                  <Button onClick={handleSaveResult}>Salvar resultado</Button>
                  <Button variant="outline" onClick={() => runAdminAction(() => recalcMatch.mutateAsync(selectedMatch.matchRecord.id))}>
                    Recalcular este jogo
                  </Button>
                </div>

                {isKnockout(selectedMatch.matchRecord.phase) && (
                  <div className="mt-6 space-y-3 rounded-xl border border-mint/15 bg-card/60 p-4">
                    <h3 className="text-lg">Confronto e horário</h3>
                    <div className="grid gap-3 sm:grid-cols-2">
                      <div>
                        <Label>Time mandante</Label>
                        <TeamSelect value={editHome} onChange={setEditHome} ariaLabel="Seleção mandante" />
                      </div>
                      <div>
                        <Label>Time visitante</Label>
                        <TeamSelect value={editAway} onChange={setEditAway} ariaLabel="Seleção visitante" />
                      </div>
                      <div>
                        <Label>Fase</Label>
                        <Select value={editPhase} onChange={(e) => setEditPhase(e.target.value)}>
                          {KNOCKOUT_PHASES.map((phase) => (
                            <option key={phase} value={phase}>
                              {phase}
                            </option>
                          ))}
                        </Select>
                      </div>
                      <div>
                        <Label>Data e horário</Label>
                        <Input type="datetime-local" value={editKickoff} onChange={(e) => setEditKickoff(e.target.value)} />
                      </div>
                    </div>
                    {scheduleError && <ErrorBanner>{scheduleError}</ErrorBanner>}
                    <div className="flex flex-wrap gap-2">
                      <Button variant="outline" onClick={handleUpdateSchedule} disabled={updateMatchSchedule.isPending}>
                        {updateMatchSchedule.isPending ? "Salvando..." : "Salvar confronto/horário"}
                      </Button>
                      <Button
                        variant="outline"
                        className="border-danger/50 text-danger hover:border-danger"
                        onClick={() => handleDeleteMatch()}
                        disabled={deleteMatch.isPending}
                      >
                        Excluir jogo
                      </Button>
                    </div>
                  </div>
                )}

                <div className="mt-6 space-y-3 rounded-xl border border-mint/15 bg-card/60 p-4">
                  <div>
                    <h3 className="text-lg">Sincronização ao vivo</h3>
                    <p className="mt-1 text-sm text-ink-muted">
                      Cole o ID do evento no provedor de placares para o jogo puxar o placar ao
                      vivo automaticamente. Sem ID, o jogo não é sincronizado.
                    </p>
                  </div>
                  <div className="grid gap-3 sm:grid-cols-2">
                    <div>
                      <Label>ID do evento externo</Label>
                      <Input
                        inputMode="numeric"
                        placeholder="ex. 760500 (vazio = não sincronizar)"
                        value={editFixtureId}
                        onChange={(e) => {
                          setEditFixtureId(e.target.value.replace(/\D+/g, ""));
                          setFixtureCheckMsg("");
                        }}
                      />
                    </div>
                    <div className="flex items-end">
                      <p className="text-sm text-ink-muted">
                        {selectedMatch.matchRecord.liveStatus
                          ? `Ao vivo: ${selectedMatch.matchRecord.liveHomeScore ?? 0} x ${
                              selectedMatch.matchRecord.liveAwayScore ?? 0
                            } · ${selectedMatch.matchRecord.liveStatus}`
                          : selectedMatch.externalFixtureId != null
                            ? `Mapeado: ID ${selectedMatch.externalFixtureId}. Aguardando a janela do jogo para sincronizar.`
                            : "Sem mapeamento."}
                      </p>
                    </div>
                  </div>
                  {isKnockout(selectedMatch.matchRecord.phase) && (
                    <p className="text-xs text-ink-muted">
                      No mata-mata, a sincronização fecha automaticamente quando a fonte traz
                      classificado/pênaltis completos. Se houver conflito, a sugestão fica para
                      revisão manual acima.
                    </p>
                  )}
                  {fixtureError && <ErrorBanner>{fixtureError}</ErrorBanner>}
                  <div className="flex flex-wrap items-center gap-3">
                    <Button variant="outline" onClick={handleSaveFixture} disabled={setMatchFixture.isPending}>
                      {setMatchFixture.isPending ? "Salvando..." : "Salvar ID do evento"}
                    </Button>
                    <Button variant="outline" onClick={handleCheckFixture} disabled={checkFixture.isPending}>
                      {checkFixture.isPending ? "Checando..." : "Checar ID"}
                    </Button>
                    {fixtureSuccess && (
                      <span className="flex items-center gap-2 text-sm font-semibold text-mint-dark">
                        <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                        {fixtureSuccess}
                      </span>
                    )}
                  </div>
                  {fixtureCheckMsg && (
                    <p className="flex items-center gap-2 text-sm font-semibold text-mint-dark">
                      <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                      {fixtureCheckMsg}
                    </p>
                  )}
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
                    <div key={`${row.poolId}-${row.userId}-${row.matchId}`} className="flex items-center justify-between gap-3 rounded-xl border border-mint/10 bg-card px-3 py-3">
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
