import { AnimatePresence, motion } from "framer-motion";
import { X } from "lucide-react";
import type { CustomQuestion, FootballScoringConfig } from "@/types";
interface Props {
  open: boolean;
  onClose: () => void;
  isFootball: boolean;
  footballScoring?: FootballScoringConfig;
  customQuestions?: CustomQuestion[];
  tieBreakPriorities?: { itemId: string; title: string; priority: number }[];
}
export function ScoringInfoModal({
  open,
  onClose,
  isFootball,
  footballScoring,
  customQuestions,
  tieBreakPriorities,
}: Props) {
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          key="scoring-backdrop"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.18 }}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
          onClick={onClose}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 8 }}
            transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
            className="relative w-full max-w-lg rounded-2xl bg-card p-6 shadow-card-hover"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="flex items-start justify-between gap-4">
              <h2 className="text-xl font-heading font-semibold">
                Como funciona a pontuação
              </h2>
              <button
                onClick={onClose}
                className="mt-0.5 shrink-0 rounded-full p-1 text-ink-muted transition-colors hover:bg-secondary hover:text-ink"
                aria-label="Fechar"
              >
                <X className="h-5 w-5" />
              </button>
            </div>
            {isFootball ? (
              <FootballScoring scoring={footballScoring} />
            ) : (
              <CustomScoring questions={customQuestions} tieBreakPriorities={tieBreakPriorities} />
            )}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
function FootballScoring({ scoring }: { scoring?: FootballScoringConfig }) {
  const rows = [
    ["Placar exato", scoring?.exactScorePoints],
    ["Resultado + lado exato", scoring?.correctResultExactSidePoints],
    ["Resultado correto", scoring?.correctResultPoints],
    ["Erro", scoring?.incorrectResultPoints],
    ["Bônus mata-mata", scoring?.knockoutBonusPoints],
  ];
  return (
    <>
      <p className="mt-1 text-sm text-ink-muted">
        Aplica-se a todos os jogos (grupos e mata-mata).
      </p>
      <table className="mt-4 w-full text-sm">
        <tbody className="divide-y divide-mint/15">
          {rows.map(([label, points]) => (
            <tr key={label as string}>
              <td className="py-2 pr-4 text-ink">{label}</td>
              <td className="py-2 text-right font-semibold text-mint-dark">
                {points ?? "—"} pts
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="mt-5 rounded-xl bg-secondary/50 px-4 py-3 text-sm text-ink-muted">
        <span className="font-semibold text-ink">Desempate</span> — mais
        placares exatos, acertos de resultado e bônus. Ajustes manuais não
        entram no desempate.
      </div>
    </>
  );
}
function CustomScoring({ questions, tieBreakPriorities }: { questions?: CustomQuestion[]; tieBreakPriorities?: { itemId: string; title: string; priority: number }[] }) {
  return (
    <>
      <p className="mt-1 text-sm text-ink-muted">
        Cada categoria usa os valores configurados neste bolão.
      </p>
      <table className="mt-4 w-full text-sm">
        <tbody className="divide-y divide-mint/15">
          {questions?.map((question) => (
            <tr key={question.itemId}>
              <td className="py-2 pr-4 text-ink">{question.title}</td>
              <td className="py-2 text-right font-semibold text-mint-dark">
                {question.correctPoints} pts
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {!!tieBreakPriorities?.length && <div className="mt-5 rounded-xl bg-secondary/50 px-4 py-3 text-sm text-ink-muted"><span className="font-semibold text-ink">Desempate</span><p className="mt-1">Em empate de pontos, vale o acerto exato nesta ordem:</p><ol className="mt-2 list-decimal space-y-1 pl-5">{tieBreakPriorities.map((item) => <li key={item.itemId}>{item.title}</li>)}</ol></div>}
    </>
  );
}
