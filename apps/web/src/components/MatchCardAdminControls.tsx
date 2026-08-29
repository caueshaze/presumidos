import { formatSelectionLabel } from "@/lib/selections";
import { Button } from "./ui/button";
import { ErrorBanner, Label, Select } from "./ui/field";
import { PenaltyScorePanel, ScoreBox, ScoreInputs, normalizeScoreField, scoreValue } from "./MatchCardShared";

export function MatchCardAdminControls(props: any) {
  const {
    isAdmin, showAdminControls, onSaveTeams, teamHome, setTeamHome, teamAway, setTeamAway,
    teamSelectionFallbacks, selectionGroups, teamsError, updateTeams, onSaveResult, knockout,
    resultHome, setResultHome, resultAway, setResultAway, resultPenHome, setResultPenHome,
    resultPenAway, setResultPenAway, resultError, setResult, game, setFinished, onToggleFinished,
  } = props;

  return <>{isAdmin && showAdminControls && (
        <div className="mt-5 space-y-5 border-t border-mint/30 pt-4">
          <form onSubmit={onSaveTeams} className="flex flex-col gap-2">
            <h4 className="font-heading font-semibold">Admin: montar confronto</h4>
            <ScoreInputs>
              <Select value={teamHome} onChange={(e) => setTeamHome(e.target.value)}>
                {teamSelectionFallbacks.map((team: string) => (
                  <option key={`fallback-home-${team}`} value={team}>
                    {formatSelectionLabel(team)}
                  </option>
                ))}
                <optgroup label="Seleções">
                  {selectionGroups.teams.map((selection: any) => (
                    <option key={selection.key} value={selection.name}>
                      {formatSelectionLabel(selection.name)}
                    </option>
                  ))}
                </optgroup>
                <optgroup label="Placeholders">
                  {selectionGroups.placeholders.map((selection: any) => (
                    <option key={selection.key} value={selection.name}>
                      {formatSelectionLabel(selection.name)}
                    </option>
                  ))}
                </optgroup>
              </Select>
              <span className="font-heading font-bold text-ink-muted">x</span>
              <Select value={teamAway} onChange={(e) => setTeamAway(e.target.value)}>
                {teamSelectionFallbacks.map((team: string) => (
                  <option key={`fallback-away-${team}`} value={team}>
                    {formatSelectionLabel(team)}
                  </option>
                ))}
                <optgroup label="Seleções">
                  {selectionGroups.teams.map((selection: any) => (
                    <option key={selection.key} value={selection.name}>
                      {formatSelectionLabel(selection.name)}
                    </option>
                  ))}
                </optgroup>
                <optgroup label="Placeholders">
                  {selectionGroups.placeholders.map((selection: any) => (
                    <option key={selection.key} value={selection.name}>
                      {formatSelectionLabel(selection.name)}
                    </option>
                  ))}
                </optgroup>
              </Select>
            </ScoreInputs>
            {teamsError && <ErrorBanner>{teamsError}</ErrorBanner>}
            <Button type="submit" variant="outline" disabled={updateTeams.isPending} className="self-start">
              {updateTeams.isPending ? "Salvando..." : "Salvar confronto"}
            </Button>
          </form>

          <form onSubmit={onSaveResult} className="flex flex-col gap-2">
            <h4 className="font-heading font-semibold">Admin: lançar resultado oficial</h4>
            {knockout && <Label>Resultado no tempo normal</Label>}
            <ScoreInputs>
              <ScoreBox
              value={resultHome ?? 0}
              onChange={(e) => setResultHome(normalizeScoreField(e.target.value))}
              />
              <span className="font-heading text-xl font-bold text-ink-muted">x</span>
              <ScoreBox
                value={resultAway}
                onChange={(e) => setResultAway(normalizeScoreField(e.target.value))}
              />
            </ScoreInputs>

            {knockout && scoreValue(resultHome) === scoreValue(resultAway) && (
              <PenaltyScorePanel
                note="Empate no tempo normal: informe o placar dos pênaltis (quem fez mais se classifica)."
              >
                <ScoreBox
                  value={resultPenHome}
                  onChange={(e) => setResultPenHome(normalizeScoreField(e.target.value))}
                />
                <span className="font-heading text-xl font-bold text-ink-muted">x</span>
                <ScoreBox
                  value={resultPenAway}
                  onChange={(e) => setResultPenAway(normalizeScoreField(e.target.value))}
                />
              </PenaltyScorePanel>
            )}

            {resultError && <ErrorBanner>{resultError}</ErrorBanner>}
            <div className="flex flex-wrap items-center gap-3">
              <Button
                type="submit"
                variant="outline"
                disabled={setResult.isPending}
                className="self-start"
              >
                {setResult.isPending ? "Salvando..." : "Salvar resultado"}
              </Button>
              <Button
                type="button"
                variant={game.finished ? "secondary" : "outline"}
                disabled={setFinished.isPending}
                onClick={onToggleFinished}
                className="self-start"
              >
                {setFinished.isPending
                  ? "Atualizando..."
                  : game.finished
                    ? "Marcar como em aberto"
                    : "Marcar como finalizado"}
              </Button>
            </div>
            <p className="text-xs text-ink-muted">
              O ranking já atualiza quando o placar oficial é salvo. Esse toggle só controla o
              estado visual de jogo encerrado.
            </p>
          </form>
        </div>
      )}
</>;
}
