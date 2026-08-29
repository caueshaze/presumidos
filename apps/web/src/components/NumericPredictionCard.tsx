import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2 } from "lucide-react";
import { useSubmitNumericPrediction } from "@/hooks/queries";
import type { CustomQuestion } from "@/types";
import { MotionCard } from "./ui/card";
import { ErrorBanner } from "./ui/field";

export function NumericPredictionCard({ question, poolId, index, preview = false }: { question: CustomQuestion; poolId: string; index: number; preview?: boolean }) {
  const submit = useSubmitNumericPrediction();
  const [value, setValue] = useState(question.currentValue ?? "");
  const [message, setMessage] = useState(""); const [error, setError] = useState("");
  const locked = question.status === "locked" || question.status === "resolved";
  useEffect(() => setValue(question.currentValue ?? ""), [question.currentValue]);
  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(() => setMessage(""), 3200);
    return () => window.clearTimeout(timer);
  }, [message]);
  const save = async () => {
    if (locked || !value.trim() || submit.isPending) return;
    if (preview) { setMessage("Prévia: valor não salvo."); return; }
    try { await submit.mutateAsync({ poolId, itemId: question.itemId, value }); setMessage("Palpite salvo."); setError(""); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "Não foi possível salvar o palpite."); }
  };
  return <MotionCard transition={{ delay: Math.min(index * 0.025, 0.25), duration: 0.25 }}>
    <div className="flex flex-wrap items-start justify-between gap-3"><div><h2 className="text-lg">{question.title}</h2><p className="mt-1 text-sm text-ink-muted">Exato: {question.exactPoints ?? 1} pts{question.unitLabel ? ` · unidade: ${question.unitLabel}` : ""}</p></div><span className="rounded-pill bg-mint/15 px-2.5 py-1 text-xs font-semibold text-mint-dark">{question.status === "resolved" ? "Resultado divulgado" : locked ? "Palpites encerrados" : value ? "Respondida" : "Em aberto"}</span></div>
    <div className="mt-4 flex gap-2"><input aria-label={`Valor para ${question.title}`} inputMode="decimal" value={value} disabled={locked || submit.isPending} onChange={e => setValue(e.target.value)} onBlur={() => void save()} placeholder={question.decimalPlaces ? `0.${"0".repeat(question.decimalPlaces)}` : "0"} className="w-40 rounded-lg border border-mint/25 bg-card px-3 py-2" />{question.unitLabel && <span className="self-center text-sm text-ink-muted">{question.unitLabel}</span>}</div>
    {(question.minValue || question.maxValue) && <p className="mt-2 text-xs text-ink-muted">Limites: {question.minValue ?? "—"} a {question.maxValue ?? "—"}</p>}
    {question.status === "resolved" && question.resultValue != null && <p className="mt-3 text-sm font-semibold text-success">Resultado oficial: {question.resultValue}{question.unitLabel ? ` ${question.unitLabel}` : ""}</p>}
    <AnimatePresence initial={false}>
      {message && (
        <motion.div
          key="saved"
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={{
            height: { duration: 0.24, ease: [0.22, 1, 0.36, 1] },
            opacity: { duration: 0.18, ease: "easeOut" },
          }}
          className="overflow-hidden"
        >
          <motion.div
            initial={{ opacity: 0, y: -5, scale: 0.985 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -3, scale: 0.99 }}
            transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
            className="mt-3 flex items-center gap-2 rounded-xl border border-success/35 bg-mint/15 px-3 py-2 text-sm font-semibold text-mint-dark"
          >
            <motion.span initial={{ scale: 0, rotate: -25 }} animate={{ scale: 1, rotate: 0 }} transition={{ type: "spring", stiffness: 500, damping: 22, delay: 0.04 }}><CheckCircle2 className="h-4 w-4" /></motion.span>
            {message}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>{error && <div className="mt-3"><ErrorBanner>{error}</ErrorBanner></div>}
  </MotionCard>;
}
