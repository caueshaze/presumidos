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
import { cn, formatKickoff, formatKnockoutPhase, formatLiveStatus, isKnockout } from "@/lib/utils";
import type { KnockoutEntry, MatchPointsSummary, MatchRecord, PredictionRecord } from "@/types";
import { MotionCard } from "./ui/card";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label, Select, ErrorBanner } from "./ui/field";

// Monta o detalhamento de pontos com os componentes que pontuaram (> 0).
function pointsBreakdown(points: MatchPointsSummary): string {
  const parts: string[] = [];
  if (points.exactScorePoints > 0) parts.push(`Placar exato ${points.exactScorePoints}`);
  if (points.outcomePoints > 0) parts.push(`Resultado ${points.outcomePoints}`);
  if (points.goalBonusPoints > 0) parts.push(`Bônus de gols ${points.goalBonusPoints}`);
  if (points.qualifierPoints > 0) parts.push(`Classificado ${points.qualifierPoints}`);
  if (points.penaltiesPoints > 0) parts.push(`Pênaltis ${points.penaltiesPoints}`);
  return parts.join(" · ");
}

function ScoreInputs({ children }: { children: React.ReactNode }) {
  return <div className="flex items-center gap-3">{children}</div>;
}

function ScoreBox(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <Input
      type="text"
      inputMode="numeric"
      pattern="[0-9]*"
      autoComplete="off"
      className="score-input w-20 text-center text-xl font-heading font-bold"
      {...props}
    />
  );
}

function scoreToField(value: number | null | undefined): string {
  return value === null || value === undefined ? "" : String(value);
}

function normalizeScoreField(raw: string): string {
  const digits = raw.replace(/\D+/g, "");
  if (!digits) return "";
  return digits.replace(/^0+(?=\d)/, "");
}

function scoreValue(field: string): number {
  return field === "" ? 0 : parseInt(field, 10) || 0;
}

function qualifierLabel(
  side: string | null | undefined,
  homeTeam: string,
  awayTeam: string,
): string | null {
  if (side === "home") return formatSelectionLabel(homeTeam);
  if (side === "away") return formatSelectionLabel(awayTeam);
  return null;
}

function PredictionSummary({
  title,
  homeTeam,
  awayTeam,
  homeScore,
  awayScore,
  qualifier,
  wentToPenalties,
  penaltyHomeScore,
  penaltyAwayScore,
  tone = "default",
}: {
  title: string;
  homeTeam: string;
  awayTeam: string;
  homeScore: number;
  awayScore: number;
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHomeScore: number | null;
  penaltyAwayScore: number | null;
  tone?: "default" | "official";
}) {
  const qualifierName = qualifierLabel(qualifier, homeTeam, awayTeam);

  return (
    <div
      className={cn(
        "rounded-lg border px-4 py-3",
        tone === "official" ? "border-sky/30 bg-sky/10" : "border-mint/25 bg-mint/10",
      )}
    >
      <p className="text-xs font-semibold uppercase tracking-[0.18em] text-ink-muted">{title}</p>
      <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1">
        <span className="text-sm text-ink">{formatSelectionLabel(homeTeam)}</span>
        <span className="font-heading text-lg font-bold text-ink">
          {homeScore} <span className="text-ink-muted">x</span> {awayScore}
        </span>
        <span className="text-sm text-ink">{formatSelectionLabel(awayTeam)}</span>
      </div>
      {qualifierName && (
        <p className="mt-2 text-sm text-mint-dark">
          Classifica: {qualifierName}
          {wentToPenalties && (
            <>
              {" "}· nos pênaltis
              {penaltyHomeScore !== null && penaltyAwayScore !== null && (
                <>
                  {" "}
                  ({penaltyHomeScore}-{penaltyAwayScore})
                </>
              )}
            </>
          )}
        </p>
      )}
    </div>
  );
}

interface Props {
  game: MatchRecord;
  prediction?: PredictionRecord;
  locked: boolean;
  isAdmin: boolean;
  index: number;
  cardId?: string;
  highlighted?: boolean;
  points?: MatchPointsSummary;
}

