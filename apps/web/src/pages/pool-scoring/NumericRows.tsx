import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/field";

export function NumericScoringRow({
  question,
  owner,
  save,
}: {
  question: import("@/types").CustomQuestion;
  owner: boolean;
  save: (
    exact: number,
    tolerance: string,
    within: number,
    incorrect: number,
  ) => Promise<unknown>;
}) {
  const [exact, setExact] = useState(String(question.exactPoints ?? 1));
  const [tolerance, setTolerance] = useState(question.tolerance ?? "0");
  const [within, setWithin] = useState(
    String(question.withinTolerancePoints ?? 0),
  );
  const [incorrect, setIncorrect] = useState(String(question.incorrectPoints));
  return (
    <div className="border-b border-mint/15 pb-3">
      <strong>{question.title}</strong>
      <div className="mt-2 flex flex-wrap gap-2">
        {owner ? (
          <>
            <label>
              Exato
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
              Tolerância{question.unitLabel ? ` (${question.unitLabel})` : ""}
              <Input
                aria-label={`Tolerância: ${question.title}`}
                className="w-20"
                inputMode="decimal"
                value={tolerance}
                onChange={(e) => setTolerance(e.target.value)}
              />
            </label>
            <label>
              Dentro
              <Input
                aria-label={`Dentro da tolerância: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={within}
                onChange={(e) => setWithin(e.target.value)}
              />
            </label>
            <label>
              Fora
              <Input
                aria-label={`Fora: ${question.title}`}
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
              onClick={() => save(+exact, tolerance, +within, +incorrect)}
            >
              Salvar
            </Button>
          </>
        ) : (
          <span className="text-sm text-ink-muted">
            Exato {exact} · tolerância {tolerance} · dentro {within} · fora{" "}
            {incorrect}
          </span>
        )}
      </div>
    </div>
  );
}
export function NumericResultRow({
  question,
  save,
}: {
  question: import("@/types").CustomQuestion;
  save: (value: string) => Promise<unknown>;
}) {
  const [value, setValue] = useState(question.resultValue ?? "");
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-mint/15 pb-3">
      <Label>{question.title}</Label>
      <div className="flex gap-2">
        <Input
          aria-label={`Resultado oficial: ${question.title}`}
          inputMode="decimal"
          value={value}
          onChange={(e) => setValue(e.target.value)}
        />
        <Button size="sm" disabled={!value} onClick={() => save(value)}>
          Salvar resultado
        </Button>
      </div>
    </div>
  );
}
