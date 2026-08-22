import { useEffect, useState } from "react";
import { CheckCircle2 } from "lucide-react";
import { useSubmitCustomPrediction } from "@/hooks/queries";
import type { CustomQuestion } from "@/types";
import { MotionCard } from "./ui/card";
import { ErrorBanner } from "./ui/field";

export function SingleChoicePredictionCard({ question, poolId, index, preview = false }: { question: CustomQuestion; poolId: string; index: number; preview?: boolean }) {
  const submit = useSubmitCustomPrediction();
  const [selected, setSelected] = useState(question.currentOptionId ?? "");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const locked = question.status === "locked" || question.status === "resolved";
  const resolved = question.status === "resolved";

  useEffect(() => setSelected(question.currentOptionId ?? ""), [question.currentOptionId]);

  const choose = async (optionId: string) => {
    if (locked || submit.isPending) return;
    setSelected(optionId);
    setError("");
    if (preview) return;
    try {
      await submit.mutateAsync({ poolId, itemId: question.itemId, optionId });
      setMessage("Palpite salvo.");
    } catch (cause) {
      setSelected(question.currentOptionId ?? "");
      setError(cause instanceof Error ? cause.message : "Não foi possível salvar o palpite.");
    }
  };

  return (
    <MotionCard transition={{ delay: Math.min(index * 0.025, 0.25), duration: 0.25 }}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg">{question.title}</h2>
          <p className="mt-1 text-sm text-ink-muted">Vale {question.correctPoints} {question.correctPoints === 1 ? "ponto" : "pontos"}</p>
        </div>
        <span className="rounded-pill bg-mint/15 px-2.5 py-1 text-xs font-semibold text-mint-dark">
          {resolved ? "Resultado divulgado" : locked ? "Palpites encerrados" : selected ? "Respondida" : "Em aberto"}
        </span>
      </div>
      {locked && <p className="mt-3 text-sm text-ink-muted">Os palpites desta categoria já foram encerrados.</p>}
      <fieldset className="mt-4 space-y-2" disabled={locked || submit.isPending}>
        <legend className="sr-only">Opções para {question.title}</legend>
        {question.options.map((option) => {
          const checked = selected === option.id;
          const correct = resolved && question.correctOptionId === option.id;
          return (
            <label key={option.id} className={`flex cursor-pointer items-center gap-3 rounded-xl border px-3 py-3 transition ${checked ? "border-mint-dark bg-mint/15" : "border-mint/20 bg-card/40 hover:border-mint/50"} ${locked ? "cursor-default opacity-80" : ""}`}>
              <input className="h-4 w-4 accent-[var(--color-mint-dark)]" type="radio" name={question.itemId} checked={checked} onChange={() => choose(option.id)} />
              <span className="flex-1 text-sm font-medium">{option.label}</span>
              {correct && <CheckCircle2 className="h-5 w-5 text-success" aria-label="Opção vencedora" />}
            </label>
          );
        })}
      </fieldset>
      {message && <p className="mt-3 text-sm font-semibold text-mint-dark">{message}</p>}
      {error && <div className="mt-3"><ErrorBanner>{error}</ErrorBanner></div>}
    </MotionCard>
  );
}
