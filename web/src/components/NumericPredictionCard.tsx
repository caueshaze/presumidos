import { useEffect, useState } from "react";
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
    {message && <p className="mt-3 text-sm font-semibold text-mint-dark">{message}</p>}{error && <div className="mt-3"><ErrorBanner>{error}</ErrorBanner></div>}
  </MotionCard>;
}
