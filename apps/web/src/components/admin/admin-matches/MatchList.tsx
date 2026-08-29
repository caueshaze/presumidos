import { motion } from "framer-motion";
import { formatKickoff, formatKnockoutPhase } from "@/lib/utils";
import { formatSelectionLabel } from "@/lib/selections";
import { formatDateInput } from "@/components/admin/fixtureValidation";
import { emptyAdminMatchFilters } from "@/hooks/useAdminMatchFilters";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import type { AdminMatchesPanelProps } from "./types";
import { adminStatusLabel } from "./utils";

export function MatchList(props: AdminMatchesPanelProps) {
  return (
<Card>
            <div className="grid gap-3 md:grid-cols-4">
              <div>
                <Label>Tipo</Label>
                <Select value={props.matchFilters.type} onChange={(e) => props.setMatchFilters((v) => ({ ...v, type: e.target.value }))}>
                  <option value="">Todos</option>
                  <option value="group">Fase de grupos</option>
                  <option value="knockout">Mata-mata</option>
                </Select>
              </div>
              <div>
                <Label>Time</Label>
                <Input
                  value={props.matchFilters.team}
                  onChange={(e) => props.setMatchFilters((v) => ({ ...v, team: e.target.value }))}
                  placeholder="Buscar seleção..."
                />
              </div>
              <div>
                <Label>Fase</Label>
                <Select value={props.matchFilters.phase} onChange={(e) => props.setMatchFilters((v) => ({ ...v, phase: e.target.value }))}>
                  <option value="">Todas</option>
                  {props.phaseOptions.map((phase) => (
                    <option key={phase} value={phase}>
                      {formatKnockoutPhase(phase)}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Grupo</Label>
                <Select value={props.matchFilters.groupName} onChange={(e) => props.setMatchFilters((v) => ({ ...v, groupName: e.target.value }))}>
                  <option value="">Todos</option>
                  {props.groupOptions.map((group) => (
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
                  value={props.matchFilters.date}
                  onChange={(e) => props.setMatchFilters((v) => ({ ...v, date: formatDateInput(e.target.value) }))}
                />
              </div>
              <div>
                <Label>Status</Label>
                <Select value={props.matchFilters.status} onChange={(e) => props.setMatchFilters((v) => ({ ...v, status: e.target.value }))}>
                  <option value="">Todos</option>
                  <option value="scheduled">Agendado</option>
                  <option value="live">Ao vivo</option>
                  <option value="finished_pending">Pendente (sugestão)</option>
                  <option value="finalized">Finalizado</option>
                </Select>
              </div>
              <div>
                <Label>Origem</Label>
                <Select value={props.matchFilters.origin} onChange={(e) => props.setMatchFilters((v) => ({ ...v, origin: e.target.value }))}>
                  <option value="">Todas</option>
                  <option value="api">Fonte externa</option>
                  <option value="manual">Manual</option>
                </Select>
              </div>
            </div>

            <div className="mt-4 flex flex-wrap items-center justify-between gap-3">
              <span className="text-sm text-ink-muted">
                {props.visibleMatches.length} jogo(s)
                {props.hasActiveMatchFilters ? " (filtrado)" : ""}
              </span>
              {props.hasActiveMatchFilters && (
                <Button size="sm" variant="outline" onClick={() => props.setMatchFilters(emptyAdminMatchFilters)}>
                  Limpar filtros
                </Button>
              )}
            </div>

            <div className="mt-3 space-y-3">
              {props.visibleMatches.length === 0 && (
                <p className="rounded-xl border border-mint/15 bg-card/70 px-4 py-6 text-center text-sm text-ink-muted">
                  Nenhum jogo encontrado com esses filtros.
                </p>
              )}
              {props.visibleMatches.map((item, index) => (
                <motion.button
                  key={item.matchRecord.id}
                  type="button"
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: Math.min(index * 0.02, 0.2) }}
                  onClick={() => props.setSelectedMatchId(item.matchRecord.id)}
                  className={`w-full rounded-xl border px-3 py-3 text-left transition ${props.selectedMatchId === item.matchRecord.id ? "border-mint-dark bg-mint/10 shadow-glow" : "border-mint/15 bg-card/70"}`}
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
                    </div>
                  </div>
                </motion.button>
              ))}
            </div>
          </Card>

          
  );
}
