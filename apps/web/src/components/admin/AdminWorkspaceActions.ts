// @ts-nocheck
import { brasiliaInputToIso } from "@/components/admin/fixtureValidation";
import { formatSelectionLabel } from "@/lib/selections";
import { isKnockout } from "@/lib/utils";

export function createAdminWorkspaceActions(context: Record<string, any>) {
  const {
    selectedUser, setPushSuccess, setError, runAdminAction, sendPushToUser, sendPushBroadcast,
    selectedMatch, resultHome, resultAway, penHome, penAway, setMatchResult,
    setCreateMatchError, setCreateMatchSuccess, newMatchHome, newMatchAway, newMatchDate,
    newMatchTime, newMatchPhase, createMatch, setNewMatchHome, setNewMatchAway, setNewMatchDate,
    setNewMatchTime, setShowCreateMatchForm, knockoutReleased, setKnockoutToggleMsg,
    setKnockoutReleased, finishEvent, deleteEvent, setScheduleError, editHome, editAway,
    editMatchDate, editMatchTime, updateMatchSchedule, editPhase, deleteMatch, selectedMatchId,
    setSelectedMatchId, setMatchFinished, overrideExpiry, overrideReason, reopenPrediction,
    parseScore,
  } = context;

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


  return {
    adminPushPending,
    handleSendPushToSelectedUser,
    handleSendPushBroadcast,
    handleSaveResult,
    handleCreateMatch,
    handleToggleKnockout,
    handleFinishEvent,
    handleDeleteEvent,
    handleUpdateSchedule,
    handleDeleteMatch,
    handleToggleFinished,
    handleReopenPrediction,
  };
}
