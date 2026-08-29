import { Check, ChevronDown, Pencil, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { EventBuilderItemsProps, Item } from "./types";
import { OptionMediaEditor } from "./OptionMediaEditor";

type ChoiceOptionsState = Pick<
  EventBuilderItemsProps,
  | "draft"
  | "editable"
  | "mediaEditable"
  | "busy"
  | "editingOptionId"
  | "optionLabelDraft"
  | "setOptionLabelDraft"
  | "openMediaOptionId"
  | "setOpenMediaOptionId"
  | "mediaDrafts"
  | "setMediaDrafts"
  | "openAddOptionItemId"
  | "setOpenAddOptionItemId"
  | "labels"
  | "setLabels"
  | "action"
  | "load"
  | "addOption"
  | "startOptionEdit"
  | "cancelOptionEdit"
  | "saveOptionLabel"
  | "saveOptionMedia"
>;

export function EventBuilderChoiceOptions({ item, state }: { item: Item; state: ChoiceOptionsState }) {
  const { draft, editable, mediaEditable, busy, editingOptionId, optionLabelDraft, setOptionLabelDraft, openMediaOptionId, setOpenMediaOptionId, mediaDrafts, setMediaDrafts, openAddOptionItemId, setOpenAddOptionItemId, labels, setLabels, action, load, addOption, startOptionEdit, cancelOptionEdit, saveOptionLabel, saveOptionMedia } = state;

  return <>
            {(item.kind === "single_choice" ||
              item.kind === "multiple_choice") && (
              <>
                <ol className="mt-4 space-y-3 pl-0">
                  {item.options.map((o, optionIndex) => (
                    <li key={o.id} className="flex items-start gap-3 rounded-xl border border-mint/20 bg-card/35 p-3 shadow-sm transition-colors hover:border-mint/35 hover:bg-card/55">
                      <span className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-mint/15 text-xs font-bold text-mint-dark">
                        {optionIndex + 1}
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-2">
                          {editingOptionId === o.id ? (
                            <div className="flex w-full min-w-0 flex-wrap items-center gap-2">
                              <Input
                                autoFocus
                                aria-label={`Nome da opção ${o.label}`}
                                className="min-w-0 flex-1 basis-full sm:basis-auto"
                                value={optionLabelDraft}
                                onChange={(event) => setOptionLabelDraft(event.target.value)}
                                onKeyDown={(event) => {
                                  if (event.key === "Enter") void saveOptionLabel(item, o);
                                  if (event.key === "Escape") cancelOptionEdit();
                                }}
                              />
                              <Button
                                type="button"
                                size="sm"
                                aria-label="Salvar nome da opção"
                                className="rounded-lg px-2.5"
                                disabled={busy}
                                onClick={() => void saveOptionLabel(item, o)}
                              >
                                <Check className="h-4 w-4" />
                              </Button>
                              <Button
                                type="button"
                                size="sm"
                                variant="outline"
                                aria-label="Cancelar edição do nome"
                                className="rounded-lg px-2.5"
                                disabled={busy}
                                onClick={cancelOptionEdit}
                              >
                                <X className="h-4 w-4" />
                              </Button>
                              {editable && (
                                <Button
                                  type="button"
                                  size="sm"
                                  variant="outline"
                                  className="rounded-lg border-danger/40 px-2.5 text-danger hover:border-danger hover:bg-danger/10"
                                  disabled={busy}
                                  onClick={() => {
                                    if (window.confirm(`Remover a opção "${o.label}"?`)) {
                                      void action(
                                        `/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/delete`,
                                      );
                                    }
                                  }}
                                >
                                  Remover
                                </Button>
                              )}
                            </div>
                          ) : (
                            <span className="font-medium text-ink">{o.label}</span>
                          )}
                        <OptionMediaEditor
                          item={item}
                          option={o}
                          state={{
                            draft, mediaEditable, busy, editingOptionId, openMediaOptionId,
                            setOpenMediaOptionId, mediaDrafts, setMediaDrafts, load, saveOptionMedia,
                          }}
                        />
                        </div>
                      </div>
                      {(editable || mediaEditable) && editingOptionId !== o.id && (
                        <span className="flex gap-1">
                          <Button
                            size="sm"
                            variant="outline"
                            className="rounded-lg border-transparent bg-transparent px-2 text-ink-muted shadow-none hover:border-transparent hover:bg-mint/10 hover:text-ink"
                            onClick={() => startOptionEdit(o)}
                          >
                            <Pencil className="h-3.5 w-3.5" />
                            Editar
                          </Button>
                          {editable && <Button
                            size="sm"
                            variant="outline"
                            disabled={optionIndex === 0}
                            onClick={() =>
                              action(
                                `/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/move`,
                                { direction: -1 },
                              )
                            }
                          >
                            ↑
                          </Button>}
                          {editable && <Button
                            size="sm"
                            variant="outline"
                            disabled={optionIndex === item.options.length - 1}
                            onClick={() =>
                              action(
                                `/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/move`,
                                { direction: 1 },
                              )
                            }
                          >
                            ↓
                          </Button>}
                        </span>
                      )}
                    </li>
                  ))}
                </ol>
                {editable && (
                  <div className="mt-3">
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      aria-expanded={openAddOptionItemId === item.id}
                      onClick={() => setOpenAddOptionItemId((current) => current === item.id ? null : item.id)}
                    >
                      <span>{openAddOptionItemId === item.id ? "Fechar" : "Adicionar opção"}</span>
                      <ChevronDown className={`h-4 w-4 transition-transform ${openAddOptionItemId === item.id ? "rotate-180" : ""}`} />
                    </Button>
                    {openAddOptionItemId === item.id && (
                      <div className="mt-2 flex gap-2 rounded-xl border border-mint/15 bg-card/35 p-3">
                        <Input
                          autoFocus
                          aria-label={`Nova opção para ${item.title}`}
                          placeholder="Nome da opção"
                          value={labels[item.id] ?? ""}
                          onChange={(e) =>
                            setLabels((v) => ({ ...v, [item.id]: e.target.value }))
                          }
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              e.preventDefault();
                              void addOption(item);
                            }
                          }}
                        />
                        <Button type="button" disabled={busy || !labels[item.id]?.trim()} onClick={() => void addOption(item)}>
                          Adicionar
                        </Button>
                      </div>
                    )}
                  </div>
                )}
              </>
            )}
  </>;
}
