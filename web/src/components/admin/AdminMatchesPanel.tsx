import { type Dispatch, type SetStateAction } from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  CheckCircle2,
  Eye,
  EyeOff,
  Trophy,
} from "lucide-react";
import { formatKickoff, formatKnockoutPhase, isKnockout } from "@/lib/utils";
import { formatSelectionLabel } from "@/lib/selections";
import {
  FixtureCheckState,
  formatDateInput,
  formatTimeInput,
  KNOCKOUT_PHASES,
} from "@/components/admin/fixtureValidation";
import { emptyAdminMatchFilters } from "@/hooks/useAdminMatchFilters";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { TeamSelect } from "@/components/admin/TeamSelect";
import type { AdminMatchRecord, AuditLogEntry } from "@/types";
import type { AdminMatchFilters } from "@/hooks/useAdminMatchFilters";

type Setter<T> = Dispatch<SetStateAction<T>>;

function parseScore(value: string) {
  return value.trim() === "" ? 0 : Number.parseInt(value, 10) || 0;
}

function adminStatusLabel(status: string): string {
  switch (status) {
    case "scheduled": return "agendado";
    case "live": return "ao vivo";
    case "finished_pending": return "pendente de confirmação";
    case "finalized": return "finalizado";
    default: return status;
  }
}

type Props = {
  knockoutMatches: AdminMatchRecord[];
  knockoutReleased: boolean;
  setKnockoutReleasedPending: boolean;
  knockoutReleasedLoading: boolean;
  onToggleKnockout: () => void | Promise<void>;
  matchFilters: AdminMatchFilters;
  setMatchFilters: Setter<AdminMatchFilters>;
  phaseOptions: string[];
  groupOptions: string[];
  visibleMatches: AdminMatchRecord[];
  hasActiveMatchFilters: boolean;
  selectedMatchId: string;
  setSelectedMatchId: Setter<string>;
  selectedMatch: AdminMatchRecord | null;
  auditEntries: AuditLogEntry[] | undefined;
  resultHome: string;
  setResultHome: Setter<string>;
  resultAway: string;
  setResultAway: Setter<string>;
  penHome: string;
  setPenHome: Setter<string>;
  penAway: string;
  setPenAway: Setter<string>;
  newMatchHome: string;
  setNewMatchHome: Setter<string>;
  newMatchAway: string;
  setNewMatchAway: Setter<string>;
  newMatchPhase: string;
  setNewMatchPhase: Setter<string>;
  newMatchDate: string;
  setNewMatchDate: Setter<string>;
  newMatchTime: string;
  setNewMatchTime: Setter<string>;
  createMatchError: string;
  createMatchSuccess: string;
  showCreateMatchForm: boolean;
  setShowCreateMatchForm: Setter<boolean>;
  setCreateMatchError: Setter<string>;
  knockoutToggleMsg: string;
  editHome: string;
  setEditHome: Setter<string>;
  editAway: string;
  setEditAway: Setter<string>;
  editPhase: string;
  setEditPhase: Setter<string>;
  editMatchDate: string;
  setEditMatchDate: Setter<string>;
  editMatchTime: string;
  setEditMatchTime: Setter<string>;
  scheduleError: string;
  editFixtureId: string;
  setEditFixtureId: Setter<string>;
  fixtureError: string;
  fixtureSuccess: string;
  fixtureCheckState: FixtureCheckState | null;
  setFixtureCheckState: Setter<FixtureCheckState | null>;
  onCreateMatch: () => void | Promise<void>;
  createMatchPending: boolean;
  onToggleFinished: () => void | Promise<void>;
  onApplySuggestion: () => void;
  onSaveResult: () => void | Promise<void>;
  onRecalculate: () => void | Promise<unknown>;
  onUpdateSchedule: () => void | Promise<void>;
  updateSchedulePending: boolean;
  onDeleteMatch: () => void | Promise<void>;
  deleteMatchPending: boolean;
  onSaveFixture: () => void | Promise<void>;
  setFixturePending: boolean;
  onCheckFixture: () => void | Promise<void>;
  checkFixturePending: boolean;
};

