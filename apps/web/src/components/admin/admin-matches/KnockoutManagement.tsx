import { CheckCircle2, Eye, EyeOff, Trophy } from "lucide-react";
import { formatKnockoutPhase } from "@/lib/utils";
import { formatDateInput, formatTimeInput, KNOCKOUT_PHASES } from "@/components/admin/fixtureValidation";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { TeamSelect } from "@/components/admin/TeamSelect";
import type { AdminMatchesPanelProps } from "./types";

export function KnockoutManagement(props: AdminMatchesPanelProps) {
  return (
<Card className="border-l-4 border-yellow-dark p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <Trophy className="h-5 w-5 shrink-0 text-yellow-dark" />
                <h2 className="text-lg">Mata-mata</h2>
                <span className="text-sm text-ink-muted">
                  {props.knockoutMatches.length} confronto(s)
                </span>
              </div>
              <span
                className={`inline-flex items-center gap-1.5 rounded-pill px-3 py-1 text-xs font-semibold ring-1 ${
                  props.knockoutReleased
                    ? "bg-success/15 text-mint-dark ring-success/40"
                    : "bg-yellow/15 text-yellow-dark ring-yellow-dark/40"
                }`}
              >
                {props.knockoutReleased ? <Eye className="h-3.5 w-3.5" /> : <EyeOff className="h-3.5 w-3.5" />}
                {props.knockoutReleased ? "Liberado" : "Oculto"}
              </span>
            </div>

            <div className="mt-3 flex flex-wrap items-center gap-2">
              <Button
                variant={props.knockoutReleased ? "outline" : "primary"}
                size="sm"
                disabled={props.setKnockoutReleasedPending || props.knockoutReleasedLoading}
                onClick={props.onToggleKnockout}
              >
                {props.setKnockoutReleasedPending
                  ? "Salvando..."
                  : props.knockoutReleased
                    ? "Ocultar mata-mata"
                    : "Liberar mata-mata"}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  props.setCreateMatchError("");
                  props.setShowCreateMatchForm((value) => !value);
                }}
              >
                {props.showCreateMatchForm ? "Fechar cadastro" : "Adicionar confronto"}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => props.setMatchFilters((value) => ({ ...value, type: "knockout", phase: "", groupName: "" }))}
              >
                Filtrar mata-mata
              </Button>
            </div>

            {props.knockoutToggleMsg && (
              <p className="mt-3 flex items-center gap-2 text-sm font-semibold text-mint-dark">
                <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                {props.knockoutToggleMsg}
              </p>
            )}
            {props.createMatchSuccess && !props.showCreateMatchForm && (
              <p className="mt-3 flex items-center gap-2 text-sm font-semibold text-mint-dark">
                <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                {props.createMatchSuccess}
              </p>
            )}

            {props.showCreateMatchForm && (
              <div className="mt-4 border-t border-mint/15 pt-4">
                <h3 className="text-base">Adicionar confronto</h3>
                <div className="mt-4 grid gap-3 md:grid-cols-5">
                  <div>
                    <Label>Mandante</Label>
                    <TeamSelect value={props.newMatchHome} onChange={props.setNewMatchHome} ariaLabel="Seleção mandante" />
                  </div>
                  <div>
                    <Label>Visitante</Label>
                    <TeamSelect value={props.newMatchAway} onChange={props.setNewMatchAway} ariaLabel="Seleção visitante" />
                  </div>
                  <div>
                    <Label>Fase</Label>
                    <Select value={props.newMatchPhase} onChange={(e) => props.setNewMatchPhase(e.target.value)}>
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
                      value={props.newMatchDate}
                      onChange={(e) => props.setNewMatchDate(formatDateInput(e.target.value))}
                    />
                  </div>
                  <div>
                    <Label>Horário</Label>
                    <Input
                      inputMode="numeric"
                      placeholder="HH:mm"
                      value={props.newMatchTime}
                      onChange={(e) => props.setNewMatchTime(formatTimeInput(e.target.value))}
                    />
                  </div>
                </div>
                {props.createMatchError && <div className="mt-3"><ErrorBanner>{props.createMatchError}</ErrorBanner></div>}
                <div className="mt-4 flex flex-wrap items-center gap-3">
                  <Button onClick={props.onCreateMatch} disabled={props.createMatchPending}>
                    {props.createMatchPending ? "Criando..." : "Adicionar ao mata-mata"}
                  </Button>
                  {props.createMatchSuccess && (
                    <span className="flex items-center gap-2 text-sm font-semibold text-mint-dark">
                      <CheckCircle2 className="h-4 w-4" strokeWidth={2.5} />
                      {props.createMatchSuccess}
                    </span>
                  )}
                </div>
              </div>
            )}
          </Card>

        
  );
}
