import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/field";

export function MultipleScoringRow({
  question,
  owner,
  save,
}: {
  question: import("@/types").CustomQuestion;
  owner: boolean;
  save: (exact: number, partial: number, incorrect: number) => Promise<unknown>;
}) {
  const [exact, setExact] = useState(String(question.exactPoints ?? 1));
  const [partial, setPartial] = useState(String(question.partialPoints ?? 0));
  const [incorrect, setIncorrect] = useState(String(question.incorrectPoints));
  return (
    <div className="border-b border-mint/15 pb-3">
      <strong>{question.title}</strong>
      <div className="mt-2 flex flex-wrap gap-2">
        {owner ? (
          <>
            <label>
              Acerto exato
              <Input
                aria-label={`Exato: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={exact}
                onChange={(e) => setExact(e.target.value)}
              />
            </label>
            <label>
              Acerto parcial
              <Input
                aria-label={`Parcial: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={partial}
                onChange={(e) => setPartial(e.target.value)}
              />
            </label>
            <label>
              Incorreto
              <Input
                aria-label={`Incorreto: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={incorrect}
                onChange={(e) => setIncorrect(e.target.value)}
              />
            </label>
            <Button
              size="sm"
              variant="outline"
              onClick={() => save(+exact, +partial, +incorrect)}
            >
              Salvar
            </Button>
          </>
        ) : (
          <span className="text-sm text-ink-muted">
            Exato {exact} · parcial {partial} · incorreto {incorrect}
          </span>
        )}
      </div>
    </div>
  );
}
export function MultipleResultRow({
  question,
  save,
}: {
  question: import("@/types").CustomQuestion;
  save: (optionIds: string[]) => Promise<unknown>;
}) {
  const [selected, setSelected] = useState<string[]>(
    question.correctOptionIds ?? [],
  );
  const min = question.minSelections ?? 1;
  const max = question.maxSelections ?? question.options.length;
  return (
    <div className="border-b border-mint/15 pb-3">
      <Label>{question.title}</Label>
      <div className="mt-2 space-y-1">
        {question.options.map((option) => (
          <label key={option.id} className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={selected.includes(option.id)}
              disabled={!selected.includes(option.id) && selected.length >= max}
              onChange={() =>
                setSelected((old) =>
                  old.includes(option.id)
                    ? old.filter((id) => id !== option.id)
                    : [...old, option.id],
                )
              }
            />
            {option.label}
          </label>
        ))}
      </div>
      <Button
        className="mt-2"
        size="sm"
        disabled={selected.length < min || selected.length > max}
        onClick={() => save(selected)}
      >
        Salvar resultado
      </Button>
    </div>
  );
}
