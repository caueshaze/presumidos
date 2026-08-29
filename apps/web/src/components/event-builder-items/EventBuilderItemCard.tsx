import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PtBrDateTimeInput, formatPtBrDateTime } from "@/components/PtBrDateTimeInput";
import { EventBuilderChoiceOptions } from "./EventBuilderChoiceOptions";
import { EventBuilderOfficialResult } from "./EventBuilderOfficialResult";
import type { EventBuilderItemsProps, Item } from "./types";

export function EventBuilderItemCard({ item, index, state }: { item: Item; index: number; state: EventBuilderItemsProps }) {
  const { draft, editable, mediaEditable, busy, editingItemId, itemTitleDraft, setItemTitleDraft, itemLockDraft, setItemLockDraft, editingOptionId, optionLabelDraft, setOptionLabelDraft, openMediaOptionId, setOpenMediaOptionId, mediaDrafts, setMediaDrafts, openAddOptionItemId, setOpenAddOptionItemId, labels, setLabels, results, setResults, multipleResults, setMultipleResults, action, load, addOption, startItemEdit, cancelItemEdit, saveItemEdit, startOptionEdit, cancelOptionEdit, saveOptionLabel, saveOptionMedia } = state;

  return (
    <Card>

            <div className="flex items-start justify-between gap-3">
              {editingItemId === item.id ? (
                <div className="min-w-0 flex-1 space-y-2">
                  <Input
                    autoFocus
                    aria-label={`Nome da pergunta ${item.title}`}
                    value={itemTitleDraft}
                    onChange={(event) => setItemTitleDraft(event.target.value)}
                  />
                  <div className="grid gap-2">
                    <label className="text-sm text-ink-muted">Fecha palpites em<PtBrDateTimeInput value={itemLockDraft} onChange={setItemLockDraft} /></label>
                  </div>
                </div>
              ) : (
                <div>
                  <h2 className="text-xl">{item.title}</h2>
                  <p className="text-sm text-ink-muted">
                    {item.kind === "numeric"
                      ? `Número${item.unitLabel ? ` · ${item.unitLabel}` : ""} · ${item.decimalPlaces ?? 0} casas`
                      : item.kind === "multiple_choice"
                        ? `Múltipla escolha · ${item.minSelections ?? 1}–${item.maxSelections ?? item.options.length} opções`
                        : "Escolha única"}{" "}
                    · Fecha: {formatPtBrDateTime(item.lockAt)}
                  </p>
                </div>
              )}
              {editable && (
                <div className="flex shrink-0 flex-wrap items-start justify-end gap-1.5">
                  {editingItemId === item.id ? (
                    <>
                      <Button size="sm" className="rounded-lg whitespace-nowrap" disabled={busy} onClick={() => void saveItemEdit(item)}>
                        Salvar
                      </Button>
                      <Button size="sm" variant="outline" className="rounded-lg whitespace-nowrap" disabled={busy} onClick={cancelItemEdit}>
                        Cancelar
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        className="rounded-lg border-danger/40 text-danger hover:border-danger hover:bg-danger/10"
                        disabled={busy}
                        onClick={() => {
                          if (window.confirm(`Remover a pergunta "${item.title}"?`)) {
                            void action(
                              `/custom/events/${draft.event.id}/items/${item.id}/delete`,
                            ).then(cancelItemEdit);
                          }
                        }}
                      >
                        Remover
                      </Button>
                    </>
                  ) : (
                    <>
                      <Button
                        size="sm"
                        variant="outline"
                        className="rounded-lg whitespace-nowrap"
                        disabled={busy}
                        onClick={() => startItemEdit(item)}
                      >
                        Editar
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        className="rounded-lg px-2"
                        disabled={busy || index === 0}
                        onClick={() =>
                          action(
                            `/custom/events/${draft.event.id}/items/${item.id}/move`,
                            { direction: -1 },
                          )
                        }
                      >
                        ↑
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        className="rounded-lg px-2"
                        disabled={busy || index === draft.items.length - 1}
                        onClick={() =>
                          action(
                            `/custom/events/${draft.event.id}/items/${item.id}/move`,
                            { direction: 1 },
                          )
                        }
                      >
                        ↓
                      </Button>
                    </>
                  )}
                </div>
              )}
            </div>
            <EventBuilderChoiceOptions
              item={item}
              state={{
                draft, editable, mediaEditable, busy, editingOptionId, optionLabelDraft,
                setOptionLabelDraft, openMediaOptionId, setOpenMediaOptionId, mediaDrafts,
                setMediaDrafts, openAddOptionItemId, setOpenAddOptionItemId, labels, setLabels,
                action, load, addOption, startOptionEdit, cancelOptionEdit, saveOptionLabel,
                saveOptionMedia,
              }}
            />
            {!editable && <EventBuilderOfficialResult item={item} state={{ action, results, setResults, multipleResults, setMultipleResults }} />}


    </Card>
  );
}
