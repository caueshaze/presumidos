import type { FormEvent } from "react";
import { X } from "lucide-react";
import type { LeaderboardEntry, PointAdjustment } from "@/types";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label, Select, ErrorBanner } from "@/components/ui/field";

interface Props {
  entries: LeaderboardEntry[];
  adjustments: PointAdjustment[];
  isOrganizer: boolean;
  isHistorical: boolean;
  form: {
    adjUser: string;
    adjMode: "add" | "subtract";
    adjPoints: string;
    adjReason: string;
    adjError: string;
  };
  onFormChange: {
    setAdjUser: (value: string) => void;
    setAdjMode: (value: "add" | "subtract") => void;
    setAdjPoints: (value: string) => void;
    setAdjReason: (value: string) => void;
  };
  onSubmit: (event: FormEvent) => void;
  onRemove: (adjustmentId: string) => void;
  isAdding: boolean;
  isRemoving: boolean;
}
export function AdjustmentPanels(props: Props) {
  return (
    <>
      {props.isOrganizer && !props.isHistorical && props.entries.length > 0 && (
        <AdjustmentForm {...props} />
      )}
      {props.adjustments.length > 0 && <AdjustmentList {...props} />}
    </>
  );
}
function AdjustmentForm({
  entries,
  form,
  onFormChange,
  onSubmit,
  isAdding,
}: Props) {
  return (
    <Card className="mt-6 border-l-4 border-yellow-dark">
      <h2 className="text-xl">Ajustar pontos</h2>
      <p className="mt-1 text-sm text-ink-muted">
        Lance pontos manualmente para corrigir erros: escolha adicionar ou
        descontar e a quantidade. O ajuste e o motivo ficam visíveis para todos
        os participantes.
      </p>
      <form
        onSubmit={onSubmit}
        className="mt-3 grid gap-3 sm:grid-cols-[1fr_auto_auto_2fr_auto] sm:items-end"
      >
        <div>
          <Label htmlFor="adj-user">Membro</Label>
          <Select
            id="adj-user"
            value={form.adjUser}
            onChange={(event) => onFormChange.setAdjUser(event.target.value)}
          >
            <option value="">Selecione</option>
            {entries.map((entry) => (
              <option key={entry.userId} value={entry.userId}>
                {entry.username}
              </option>
            ))}
          </Select>
        </div>
        <div>
          <Label>Operação</Label>
          <div
            className="inline-flex gap-1 rounded-pill bg-secondary/40 p-1"
            role="group"
            aria-label="Tipo de ajuste"
          >
            <Button
              type="button"
              size="sm"
              variant={form.adjMode === "add" ? "primary" : "outline"}
              className={
                form.adjMode === "add"
                  ? ""
                  : "border-transparent bg-transparent"
              }
              aria-pressed={form.adjMode === "add"}
              onClick={() => onFormChange.setAdjMode("add")}
            >
              + Adicionar
            </Button>
            <Button
              type="button"
              size="sm"
              variant={form.adjMode === "subtract" ? "primary" : "outline"}
              className={
                form.adjMode === "subtract"
                  ? ""
                  : "border-transparent bg-transparent"
              }
              aria-pressed={form.adjMode === "subtract"}
              onClick={() => onFormChange.setAdjMode("subtract")}
            >
              − Descontar
            </Button>
          </div>
        </div>
        <div>
          <Label htmlFor="adj-points">Pontos</Label>
          <Input
            id="adj-points"
            type="number"
            inputMode="numeric"
            min={1}
            max={1000}
            placeholder="3"
            value={form.adjPoints}
            onChange={(event) => onFormChange.setAdjPoints(event.target.value)}
            className="w-24"
          />
        </div>
        <div>
          <Label htmlFor="adj-reason">Motivo (opcional)</Label>
          <Input
            id="adj-reason"
            placeholder="Ex.: erro de cadastro de placar"
            value={form.adjReason}
            maxLength={200}
            onChange={(event) => onFormChange.setAdjReason(event.target.value)}
          />
        </div>
        <Button
          type="submit"
          disabled={isAdding}
          className="self-start sm:self-auto"
        >
          {isAdding
            ? "Lançando..."
            : form.adjMode === "subtract"
              ? "Descontar"
              : "Adicionar"}
        </Button>
      </form>
      {form.adjError && (
        <div className="mt-3">
          <ErrorBanner>{form.adjError}</ErrorBanner>
        </div>
      )}
    </Card>
  );
}
function AdjustmentList({
  adjustments,
  isOrganizer,
  isHistorical,
  onRemove,
  isRemoving,
}: Props) {
  return (
    <Card className="mt-6">
      <h2 className="text-xl">Ajustes de pontos</h2>
      <ul className="mt-3 divide-y divide-mint/20">
        {adjustments.map((adjustment) => (
          <li
            key={adjustment.id}
            className="flex items-center justify-between gap-3 py-3"
          >
            <div className="min-w-0">
              <div className="font-heading font-semibold text-ink">
                {adjustment.username}{" "}
                <span
                  className={
                    adjustment.delta >= 0 ? "text-mint-dark" : "text-danger"
                  }
                >
                  {adjustment.delta >= 0
                    ? `+${adjustment.delta}`
                    : adjustment.delta}{" "}
                  pts
                </span>
              </div>
              {adjustment.reason && (
                <div className="truncate text-sm text-ink-muted">
                  {adjustment.reason}
                </div>
              )}
            </div>
            {isOrganizer && !isHistorical && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => onRemove(adjustment.id)}
                disabled={isRemoving}
                className="shrink-0"
              >
                <X className="h-4 w-4" /> Remover
              </Button>
            )}
          </li>
        ))}
      </ul>
    </Card>
  );
}
