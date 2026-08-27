import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Check } from "lucide-react";
import { useSubmitMultipleChoicePrediction } from "@/hooks/queries";
import type { CustomQuestion } from "@/types";
import { MotionCard } from "./ui/card";
import { Button } from "./ui/button";
import { ErrorBanner } from "./ui/field";
import { OptionMediaActions } from "./OptionMediaActions";

export function MultipleChoicePredictionCard({
  question,
  poolId,
  index,
  preview = false,
}: {
  question: CustomQuestion;
  poolId: string;
  index: number;
  preview?: boolean;
}) {
  const submit = useSubmitMultipleChoicePrediction();
  const saved = question.currentOptionIds ?? [];
  const [selected, setSelected] = useState<string[]>(saved);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  useEffect(() => setSelected(saved), [question.currentOptionIds]);
  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(() => setMessage(""), 3200);
    return () => window.clearTimeout(timer);
  }, [message]);
  const locked = question.status === "locked" || question.status === "resolved";
  const min = question.minSelections ?? 1;
  const max = question.maxSelections ?? question.options.length;
  const dirty =
    selected.length !== saved.length ||
    selected.some((id) => !saved.includes(id));
  const toggle = (id: string) => {
    if (locked || submit.isPending) return;
    setSelected((old) =>
      old.includes(id)
        ? old.filter((value) => value !== id)
        : old.length >= max
          ? old
          : [...old, id],
    );
  };
  const save = async () => {
    if (
      locked ||
      selected.length < min ||
      selected.length > max ||
      !dirty ||
      submit.isPending
    )
      return;
    if (preview) {
      setMessage("Prévia: conjunto não salvo.");
      return;
    }
    try {
      await submit.mutateAsync({
        poolId,
        itemId: question.itemId,
        optionIds: selected,
      });
      setMessage("Palpite salvo.");
      setError("");
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : "Não foi possível salvar o palpite.",
      );
    }
  };
  return (
    <MotionCard
      transition={{ delay: Math.min(index * 0.025, 0.25), duration: 0.25 }}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg">{question.title}</h2>
          <p className="mt-1 text-sm text-ink-muted">
            Exato: {question.exactPoints ?? 1} pts · Parcial:{" "}
            {question.partialPoints ?? 0} pts
          </p>
        </div>
        <span className="rounded-pill bg-mint/15 px-2.5 py-1 text-xs font-semibold text-mint-dark">
          {question.status === "resolved"
            ? "Resultado divulgado"
            : locked
              ? "Palpites encerrados"
              : saved.length
                ? "Respondida"
                : "Em aberto"}
        </span>
      </div>
      <p className="mt-2 text-sm text-ink-muted">
        Selecione de {min} a {max}. Parcial: você selecionou apenas opções
        corretas, mas deixou de marcar uma ou mais opções vencedoras.
      </p>
      <fieldset className="mt-3 space-y-2">
        {question.options.map((option, optionIndex) => (
          <motion.div
            key={option.id}
            animate={{ scale: selected.includes(option.id) ? 1.012 : 1 }}
            transition={{ type: "spring", stiffness: 420, damping: 26 }}
            whileTap={locked ? undefined : { scale: 0.985 }}
            className={`block rounded-xl border p-3 transition-colors ${selected.includes(option.id) ? "border-mint-dark bg-mint/15 shadow-sm" : "border-mint/20 bg-card/40 hover:border-mint/50 hover:bg-card/65"} ${locked ? "cursor-default opacity-80" : ""}`}
          >
            <label htmlFor={`${question.itemId}-${option.id}`} className="flex w-full cursor-pointer items-center gap-3 focus-within:outline-none focus-within:ring-2 focus-within:ring-mint-dark/50 focus-within:ring-offset-2 focus-within:ring-offset-surface">
              <input
                id={`${question.itemId}-${option.id}`}
                type="checkbox"
                className="sr-only"
                checked={selected.includes(option.id)}
                disabled={
                  locked ||
                  submit.isPending ||
                  (!selected.includes(option.id) && selected.length >= max)
                }
                onChange={() => toggle(option.id)}
              />
              <span aria-hidden="true" className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-mint/15 text-xs font-bold text-mint-dark">
                {optionIndex + 1}
              </span>
              {(option.imageAssetUrl ?? option.imageUrl) && <img src={option.imageAssetUrl ?? option.imageUrl ?? undefined} alt="" loading="lazy" className="aspect-square h-16 w-16 shrink-0 rounded-xl object-cover ring-1 ring-mint/20" onError={(event) => {
                if (option.imageAssetUrl && option.imageUrl && event.currentTarget.dataset.fallback !== "used") {
                  event.currentTarget.dataset.fallback = "used";
                  event.currentTarget.src = option.imageUrl;
                } else {
                  event.currentTarget.style.display = "none";
                }
              }} />}
              <span className="flex-1 text-sm font-medium">{option.label}</span>
              <span aria-hidden="true" className="flex h-7 w-7 shrink-0 items-center justify-center">
                <AnimatePresence initial={false}>
                  {selected.includes(option.id) && <motion.span key="selected" initial={{ opacity: 0, scale: 0.35, rotate: -18 }} animate={{ opacity: 1, scale: 1, rotate: 0 }} exit={{ opacity: 0, scale: 0.35, rotate: 18 }} transition={{ type: "spring", stiffness: 520, damping: 20 }} className="flex h-7 w-7 items-center justify-center rounded-lg bg-mint-dark text-accent-fg"><Check className="h-4 w-4" /></motion.span>}
                </AnimatePresence>
              </span>
            </label>
            <OptionMediaActions option={option} poolId={poolId} />
          </motion.div>
        ))}
      </fieldset>
      <p className="mt-2 text-xs text-ink-muted">
        {selected.length} de {max} selecionados
      </p>
      <Button
        className="mt-3"
        disabled={
          locked ||
          !dirty ||
          selected.length < min ||
          selected.length > max ||
          submit.isPending
        }
        onClick={() => void save()}
      >
        Salvar palpite
      </Button>
      {question.status === "resolved" && question.correctOptionIds && (
        <p className="mt-3 text-sm font-semibold text-success">
          Resultado oficial:{" "}
          {question.options
            .filter((option) => question.correctOptionIds?.includes(option.id))
            .map((option) => option.label)
            .join(" • ")}
        </p>
      )}
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
              <motion.span initial={{ scale: 0, rotate: -25 }} animate={{ scale: 1, rotate: 0 }} transition={{ type: "spring", stiffness: 500, damping: 22, delay: 0.04 }}><Check className="h-4 w-4" /></motion.span>
              {message}
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
      {error && <ErrorBanner>{error}</ErrorBanner>}
    </MotionCard>
  );
}
