import { useEffect, useState } from "react";
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
        {question.options.map((option) => (
          <div
            key={option.id}
            className="block rounded-xl border border-mint/20 p-3"
          >
            <label htmlFor={`${question.itemId}-${option.id}`} className="flex cursor-pointer items-center gap-3">
              <input
                id={`${question.itemId}-${option.id}`}
                type="checkbox"
                checked={selected.includes(option.id)}
                disabled={
                  locked ||
                  submit.isPending ||
                  (!selected.includes(option.id) && selected.length >= max)
                }
                onChange={() => toggle(option.id)}
              />
              {(option.imageAssetUrl ?? option.imageUrl) && <img src={option.imageAssetUrl ?? option.imageUrl ?? undefined} alt="" loading="lazy" className="aspect-square h-11 w-11 shrink-0 rounded-lg object-cover" onError={(event) => {
                if (option.imageAssetUrl && option.imageUrl && event.currentTarget.dataset.fallback !== "used") {
                  event.currentTarget.dataset.fallback = "used";
                  event.currentTarget.src = option.imageUrl;
                } else {
                  event.currentTarget.style.display = "none";
                }
              }} />}
              <span>{option.label}</span>
            </label>
            <OptionMediaActions option={option} poolId={poolId} />
          </div>
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
      {message && (
        <p className="mt-3 text-sm font-semibold text-mint-dark">{message}</p>
      )}
      {error && <ErrorBanner>{error}</ErrorBanner>}
    </MotionCard>
  );
}
