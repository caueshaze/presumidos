import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/field";

export function CustomScoringRow({
  question,
  owner,
  save,
}: {
  question: import("@/types").CustomQuestion;
  owner: boolean;
  save: (correct: number, incorrect: number) => Promise<unknown>;
}) {
  const [correct, setCorrect] = useState(String(question.correctPoints));
  const [incorrect, setIncorrect] = useState(String(question.incorrectPoints));
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const handleSave = async () => {
    if (saving) return;
    setSaving(true);
    setSaved(false);
    try {
      await save(+correct, +incorrect);
      setSaved(true);
      window.setTimeout(() => setSaved(false), 2200);
    } finally {
      setSaving(false);
    }
  };
  return (
    <div className="grid gap-3 border-b border-mint/15 pb-3 last:border-0 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
      <span className="font-medium">{question.title}</span>
      {owner ? (
        <div className="flex flex-wrap items-end gap-3">
          <label className="flex flex-col gap-1 text-xs font-semibold text-ink-muted">
            Acerto
            <Input
              aria-label={`Pontos por acerto: ${question.title}`}
              className="w-16"
              type="number"
              min="0"
              value={correct}
              onChange={(e) => {
                setCorrect(e.target.value);
                setSaved(false);
              }}
            />
          </label>
          <label className="flex flex-col gap-1 text-xs font-semibold text-ink-muted">
            Erro
            <Input
              aria-label={`Pontos por erro: ${question.title}`}
              className="w-16"
              type="number"
              min="0"
              value={incorrect}
              onChange={(e) => {
                setIncorrect(e.target.value);
                setSaved(false);
              }}
            />
          </label>
          <Button
            size="sm"
            variant="outline"
            className={saved ? "border-mint-dark bg-mint/15 text-mint-dark" : undefined}
            disabled={saving}
            onClick={() => void handleSave()}
          >
            {saving ? "Salvando..." : saved ? "Salvo ✓" : "Salvar"}
          </Button>
        </div>
      ) : (
        <span className="text-sm text-ink-muted">
          {question.status !== "open"
            ? "Palpites encerrados; pontuação somente leitura."
            : `${question.correctPoints} pts por acerto`}
        </span>
      )}
    </div>
  );
}
export function CustomResultRow({
  question,
  save,
}: {
  question: import("@/types").CustomQuestion;
  save: (optionId: string) => Promise<unknown>;
}) {
  const [optionId, setOptionId] = useState(question.correctOptionId ?? "");
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-mint/15 pb-3 last:border-0">
      <Label>{question.title}</Label>
      <div className="flex w-full gap-2 sm:w-[32rem]">
        <select
          aria-label={`Vencedor: ${question.title}`}
          className="min-w-0 flex-1 rounded-lg border border-mint/25 bg-card px-3 py-2"
          value={optionId}
          onChange={(e) => setOptionId(e.target.value)}
        >
          <option value="">Selecione</option>
          {question.options.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label}
            </option>
          ))}
        </select>
        <Button
          size="sm"
          className="w-32 shrink-0 justify-center"
          disabled={!optionId}
          onClick={() => save(optionId)}
        >
          Salvar resultado
        </Button>
      </div>
    </div>
  );
}