export function AdminMatchesPanel({
  knockoutMatches,
  knockoutReleased,
  setKnockoutReleasedPending,
  knockoutReleasedLoading,
  onToggleKnockout,
  matchFilters,
  setMatchFilters,
  phaseOptions,
  groupOptions,
  visibleMatches,
  hasActiveMatchFilters,
  selectedMatchId,
  setSelectedMatchId,
  selectedMatch,
  auditEntries,
  resultHome,
  setResultHome,
  resultAway,
  setResultAway,
  penHome,
  setPenHome,
  penAway,
  setPenAway,
  newMatchHome,
  setNewMatchHome,
  newMatchAway,
  setNewMatchAway,
  newMatchPhase,
  setNewMatchPhase,
  newMatchDate,
  setNewMatchDate,
  newMatchTime,
  setNewMatchTime,
  createMatchError,
  createMatchSuccess,
  showCreateMatchForm,
  setShowCreateMatchForm,
  setCreateMatchError,
  knockoutToggleMsg,
  editHome,
  setEditHome,
  editAway,
  setEditAway,
  editPhase,
  setEditPhase,
  editMatchDate,
  setEditMatchDate,
  editMatchTime,
  setEditMatchTime,
  scheduleError,
  editFixtureId,
  setEditFixtureId,
  fixtureError,
  fixtureSuccess,
  fixtureCheckState,
  setFixtureCheckState,
  onCreateMatch,
  createMatchPending,
  onToggleFinished,
  onApplySuggestion,
  onSaveResult,
  onRecalculate,
  onUpdateSchedule,
  updateSchedulePending,
  onDeleteMatch,
  deleteMatchPending,
  onSaveFixture,
  setFixturePending,
  onCheckFixture,
  checkFixturePending,
}: Props) {
  return (

        <div className="mt-6 space-y-5">
          <Card className="border-l-4 border-yellow-dark p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <Trophy className="h-5 w-5 shrink-0 text-yellow-dark" />
                <h2 className="text-lg">Mata-mata</h2>
                <span className="text-sm text-ink-muted">
                  {knockoutMatches.length} confronto(s)
                </span>
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

            <div className="mt-3 flex flex-wrap items-center gap-2">
              <Button
                variant={knockoutReleased ? "outline" : "primary"}
                size="sm"
                disabled={setKnockoutReleasedPending || knockoutReleasedLoading}
                onClick={onToggleKnockout}
              >
                {setKnockoutReleasedPending
                  ? "Salvando..."
                  : knockoutReleased
                    ? "Ocultar mata-mata"
                    : "Liberar mata-mata"}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  setCreateMatchError("");
                  setShowCreateMatchForm((value) => !value);
                }}
              >
                {showCreateMatchForm ? "Fechar cadastro" : "Adicionar confronto"}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setMatchFilters((value) => ({ ...value, type: "knockout", phase: "", groupName: "" }))}
              >
                Filtrar mata-mata
              </Button>
            </div>

            {knockoutToggleMsg && (
              <p className="mt-3 flex items-center gap-2 text-sm font-semibold text-mint-dark">
                <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                {knockoutToggleMsg}
              </p>
            )}
            {createMatchSuccess && !showCreateMatchForm && (
              <p className="mt-3 flex items-center gap-2 text-sm font-semibold text-mint-dark">
                <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                {createMatchSuccess}
              </p>
            )}

            {showCreateMatchForm && (
              <div className="mt-4 border-t border-mint/15 pt-4">
                <h3 className="text-base">Adicionar confronto</h3>
                <div className="mt-4 grid gap-3 md:grid-cols-5">
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
                          {formatKnockoutPhase(phase)}
                        </option>
                      ))}
                    </Select>
                  </div>
                  <div>
                    <Label>Data</Label>
                    <Input
                      inputMode="numeric"
                      placeholder="DD/MM/AAAA"
                      value={newMatchDate}
                      onChange={(e) => setNewMatchDate(formatDateInput(e.target.value))}
                    />
                  </div>
                  <div>
                    <Label>Horário</Label>
                    <Input
                      inputMode="numeric"
                      placeholder="HH:mm"
                      value={newMatchTime}
                      onChange={(e) => setNewMatchTime(formatTimeInput(e.target.value))}
                    />
                  </div>
                </div>
                {createMatchError && <div className="mt-3"><ErrorBanner>{createMatchError}</ErrorBanner></div>}
                <div className="mt-4 flex flex-wrap items-center gap-3">
                  <Button onClick={onCreateMatch} disabled={createMatchPending}>
                    {createMatchPending ? "Criando..." : "Adicionar ao mata-mata"}
                  </Button>
                  {createMatchSuccess && (
                    <span className="flex items-center gap-2 text-sm font-semibold text-mint-dark">
                      <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                      {createMatchSuccess}
                    </span>
                  )}
                </div>
              </div>
            )}
          </Card>

        <div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr] [&>*]:min-w-0">
          <Card>
            <div className="grid gap-3 md:grid-cols-4">
              <div>
                <Label>Tipo</Label>
                <Select value={matchFilters.type} onChange={(e) => setMatchFilters((v) => ({ ...v, type: e.target.value }))}>
                  <option value="">Todos</option>
                  <option value="group">Fase de grupos</option>
                  <option value="knockout">Mata-mata</option>
                </Select>
              </div>
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
                      {formatKnockoutPhase(phase)}
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
                <Input
                  inputMode="numeric"
                  placeholder="DD/MM/AAAA"
                  value={matchFilters.date}
                  onChange={(e) => setMatchFilters((v) => ({ ...v, date: formatDateInput(e.target.value) }))}
                />
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
                <Button size="sm" variant="outline" onClick={() => setMatchFilters(emptyAdminMatchFilters)}>
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
                  className={`w-full rounded-xl border px-3 py-3 text-left transition ${selectedMatchId === item.matchRecord.id ? "border-mint-dark bg-mint/10 shadow-glow" : "border-mint/15 bg-card/70"}`}
                >
                  <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-2">
                    <div className="min-w-0">
                      <p className="truncate font-heading text-base text-ink">
                        {formatSelectionLabel(item.matchRecord.homeTeam)}{" "}
                        <span className="text-ink-muted">x</span>{" "}
                        {formatSelectionLabel(item.matchRecord.awayTeam)}
                      </p>
                      <div className="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-ink-muted">
                        <span>{formatKickoff(item.matchRecord.kickoff)}</span>
                        <span>·</span>
                        <span>{formatKnockoutPhase(item.matchRecord.phase)}</span>
                        <span>·</span>
                        <span>{adminStatusLabel(item.adminStatus)}</span>
                        {item.adminStatus === "finished_pending" && (
                          <span className="rounded-pill bg-yellow/20 px-2 py-0.5 font-semibold text-yellow-dark">
                            Sugestão
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="shrink-0 text-right text-sm">
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
                  <Button variant={selectedMatch.matchRecord.finished ? "secondary" : "outline"} onClick={onToggleFinished}>
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
                    <Button size="sm" variant="outline" onClick={onApplySuggestion}>
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
                  <Button onClick={onSaveResult}>Salvar resultado</Button>
                  <Button variant="outline" onClick={() => onRecalculate()}>
                    Recalcular este jogo
                  </Button>
                </div>

                {isKnockout(selectedMatch.matchRecord.phase) && (
                  <div className="mt-6 space-y-3 rounded-xl border border-mint/15 bg-card/60 p-4">
                    <h3 className="text-lg">Confronto e horário</h3>
                    <div className="grid gap-3 sm:grid-cols-2">
                      <div>
                        <Label>Time mandante</Label>
                        <TeamSelect
                          value={editHome}
                          onChange={(value) => {
                            setEditHome(value);
                            setFixtureCheckState(null);
                          }}
                          ariaLabel="Seleção mandante"
                        />
                      </div>
                      <div>
                        <Label>Time visitante</Label>
                        <TeamSelect
                          value={editAway}
                          onChange={(value) => {
                            setEditAway(value);
                            setFixtureCheckState(null);
                          }}
                          ariaLabel="Seleção visitante"
                        />
                      </div>
                      <div>
                        <Label>Fase</Label>
                        <Select value={editPhase} onChange={(e) => setEditPhase(e.target.value)}>
                          {KNOCKOUT_PHASES.map((phase) => (
                            <option key={phase} value={phase}>
                              {formatKnockoutPhase(phase)}
                            </option>
                          ))}
                        </Select>
                      </div>
                      <div>
                        <Label>Data</Label>
                        <Input
                          inputMode="numeric"
                          placeholder="DD/MM/AAAA"
                          value={editMatchDate}
                          onChange={(e) => {
                            setEditMatchDate(formatDateInput(e.target.value));
                            setFixtureCheckState(null);
                          }}
                        />
                      </div>
                      <div>
                        <Label>Horário</Label>
                        <Input
                          inputMode="numeric"
                          placeholder="HH:mm"
                          value={editMatchTime}
                          onChange={(e) => {
                            setEditMatchTime(formatTimeInput(e.target.value));
                            setFixtureCheckState(null);
                          }}
                        />
                      </div>
                    </div>
                    {scheduleError && <ErrorBanner>{scheduleError}</ErrorBanner>}
                    <div className="flex flex-wrap gap-2">
                      <Button variant="outline" onClick={onUpdateSchedule} disabled={updateSchedulePending}>
                        {updateSchedulePending ? "Salvando..." : "Salvar confronto/horário"}
                      </Button>
                      <Button
                        variant="outline"
                        className="border-danger/50 text-danger hover:border-danger"
                        onClick={() => onDeleteMatch()}
                        disabled={deleteMatchPending}
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
                          setFixtureCheckState(null);
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
                    <Button variant="outline" onClick={onSaveFixture} disabled={setFixturePending}>
                      {setFixturePending ? "Salvando..." : "Salvar ID do evento"}
                    </Button>
                    <Button variant="outline" onClick={onCheckFixture} disabled={checkFixturePending}>
                      {checkFixturePending ? "Checando..." : "Checar ID"}
                    </Button>
                    {fixtureSuccess && (
                      <span className="flex items-center gap-2 text-sm font-semibold text-mint-dark">
                        <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                        {fixtureSuccess}
                      </span>
                    )}
                  </div>
                  {fixtureCheckState && (
                    <p
                      className={`flex items-center gap-2 text-sm font-semibold ${
                        fixtureCheckState.ok ? "text-mint-dark" : "text-danger"
                      }`}
                    >
                      {fixtureCheckState.ok ? (
                        <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                      ) : (
                        <AlertTriangle className="h-4 w-4" strokeWidth={2.5} />
                      )}
                      {fixtureCheckState.message}
                    </p>
                  )}
                </div>

                <div className="mt-6">
                  <h3 className="text-lg">Auditoria deste jogo</h3>
                  <div className="mt-3 space-y-2">
                    {auditEntries?.map((entry) => (
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
  );
}
