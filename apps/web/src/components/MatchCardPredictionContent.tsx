import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "./ui/button";
import { ErrorBanner, Label } from "./ui/field";
import { PenaltyScorePanel, PredictionSummary, ScoreBox, ScoreInputs, normalizeScoreField, pointsBreakdown } from "./MatchCardShared";

export function MatchCardPredictionContent(props: any) {
  const {
    isAdmin, locked, hasPrediction, prediction, game, showLockedMessage, hasOfficial,
    exactScoreHit, knockout, qualifierHit, points, penaltyLabel, onSave,
    homeGuess, setHomeGuess, awayGuess, setAwayGuess, drawGuess,
    penHome, setPenHome, penAway, setPenAway, error, savedMessage, submit,
  } = props;

  return <>{/* Admin não palpita — vê apenas os controles administrativos abaixo. */}
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
                <PenaltyScorePanel className="mt-3">
                  <ScoreBox
                    value={penHome}
                    onChange={(e) => setPenHome(normalizeScoreField(e.target.value))}
                  />
                  <span className="font-heading text-xl font-bold text-ink-muted">x</span>
                  <ScoreBox
                    value={penAway}
                    onChange={(e) => setPenAway(normalizeScoreField(e.target.value))}
                  />
                </PenaltyScorePanel>
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
</>;
}
