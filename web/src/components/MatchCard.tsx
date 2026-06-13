import { useEffect, useState, type FormEvent } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2 } from "lucide-react";
import {
  useSubmitPrediction,
  useSetMatchResult,
  useUpdateMatchTeams,
  useSetMatchFinished,
  useReauth,
} from "@/hooks/queries";
import { withAdminReauth } from "@/lib/adminReauth";
import {
  formatSelectionLabel,
  getSelectionGroups,
  isKnownSelection,
} from "@/lib/selections";
import { cn, formatKickoff, isKnockout, winnerSide } from "@/lib/utils";
import type { KnockoutEntry, MatchRecord, PredictionRecord } from "@/types";
import { MotionCard } from "./ui/card";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label, Select, ErrorBanner } from "./ui/field";

function ScoreInputs({ children }: { children: React.ReactNode }) {
  return <div className="flex items-center gap-3">{children}</div>;
}

function ScoreBox(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <Input
      type="number"
      min={0}
      className="score-input w-20 text-center text-xl font-heading font-bold"
      {...props}
    />
  );
}

interface Props {
  game: MatchRecord;
  prediction?: PredictionRecord;
  locked: boolean;
  isAdmin: boolean;
  index: number;
}

export function MatchCard({ game, prediction, locked, isAdmin, index }: Props) {
  const knockout = isKnockout(game.phase);
  const selectionGroups = getSelectionGroups();

  const submit = useSubmitPrediction();
  const setResult = useSetMatchResult();
  const setFinished = useSetMatchFinished();
  const updateTeams = useUpdateMatchTeams();
  const reauth = useReauth();

  // ---- Palpite do usuário ----
  const initialHome = prediction?.homeScore ?? 0;
  const initialAway = prediction?.awayScore ?? 0;
  const [homeGuess, setHomeGuess] = useState(initialHome);
  const [awayGuess, setAwayGuess] = useState(initialAway);
  const [qualifier, setQualifier] = useState(
    prediction?.qualifier ?? winnerSide(initialHome, initialAway) ?? "home",
  );
  const [qualifierTouched, setQualifierTouched] = useState(!!prediction?.qualifier);
  const [wentPens, setWentPens] = useState(prediction?.wentToPenalties ?? false);
  const [penHome, setPenHome] = useState<number | null>(prediction?.penaltyHomeScore ?? null);
  const [penAway, setPenAway] = useState<number | null>(prediction?.penaltyAwayScore ?? null);
  const [savedMessage, setSavedMessage] = useState("");
  const [error, setError] = useState("");

  // A confirmação de "Palpite salvo!" some sozinha depois de alguns segundos.
  useEffect(() => {
    if (!savedMessage) return;
    const timer = setTimeout(() => setSavedMessage(""), 4000);
    return () => clearTimeout(timer);
  }, [savedMessage]);

  useEffect(() => {
    setResultHome(game.homeScore);
    setResultAway(game.awayScore);
    setResultQualifier(
      game.qualifier ?? winnerSide(game.homeScore ?? 0, game.awayScore ?? 0) ?? "home",
    );
    setResultQualTouched(!!game.qualifier);
    setResultPens(game.wentToPenalties);
    setResultPenHome(game.penaltyHomeScore);
    setResultPenAway(game.penaltyAwayScore);
  }, [
    game.homeScore,
    game.awayScore,
    game.qualifier,
    game.wentToPenalties,
    game.penaltyHomeScore,
    game.penaltyAwayScore,
  ]);

  useEffect(() => {
    setTeamHome(game.homeTeam);
    setTeamAway(game.awayTeam);
  }, [game.homeTeam, game.awayTeam]);

  // ---- Resultado oficial (admin) ----
  const [resultHome, setResultHome] = useState<number | null>(game.homeScore);
  const [resultAway, setResultAway] = useState<number | null>(game.awayScore);
  const [resultQualifier, setResultQualifier] = useState(
    game.qualifier ?? winnerSide(game.homeScore ?? 0, game.awayScore ?? 0) ?? "home",
  );
  const [resultQualTouched, setResultQualTouched] = useState(!!game.qualifier);
  const [resultPens, setResultPens] = useState(game.wentToPenalties);
  const [resultPenHome, setResultPenHome] = useState<number | null>(game.penaltyHomeScore);
  const [resultPenAway, setResultPenAway] = useState<number | null>(game.penaltyAwayScore);
  const [resultError, setResultError] = useState("");

  // ---- Confronto (admin) ----
  const [teamHome, setTeamHome] = useState(game.homeTeam);
  const [teamAway, setTeamAway] = useState(game.awayTeam);
  const [teamsError, setTeamsError] = useState("");

  const teamSelectionFallbacks = [teamHome, teamAway].filter(
    (team, position, allTeams) => allTeams.indexOf(team) === position && !isKnownSelection(team),
  );

  const buildKnockout = (home: number, away: number, q: string, pens: boolean, ph: number | null, pa: number | null): KnockoutEntry => {
    if (!knockout) return { qualifier: null, wentToPenalties: false, penaltyHome: null, penaltyAway: null };
    const wentPenalties = pens && home === away;
    return {
      qualifier: q,
      wentToPenalties: wentPenalties,
      penaltyHome: wentPenalties ? ph : null,
      penaltyAway: wentPenalties ? pa : null,
    };
  };

  const onSave = async (e: FormEvent) => {
    e.preventDefault();
    setSavedMessage("");
    setError("");
    try {
      await submit.mutateAsync({
        matchId: game.id,
        homeScore: homeGuess,
        awayScore: awayGuess,
        knockout: buildKnockout(homeGuess, awayGuess, qualifier, wentPens, penHome, penAway),
      });
      setSavedMessage("Palpite salvo!");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao salvar palpite.");
    }
  };

  const onSaveResult = async (e: FormEvent) => {
    e.preventDefault();
    setResultError("");
    const home = resultHome ?? 0;
    const away = resultAway ?? 0;
    try {
      await withAdminReauth(
        () =>
          setResult.mutateAsync({
            matchId: game.id,
            homeScore: home,
            awayScore: away,
            knockout: buildKnockout(home, away, resultQualifier, resultPens, resultPenHome, resultPenAway),
          }),
        (password) => reauth.mutateAsync(password),
      );
    } catch (err) {
      setResultError(err instanceof Error ? err.message : "Falha ao salvar resultado.");
    }
  };

  const onSaveTeams = async (e: FormEvent) => {
    e.preventDefault();
    setTeamsError("");
    try {
      await withAdminReauth(
        () => updateTeams.mutateAsync({ matchId: game.id, homeTeam: teamHome, awayTeam: teamAway }),
        (password) => reauth.mutateAsync(password),
      );
    } catch (err) {
      setTeamsError(err instanceof Error ? err.message : "Falha ao salvar confronto.");
    }
  };

  const onToggleFinished = async () => {
    setResultError("");
    try {
      await withAdminReauth(
        () => setFinished.mutateAsync({ matchId: game.id, finished: !game.finished }),
        (password) => reauth.mutateAsync(password),
      );
    } catch (err) {
      setResultError(err instanceof Error ? err.message : "Falha ao atualizar status do jogo.");
    }
  };

  // Contorno verde permanente sempre que já existe palpite para este jogo
  // (o `savedMessage` cobre o instante entre salvar e a lista revalidar).
  const hasPrediction = !!prediction || !!savedMessage;
  const hasOfficial = game.homeScore !== null && game.awayScore !== null;
  const qualifierSuffix =
    knockout && game.qualifier
      ? ` — ${formatSelectionLabel(game.qualifier === "home" ? game.homeTeam : game.awayTeam)} classificou`
      : "";
  const penaltyLabel =
    game.penaltyHomeScore !== null && game.penaltyAwayScore !== null
      ? `Pênaltis: ${game.penaltyHomeScore} x ${game.penaltyAwayScore}`
      : "Decidido nos pênaltis";

  return (
    <MotionCard
      className={cn(
        "mb-4 transition-shadow duration-500",
        hasPrediction && "ring-2 ring-success/60",
        savedMessage && "shadow-[0_0_0_6px_rgba(95,191,159,0.18)]",
      )}
      transition={{ delay: Math.min(index * 0.03, 0.3), duration: 0.28 }}
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="font-heading text-lg font-semibold">
          {formatSelectionLabel(game.homeTeam)} vs {formatSelectionLabel(game.awayTeam)}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {hasPrediction && (
            <span className="inline-flex items-center gap-1 rounded-pill bg-success/15 px-3 py-1 text-xs font-semibold text-mint-dark ring-1 ring-success/40">
              <CheckCircle2 className="h-3.5 w-3.5" strokeWidth={2.5} />
              Palpite salvo
            </span>
          )}
          {game.phase && (
            <span className="rounded-pill bg-sky/40 px-3 py-1 text-xs font-semibold">
              {game.phase}
            </span>
          )}
          {game.groupName && (
            <span className="rounded-pill bg-mint/40 px-3 py-1 text-xs font-semibold">
              Grupo {game.groupName}
            </span>
          )}
          {game.finished && (
            <span className="rounded-pill bg-success/15 px-3 py-1 text-xs font-semibold text-mint-dark ring-1 ring-success/35">
              Finalizado
            </span>
          )}
        </div>
      </div>
      <div className="mt-1 text-sm text-ink-muted">{formatKickoff(game.kickoff)}</div>

      {hasOfficial && (
        <p className="mt-2 font-semibold">
          Resultado oficial: {game.homeScore} x {game.awayScore}
          {qualifierSuffix}
        </p>
      )}
      {hasOfficial && knockout && game.wentToPenalties && (
        <p className="text-sm text-ink-muted">{penaltyLabel}</p>
      )}

      {/* Admin não palpita — vê apenas os controles administrativos abaixo. */}
      {!isAdmin &&
        (locked ? (
          <p className="mt-3 rounded-md bg-danger-bg px-3 py-2 text-sm font-semibold">
            Partida já iniciada — palpites encerrados.
          </p>
        ) : (
        <form onSubmit={onSave} className="mt-4 flex flex-col gap-3">
          {knockout && <Label>Placar no tempo normal</Label>}
          <ScoreInputs>
            <ScoreBox
              value={homeGuess}
              onChange={(e) => {
                const v = parseInt(e.target.value) || 0;
                setHomeGuess(v);
                if (knockout) {
                  if (!qualifierTouched) {
                    const w = winnerSide(v, awayGuess);
                    if (w) setQualifier(w);
                  }
                  if (v !== awayGuess) setWentPens(false);
                }
              }}
            />
            <span className="font-heading text-xl font-bold text-ink-muted">x</span>
            <ScoreBox
              value={awayGuess}
              onChange={(e) => {
                const v = parseInt(e.target.value) || 0;
                setAwayGuess(v);
                if (knockout) {
                  if (!qualifierTouched) {
                    const w = winnerSide(homeGuess, v);
                    if (w) setQualifier(w);
                  }
                  if (homeGuess !== v) setWentPens(false);
                }
              }}
            />
          </ScoreInputs>

          {knockout && (
            <div className="flex flex-col gap-2 rounded-md bg-mint/10 p-3">
              <Label>Quem se classifica</Label>
              <Select
                value={qualifier}
                onChange={(e) => {
                  setQualifier(e.target.value);
                  setQualifierTouched(true);
                }}
              >
                <option value="home">{formatSelectionLabel(game.homeTeam)}</option>
                <option value="away">{formatSelectionLabel(game.awayTeam)}</option>
              </Select>
              {homeGuess === awayGuess && (
                <>
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={wentPens}
                      onChange={(e) => setWentPens(e.target.checked)}
                    />
                    Foi para os pênaltis?
                  </label>
                  {wentPens && (
                    <>
                      <Label>Placar dos pênaltis (opcional)</Label>
                      <ScoreInputs>
                        <ScoreBox
                          value={penHome ?? 0}
                          onChange={(e) => setPenHome(parseInt(e.target.value) || 0)}
                        />
                        <span className="font-heading text-xl font-bold text-ink-muted">x</span>
                        <ScoreBox
                          value={penAway ?? 0}
                          onChange={(e) => setPenAway(parseInt(e.target.value) || 0)}
                        />
                      </ScoreInputs>
                    </>
                  )}
                </>
              )}
            </div>
          )}

          {error && <ErrorBanner>{error}</ErrorBanner>}
          <AnimatePresence>
            {savedMessage && (
              <motion.div
                key="saved"
                initial={{ opacity: 0, y: -6, scale: 0.96 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, scale: 0.96 }}
                transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
                className="flex items-center gap-2 rounded-md border border-success/40 bg-mint/30 px-4 py-2.5 font-heading font-semibold text-mint-dark"
              >
                <motion.span
                  initial={{ scale: 0, rotate: -30 }}
                  animate={{ scale: 1, rotate: 0 }}
                  transition={{ type: "spring", stiffness: 500, damping: 18, delay: 0.05 }}
                  className="flex"
                >
                  <CheckCircle2 className="h-5 w-5" strokeWidth={2.5} />
                </motion.span>
                {savedMessage}
              </motion.div>
            )}
          </AnimatePresence>
          <Button type="submit" disabled={submit.isPending} className="self-start">
            {submit.isPending ? "Salvando..." : savedMessage ? "Palpite salvo ✓" : "Salvar palpite"}
          </Button>
        </form>
        ))}

      {isAdmin && (
        <div className="mt-5 space-y-5 border-t border-mint/30 pt-4">
          <form onSubmit={onSaveTeams} className="flex flex-col gap-2">
            <h4 className="font-heading font-semibold">Admin: montar confronto</h4>
            <ScoreInputs>
              <Select value={teamHome} onChange={(e) => setTeamHome(e.target.value)}>
                {teamSelectionFallbacks.map((team) => (
                  <option key={`fallback-home-${team}`} value={team}>
                    {formatSelectionLabel(team)}
                  </option>
                ))}
                <optgroup label="Seleções">
                  {selectionGroups.teams.map((selection) => (
                    <option key={selection.key} value={selection.name}>
                      {formatSelectionLabel(selection.name)}
                    </option>
                  ))}
                </optgroup>
                <optgroup label="Placeholders">
                  {selectionGroups.placeholders.map((selection) => (
                    <option key={selection.key} value={selection.name}>
                      {formatSelectionLabel(selection.name)}
                    </option>
                  ))}
                </optgroup>
              </Select>
              <span className="font-heading font-bold text-ink-muted">x</span>
              <Select value={teamAway} onChange={(e) => setTeamAway(e.target.value)}>
                {teamSelectionFallbacks.map((team) => (
                  <option key={`fallback-away-${team}`} value={team}>
                    {formatSelectionLabel(team)}
                  </option>
                ))}
                <optgroup label="Seleções">
                  {selectionGroups.teams.map((selection) => (
                    <option key={selection.key} value={selection.name}>
                      {formatSelectionLabel(selection.name)}
                    </option>
                  ))}
                </optgroup>
                <optgroup label="Placeholders">
                  {selectionGroups.placeholders.map((selection) => (
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
                onChange={(e) => {
                  const v = e.target.value === "" ? null : parseInt(e.target.value) || 0;
                  setResultHome(v);
                  if (knockout) {
                    const h = v ?? 0;
                    const a = resultAway ?? 0;
                    if (!resultQualTouched) {
                      const w = winnerSide(h, a);
                      if (w) setResultQualifier(w);
                    }
                    if (h !== a) setResultPens(false);
                  }
                }}
              />
              <span className="font-heading text-xl font-bold text-ink-muted">x</span>
              <ScoreBox
                value={resultAway ?? 0}
                onChange={(e) => {
                  const v = e.target.value === "" ? null : parseInt(e.target.value) || 0;
                  setResultAway(v);
                  if (knockout) {
                    const h = resultHome ?? 0;
                    const a = v ?? 0;
                    if (!resultQualTouched) {
                      const w = winnerSide(h, a);
                      if (w) setResultQualifier(w);
                    }
                    if (h !== a) setResultPens(false);
                  }
                }}
              />
            </ScoreInputs>

            {knockout && (
              <div className="flex flex-col gap-2 rounded-md bg-sky/10 p-3">
                <Label>Quem se classifica</Label>
                <Select
                  value={resultQualifier}
                  onChange={(e) => {
                    setResultQualifier(e.target.value);
                    setResultQualTouched(true);
                  }}
                >
                  <option value="home">{formatSelectionLabel(game.homeTeam)}</option>
                  <option value="away">{formatSelectionLabel(game.awayTeam)}</option>
                </Select>
                {(resultHome ?? 0) === (resultAway ?? 0) && (
                  <>
                    <label className="flex items-center gap-2 text-sm">
                      <input
                        type="checkbox"
                        checked={resultPens}
                        onChange={(e) => setResultPens(e.target.checked)}
                      />
                      Foi para os pênaltis?
                    </label>
                    {resultPens && (
                      <>
                        <Label>Placar dos pênaltis (opcional)</Label>
                        <ScoreInputs>
                          <ScoreBox
                            value={resultPenHome ?? 0}
                            onChange={(e) => setResultPenHome(parseInt(e.target.value) || 0)}
                          />
                          <span className="font-heading text-xl font-bold text-ink-muted">x</span>
                          <ScoreBox
                            value={resultPenAway ?? 0}
                            onChange={(e) => setResultPenAway(parseInt(e.target.value) || 0)}
                          />
                        </ScoreInputs>
                      </>
                    )}
                  </>
                )}
                <p className="text-xs text-ink-muted">
                  Empate no tempo normal sem pênaltis = classificado decidido na prorrogação (sem
                  pontos extras).
                </p>
              </div>
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
    </MotionCard>
  );
}
