import { useEffect, useState, type FormEvent } from "react";
import { CheckCircle2, Trophy } from "lucide-react";
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
import { cn, formatKickoff, formatKnockoutPhase, isKnockout } from "@/lib/utils";
import type { MatchPointsSummary, MatchRecord, PredictionRecord } from "@/types";
import { MotionCard } from "./ui/card";
import { MatchCardPredictionContent } from "./MatchCardPredictionContent";
import { MatchCardAdminControls } from "./MatchCardAdminControls";
import { buildKnockoutEntry, penaltiesError, qualifierLabel, scoreToField, scoreValue } from "./MatchCardShared";
interface Props {
  poolId: string;
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
  poolId,
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
  const isFinal = game.phase?.trim().toLocaleLowerCase("pt-BR") === "final";
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
  const onSave = async (e: FormEvent) => {
    e.preventDefault();
    setSavedMessage("");
    setError("");
    const home = scoreValue(homeGuess);
    const away = scoreValue(awayGuess);
    const penError = penaltiesError(knockout, home, away, penHome, penAway);
    if (penError) {
      setError(penError);
      return;
    }
    try {
      await submit.mutateAsync({
        poolId,
        matchId: game.id,
        homeScore: home,
        awayScore: away,
        knockout: buildKnockoutEntry(knockout, home, away, penHome, penAway),
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
    const penError = penaltiesError(knockout, home, away, resultPenHome, resultPenAway);
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
            knockout: buildKnockoutEntry(knockout, home, away, resultPenHome, resultPenAway),
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
        isFinal && "final-match-card",
        hasPrediction && "ring-2 ring-success/60",
        savedMessage && "shadow-[0_0_0_6px_rgba(95,191,159,0.18)]",
        highlighted && "ring-2 ring-sky/60 shadow-[0_0_0_6px_rgba(130,207,255,0.22)]",
      )}
      transition={{ delay: Math.min(index * 0.03, 0.3), duration: 0.28 }}
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className={cn("font-heading text-lg font-semibold", isFinal && "final-match-title")}>
          {formatSelectionLabel(game.homeTeam)} vs {formatSelectionLabel(game.awayTeam)}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {isFinal && (
            <span className="final-match-badge">
              <Trophy className="h-3.5 w-3.5" aria-hidden="true" />
              Grande final
            </span>
          )}
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

      {hasOfficial && !showInlineOfficialSummary && (
        <p className="mt-2 font-semibold">
          Resultado oficial: {game.homeScore} x {game.awayScore}
          {qualifierSuffix}
        </p>
      )}
      {hasOfficial && knockout && game.wentToPenalties && !showInlineOfficialSummary && (
        <p className="text-sm text-ink-muted">{penaltyLabel}</p>
      )}

      <MatchCardPredictionContent {...{
        isAdmin, locked, hasPrediction, prediction, game, showLockedMessage, hasOfficial,
        exactScoreHit, knockout, qualifierHit, points, penaltyLabel, onSave,
        homeGuess, setHomeGuess, awayGuess, setAwayGuess, drawGuess,
        penHome, setPenHome, penAway, setPenAway, error, savedMessage, submit,
      }} />
      <MatchCardAdminControls {...{
        isAdmin, showAdminControls, onSaveTeams, teamHome, setTeamHome, teamAway, setTeamAway,
        teamSelectionFallbacks, selectionGroups, teamsError, updateTeams, onSaveResult, knockout,
        resultHome, setResultHome, resultAway, setResultAway, resultPenHome, setResultPenHome,
        resultPenAway, setResultPenAway, resultError, setResult, game, setFinished, onToggleFinished,
      }} />
    </MotionCard>
  );
}
