import { formatKickoff, formatKnockoutPhase, isKnockout } from "@/lib/utils";
import { formatDateInput, formatTimeInput, KNOCKOUT_PHASES } from "@/components/admin/fixtureValidation";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { TeamSelect } from "@/components/admin/TeamSelect";
import type { AdminMatchesPanelProps } from "./types";
import { parseScore } from "./utils";

export function MatchEditor(props: AdminMatchesPanelProps) {
  return (
<Card id="match-edit-panel">
            {props.selectedMatch ? (
              <>
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h2 className="text-2xl">
                      {props.selectedMatch.matchRecord.homeTeam} x {props.selectedMatch.matchRecord.awayTeam}
                    </h2>
                    <p className="mt-1 text-sm text-ink-muted">
                      {props.selectedMatch.matchRecord.groupName ?? "Sem grupo"} · {formatKickoff(props.selectedMatch.matchRecord.kickoff)}
                    </p>
                  </div>
                  <Button variant={props.selectedMatch.matchRecord.finished ? "secondary" : "outline"} onClick={props.onToggleFinished}>
                    {props.selectedMatch.matchRecord.finished ? "Marcar como não finalizado" : "Marcar finalizado"}
                  </Button>
                </div>

                <div className="mt-4 grid gap-3 sm:grid-cols-2">
                  <div>
                    <Label>Placar mandante</Label>
                    <Input value={props.resultHome} onChange={(e) => props.setResultHome(e.target.value.replace(/\D+/g, ""))} />
                  </div>
                  <div>
                    <Label>Placar visitante</Label>
                    <Input value={props.resultAway} onChange={(e) => props.setResultAway(e.target.value.replace(/\D+/g, ""))} />
                  </div>
                </div>

                {isKnockout(props.selectedMatch.matchRecord.phase) &&
                  props.resultHome !== "" &&
                  props.resultAway !== "" &&
                  parseScore(props.resultHome) === parseScore(props.resultAway) && (
                    <div className="mt-4 space-y-2">
                      <p className="text-sm text-ink-muted">
                        Empate no tempo normal → decidido nos pênaltis (quem fizer mais se classifica).
                      </p>
                      <div className="grid gap-3 sm:grid-cols-2">
                        <div>
                          <Label>Pênaltis mandante</Label>
                          <Input value={props.penHome} onChange={(e) => props.setPenHome(e.target.value.replace(/\D+/g, ""))} />
                        </div>
                        <div>
                          <Label>Pênaltis visitante</Label>
                          <Input value={props.penAway} onChange={(e) => props.setPenAway(e.target.value.replace(/\D+/g, ""))} />
                        </div>
                      </div>
                    </div>
                  )}

                <div className="mt-5 flex flex-wrap gap-2">
                  <Button onClick={props.onSaveResult}>Salvar resultado</Button>
                  <Button variant="outline" onClick={() => props.onRecalculate()}>
                    Recalcular este jogo
                  </Button>
                </div>

                {isKnockout(props.selectedMatch.matchRecord.phase) && (
                  <div className="mt-6 space-y-3 rounded-xl border border-mint/15 bg-card/60 p-4">
                    <h3 className="text-lg">Confronto e horário</h3>
                    <div className="grid gap-3 sm:grid-cols-2">
                      <div>
                        <Label>Time mandante</Label>
                        <TeamSelect
                          value={props.editHome}
                          onChange={(value) => {
                            props.setEditHome(value);
                          }}
                          ariaLabel="Seleção mandante"
                        />
                      </div>
                      <div>
                        <Label>Time visitante</Label>
                        <TeamSelect
                          value={props.editAway}
                          onChange={(value) => {
                            props.setEditAway(value);
                          }}
                          ariaLabel="Seleção visitante"
                        />
                      </div>
                      <div>
                        <Label>Fase</Label>
                        <Select value={props.editPhase} onChange={(e) => props.setEditPhase(e.target.value)}>
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
                          value={props.editMatchDate}
                          onChange={(e) => {
                            props.setEditMatchDate(formatDateInput(e.target.value));
                          }}
                        />
                      </div>
                      <div>
                        <Label>Horário</Label>
                        <Input
                          inputMode="numeric"
                          placeholder="HH:mm"
                          value={props.editMatchTime}
                          onChange={(e) => {
                            props.setEditMatchTime(formatTimeInput(e.target.value));
                          }}
                        />
                      </div>
                    </div>
                    {props.scheduleError && <ErrorBanner>{props.scheduleError}</ErrorBanner>}
                    <div className="flex flex-wrap gap-2">
                      <Button variant="outline" onClick={props.onUpdateSchedule} disabled={props.updateSchedulePending}>
                        {props.updateSchedulePending ? "Salvando..." : "Salvar confronto/horário"}
                      </Button>
                      <Button
                        variant="outline"
                        className="border-danger/50 text-danger hover:border-danger"
                        onClick={() => props.onDeleteMatch()}
                        disabled={props.deleteMatchPending}
                      >
                        Excluir jogo
                      </Button>
                    </div>
                  </div>
                )}

                <div className="mt-6">
                  <h3 className="text-lg">Auditoria deste jogo</h3>
                  <div className="mt-3 space-y-2">
                    {props.auditEntries?.map((entry) => (
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
  );
}