export function MatchCard({
  game,
  prediction,
  locked,
  isAdmin,
  index,
  cardId,
  highlighted = false,
  points,
}: Props) {
  const knockout = isKnockout(game.phase);
  const selectionGroups = getSelectionGroups();

  const submit = useSubmitPrediction();
  const setResult = useSetMatchResult();
  const setFinished = useSetMatchFinished();
  const updateTeams = useUpdateMatchTeams();
  const reauth = useReauth();

  // ---- Palpite do usuário ----
  const initialHome = scoreToField(prediction?.homeScore);
  const initialAway = scoreToField(prediction?.awayScore);
  const [homeGuess, setHomeGuess] = useState(initialHome);
  const [awayGuess, setAwayGuess] = useState(initialAway);
  const [penHome, setPenHome] = useState(scoreToField(prediction?.penaltyHomeScore));
  const [penAway, setPenAway] = useState(scoreToField(prediction?.penaltyAwayScore));
  const [savedMessage, setSavedMessage] = useState("");
  const [error, setError] = useState("");

  // A confirmação de "Palpite salvo!" some sozinha depois de alguns segundos.
  useEffect(() => {
    if (!savedMessage) return;
    const timer = setTimeout(() => setSavedMessage(""), 4000);
    return () => clearTimeout(timer);
  }, [savedMessage]);

  useEffect(() => {
    setResultHome(scoreToField(game.homeScore));
    setResultAway(scoreToField(game.awayScore));
    setResultPenHome(scoreToField(game.penaltyHomeScore));
    setResultPenAway(scoreToField(game.penaltyAwayScore));
  }, [
    game.homeScore,
    game.awayScore,
    game.penaltyHomeScore,
    game.penaltyAwayScore,
  ]);

  useEffect(() => {
    setTeamHome(game.homeTeam);
    setTeamAway(game.awayTeam);
  }, [game.homeTeam, game.awayTeam]);

  // ---- Resultado oficial (admin) ----
  const [resultHome, setResultHome] = useState(scoreToField(game.homeScore));
  const [resultAway, setResultAway] = useState(scoreToField(game.awayScore));
  const [resultPenHome, setResultPenHome] = useState(scoreToField(game.penaltyHomeScore));
  const [resultPenAway, setResultPenAway] = useState(scoreToField(game.penaltyAwayScore));
  const [resultError, setResultError] = useState("");

  // ---- Confronto (admin) ----
  const [teamHome, setTeamHome] = useState(game.homeTeam);
  const [teamAway, setTeamAway] = useState(game.awayTeam);
  const [teamsError, setTeamsError] = useState("");

  const teamSelectionFallbacks = [teamHome, teamAway].filter(
    (team, position, allTeams) => allTeams.indexOf(team) === position && !isKnownSelection(team),
  );

  // O classificado é deduzido no servidor (placar ou vencedor dos pênaltis); o
  // empate no tempo normal exige o placar dos pênaltis.
  const buildKnockout = (home: number, away: number, ph: string, pa: string): KnockoutEntry => {
    if (!knockout) return { qualifier: null, wentToPenalties: false, penaltyHome: null, penaltyAway: null };
    const wentPenalties = home === away;
    return {
      qualifier: null,
      wentToPenalties: wentPenalties,
      penaltyHome: wentPenalties && ph !== "" ? scoreValue(ph) : null,
      penaltyAway: wentPenalties && pa !== "" ? scoreValue(pa) : null,
    };
  };

  // Valida o placar dos pênaltis quando o palpite/resultado é um empate.
  const penaltiesError = (home: number, away: number, ph: string, pa: string): string | null => {
    if (!knockout || home !== away) return null;
    if (ph === "" || pa === "") return "Empate no tempo normal: informe o placar dos pênaltis dos dois lados.";
    if (scoreValue(ph) === scoreValue(pa)) return "O placar dos pênaltis não pode terminar empatado.";
    return null;
  };

  const onSave = async (e: FormEvent) => {
    e.preventDefault();
    setSavedMessage("");
    setError("");
    const home = scoreValue(homeGuess);
    const away = scoreValue(awayGuess);
    const penError = penaltiesError(home, away, penHome, penAway);
    if (penError) {
      setError(penError);
      return;
    }
    try {
      await submit.mutateAsync({
        matchId: game.id,
        homeScore: home,
        awayScore: away,
        knockout: buildKnockout(home, away, penHome, penAway),
      });
      setSavedMessage("Palpite salvo!");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao salvar palpite.");
    }
  };

  const onSaveResult = async (e: FormEvent) => {
    e.preventDefault();
    setResultError("");
    const home = scoreValue(resultHome);
    const away = scoreValue(resultAway);
    const penError = penaltiesError(home, away, resultPenHome, resultPenAway);
    if (penError) {
      setResultError(penError);
      return;
    }
    try {
      await withAdminReauth(
        () =>
          setResult.mutateAsync({
            matchId: game.id,
            homeScore: home,
            awayScore: away,
            knockout: buildKnockout(home, away, resultPenHome, resultPenAway),
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

  // Pênaltis só aparecem num empate de verdade: os dois placares preenchidos e
  // iguais (inclui 0x0). Campos vazios não contam como empate.
  const drawGuess =
    knockout &&
    homeGuess !== "" &&
    awayGuess !== "" &&
    scoreValue(homeGuess) === scoreValue(awayGuess);

  // Jogo em andamento segundo o poller (placar parcial). O backend só mantém
  // live_status preenchido enquanto o jogo está em andamento.
  const liveInProgress = !game.finished && !hasOfficial && !!game.liveStatus;
  const liveLabel = formatLiveStatus(game.liveStatus, game.liveElapsed);
  const qualifierSuffix =
    knockout && game.qualifier
      ? ` — ${qualifierLabel(game.qualifier, game.homeTeam, game.awayTeam)} classificou`
      : "";
  const penaltyLabel =
    game.penaltyHomeScore !== null && game.penaltyAwayScore !== null
      ? `Pênaltis: ${game.penaltyHomeScore} x ${game.penaltyAwayScore}`
      : "Decidido nos pênaltis";
  const showInlineOfficialSummary = locked && !isAdmin && !!prediction;
  const showLockedMessage = locked && !game.finished;
  const showAdminControls = false;
  const exactScoreHit =
    hasOfficial &&
    prediction &&
    game.homeScore === prediction.homeScore &&
    game.awayScore === prediction.awayScore;
  const qualifierHit =
    knockout &&
    hasOfficial &&
    prediction &&
    game.qualifier &&
    prediction.qualifier === game.qualifier;

  return (
    <MotionCard
      id={cardId}
      className={cn(
        "mb-4 scroll-mt-24 transition-shadow duration-500",
        hasPrediction && "ring-2 ring-success/60",
        savedMessage && "shadow-[0_0_0_6px_rgba(95,191,159,0.18)]",
        highlighted && "ring-2 ring-sky/60 shadow-[0_0_0_6px_rgba(130,207,255,0.22)]",
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
              {formatKnockoutPhase(game.phase)}
            </span>
          )}
          {game.groupName && (
            <span className="rounded-pill bg-mint/40 px-3 py-1 text-xs font-semibold">
              Grupo {game.groupName}
            </span>
          )}
          {locked && !game.finished && (
            <span className="inline-flex items-center gap-1.5 rounded-pill bg-danger-bg px-3 py-1 text-xs font-semibold text-danger ring-1 ring-danger/40">
              <span className="relative flex h-2 w-2">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-danger opacity-75" />
                <span className="relative inline-flex h-2 w-2 rounded-full bg-danger" />
              </span>
              AO VIVO
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

      {liveInProgress && (
        <p className="mt-2 flex flex-wrap items-center gap-2 font-semibold text-danger">
          <span>
            Ao vivo: {game.liveHomeScore ?? 0} x {game.liveAwayScore ?? 0}
          </span>
          <span className="rounded-pill bg-danger-bg px-2 py-0.5 text-xs font-semibold text-danger ring-1 ring-danger/40">
            {liveLabel}
          </span>
        </p>
      )}

      {hasOfficial && !showInlineOfficialSummary && (
        <p className="mt-2 font-semibold">
          Resultado oficial: {game.homeScore} x {game.awayScore}
          {qualifierSuffix}
        </p>
      )}
      {hasOfficial && knockout && game.wentToPenalties && !showInlineOfficialSummary && (
        <p className="text-sm text-ink-muted">{penaltyLabel}</p>
      )}

      {/* Admin não palpita — vê apenas os controles administrativos abaixo. */}
      {!isAdmin &&
        (locked ? (
          hasPrediction && prediction ? (
            <div className="mt-4 space-y-3">
              <PredictionSummary
                title="Seu palpite"
                homeTeam={game.homeTeam}
                awayTeam={game.awayTeam}
                homeScore={prediction.homeScore}
                awayScore={prediction.awayScore}
                qualifier={prediction.qualifier}
                wentToPenalties={prediction.wentToPenalties}
                penaltyHomeScore={prediction.penaltyHomeScore}
                penaltyAwayScore={prediction.penaltyAwayScore}
              />
              {showLockedMessage && (
                <p className="rounded-md bg-danger-bg px-3 py-2 text-sm font-semibold">
                  Partida já iniciada — palpites encerrados.
                </p>
              )}
              {hasOfficial && (
                <>
                  <PredictionSummary
                    title="Resultado oficial"
                    homeTeam={game.homeTeam}
                    awayTeam={game.awayTeam}
                    homeScore={game.homeScore ?? 0}
                    awayScore={game.awayScore ?? 0}
                    qualifier={game.qualifier}
                    wentToPenalties={game.wentToPenalties}
                    penaltyHomeScore={game.penaltyHomeScore}
                    penaltyAwayScore={game.penaltyAwayScore}
                    tone="official"
                  />
                  <div className="flex flex-wrap gap-2 text-xs font-semibold">
                    <span
                      className={cn(
                        "rounded-pill px-3 py-1 ring-1",
                        exactScoreHit
                          ? "bg-success/15 text-mint-dark ring-success/35"
                          : "bg-card text-ink-muted ring-mint/25",
                      )}
                    >
                      {exactScoreHit ? "Placar exato: acertou" : "Placar exato: não bateu"}
                    </span>
                    {knockout && game.qualifier && (
                      <span
                        className={cn(
                          "rounded-pill px-3 py-1 ring-1",
                          qualifierHit
                            ? "bg-success/15 text-mint-dark ring-success/35"
                            : "bg-card text-ink-muted ring-mint/25",
                        )}
                      >
                        {qualifierHit ? "Classificado: acertou" : "Classificado: não bateu"}
                      </span>
                    )}
                  </div>
                  {points && (
                    <div className="rounded-md bg-mint/10 px-3 py-2">
                      {points.totalPoints > 0 ? (
                        <>
                          <p className="font-heading font-semibold text-mint-dark">
                            Você fez {points.totalPoints}{" "}
                            {points.totalPoints === 1 ? "ponto" : "pontos"}
                          </p>
                          {pointsBreakdown(points) && (
                            <p className="mt-0.5 text-xs text-ink-muted">{pointsBreakdown(points)}</p>
                          )}
                        </>
                      ) : (
                        <p className="text-sm font-semibold text-ink-muted">
                          Nenhum ponto neste jogo
                        </p>
                      )}
                    </div>
                  )}
                  {knockout && game.wentToPenalties && (
                    <p className="text-sm text-ink-muted">{penaltyLabel}</p>
                  )}
                </>
              )}
            </div>
          ) : (
            showLockedMessage && (
              <p className="mt-3 rounded-md bg-danger-bg px-3 py-2 text-sm font-semibold">
                Partida já iniciada palpites encerrados!
              </p>
            )
          )
        ) : (
        <form onSubmit={onSave} className="mt-4 flex flex-col gap-3">
          {knockout && <Label>Considere os 90 minutos. Se houver prorrogação, considere os 120 minutos.</Label>}
          <ScoreInputs>
            <ScoreBox
              value={homeGuess}
              onChange={(e) => setHomeGuess(normalizeScoreField(e.target.value))}
            />
            <span className="font-heading text-xl font-bold text-ink-muted">x</span>
            <ScoreBox
              value={awayGuess}
              onChange={(e) => setAwayGuess(normalizeScoreField(e.target.value))}
            />
          </ScoreInputs>

          <AnimatePresence initial={false}>
            {drawGuess && (
              // -mt-3 cancela o gap-3 do form enquanto a altura anima (mesmo
              // padrão do "Palpite salvo!"), evitando o "pulo" do flex gap.
              <motion.div
                key="penalties"
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                transition={{
                  height: { duration: 0.32, ease: [0.22, 1, 0.36, 1] },
                  opacity: { duration: 0.22, ease: "easeOut" },
                }}
                className="-mt-3 overflow-hidden"
              >
                <div className="mt-3 flex flex-col gap-2 rounded-md bg-mint/10 p-3">
                  <Label>Placar dos pênaltis</Label>
                  <ScoreInputs>
                    <ScoreBox
                      value={penHome}
                      onChange={(e) => setPenHome(normalizeScoreField(e.target.value))}
                    />
                    <span className="font-heading text-xl font-bold text-ink-muted">x</span>
                    <ScoreBox
                      value={penAway}
                      onChange={(e) => setPenAway(normalizeScoreField(e.target.value))}
                    />
                  </ScoreInputs>
                </div>
              </motion.div>
            )}
          </AnimatePresence>

          {error && <ErrorBanner>{error}</ErrorBanner>}
          <AnimatePresence initial={false}>
            {savedMessage && (
              // Wrapper externo: -mt-3 cancela de forma constante o gap-3 do form
              // (evita o "pulo" que o flex gap dá ao montar/desmontar o item).
              // Só height + opacity animam; o espaçamento visível (mt-3 interno)
              // vive dentro da área overflow-hidden, então colapsa junto com a altura.
              <motion.div
                key="saved"
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                transition={{
                  height: { duration: 0.32, ease: [0.22, 1, 0.36, 1] },
                  opacity: { duration: 0.22, ease: "easeOut" },
                }}
                className="-mt-3 overflow-hidden"
              >
                <div className="mt-3 flex items-center gap-2 rounded-md border border-success/40 bg-mint/30 px-4 py-2.5 font-heading font-semibold text-mint-dark">
                  <motion.span
                    initial={{ scale: 0, rotate: -30 }}
                    animate={{ scale: 1, rotate: 0 }}
                    transition={{ type: "spring", stiffness: 500, damping: 18, delay: 0.08 }}
                    className="flex"
                  >
                    <CheckCircle2 className="h-5 w-5" strokeWidth={2.5} />
                  </motion.span>
                  {savedMessage}
                </div>
              </motion.div>
            )}
          </AnimatePresence>
          <Button type="submit" disabled={submit.isPending} className="self-start">
            {submit.isPending ? "Salvando..." : savedMessage ? "Palpite salvo ✓" : "Salvar palpite"}
          </Button>
        </form>
        ))}

      {isAdmin && showAdminControls && (
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
              onChange={(e) => setResultHome(normalizeScoreField(e.target.value))}
              />
              <span className="font-heading text-xl font-bold text-ink-muted">x</span>
              <ScoreBox
                value={resultAway}
                onChange={(e) => setResultAway(normalizeScoreField(e.target.value))}
              />
            </ScoreInputs>

            {knockout && scoreValue(resultHome) === scoreValue(resultAway) && (
              <div className="flex flex-col gap-2 rounded-md bg-sky/10 p-3">
                <Label>Placar dos pênaltis</Label>
                <ScoreInputs>
                  <ScoreBox
                    value={resultPenHome}
                    onChange={(e) => setResultPenHome(normalizeScoreField(e.target.value))}
                  />
                  <span className="font-heading text-xl font-bold text-ink-muted">x</span>
                  <ScoreBox
                    value={resultPenAway}
                    onChange={(e) => setResultPenAway(normalizeScoreField(e.target.value))}
                  />
                </ScoreInputs>
                <p className="text-xs text-ink-muted">
                  Empate no tempo normal: informe o placar dos pênaltis (quem fez mais se classifica).
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
