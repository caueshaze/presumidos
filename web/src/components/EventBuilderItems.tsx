import { Check, ChevronDown, Image as ImageIcon, Pencil, Trophy, X } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label, Select } from "@/components/ui/field";
import { AssetUploadControl } from "@/components/AssetUploadControl";
import { PtBrDateTimeInput, formatPtBrDateTime } from "@/components/PtBrDateTimeInput";
import type { OptionLink } from "@/types";

export type Option = {
  id: string;
  label: string;
  imageUrl?: string | null;
  imageAssetUrl?: string | null;
  links?: OptionLink[];
};

export type Item = {
  id: string;
  kind: "single_choice" | "numeric" | "multiple_choice";
  title: string;
  lockAt: string;
  revealAt: string;
  correctOptionId: string | null;
  options: Option[];
  decimalPlaces?: number;
  unitLabel?: string | null;
  minValue?: string | null;
  maxValue?: string | null;
  resultValue?: string | null;
  minSelections?: number;
  maxSelections?: number | null;
};

type MediaDraft = { imageUrl: string; links: OptionLink[] };
type Action = (path: string, body?: unknown) => Promise<boolean>;
type SetState<T> = React.Dispatch<React.SetStateAction<T>>;

type Props = {
  draft: { event: { id: string }; items: Item[] };
  editable: boolean;
  mediaEditable: boolean;
  busy: boolean;
  editingItemId: string | null;
  itemTitleDraft: string;
  setItemTitleDraft: SetState<string>;
  itemLockDraft: string;
  setItemLockDraft: SetState<string>;
  editingOptionId: string | null;
  optionLabelDraft: string;
  setOptionLabelDraft: SetState<string>;
  openMediaOptionId: string | null;
  setOpenMediaOptionId: SetState<string | null>;
  mediaDrafts: Record<string, MediaDraft>;
  setMediaDrafts: SetState<Record<string, MediaDraft>>;
  openAddOptionItemId: string | null;
  setOpenAddOptionItemId: SetState<string | null>;
  labels: Record<string, string>;
  setLabels: SetState<Record<string, string>>;
  results: Record<string, string>;
  setResults: SetState<Record<string, string>>;
  multipleResults: Record<string, string[]>;
  setMultipleResults: SetState<Record<string, string[]>>;
  action: Action;
  load: (id: string) => Promise<void>;
  addOption: (item: Item) => Promise<void>;
  startItemEdit: (item: Item) => void;
  cancelItemEdit: () => void;
  saveItemEdit: (item: Item) => Promise<void>;
  startOptionEdit: (option: Option) => void;
  cancelOptionEdit: () => void;
  saveOptionLabel: (item: Item, option: Option) => Promise<void>;
  saveOptionMedia: (item: Item, option: Option) => Promise<void>;
};

const optionLinkKinds: Array<{ value: OptionLink["kind"]; label: string }> = [
  { value: "video", label: "Vídeo" },
  { value: "audio", label: "Áudio" },
  { value: "official", label: "Link oficial" },
  { value: "other", label: "Outro" },
];

export function EventBuilderItems({
  draft,
  editable,
  mediaEditable,
  busy,
  editingItemId,
  itemTitleDraft,
  setItemTitleDraft,
  itemLockDraft,
  setItemLockDraft,
  editingOptionId,
  optionLabelDraft,
  setOptionLabelDraft,
  openMediaOptionId,
  setOpenMediaOptionId,
  mediaDrafts,
  setMediaDrafts,
  openAddOptionItemId,
  setOpenAddOptionItemId,
  labels,
  setLabels,
  results,
  setResults,
  multipleResults,
  setMultipleResults,
  action,
  load,
  addOption,
  startItemEdit,
  cancelItemEdit,
  saveItemEdit,
  startOptionEdit,
  cancelOptionEdit,
  saveOptionLabel,
  saveOptionMedia,
}: Props) {
  return (
      <div className="mt-4 flex flex-col gap-4">
        {draft.items.map((item, index) => (
          <Card key={item.id}>
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
                        {mediaEditable && editingOptionId === o.id && (() => {
                          const media = mediaDrafts[o.id] ?? { imageUrl: o.imageUrl ?? "", links: o.links ?? [] };
                          const mediaOpen = openMediaOptionId === o.id;
                          const hasMedia = Boolean(o.imageAssetUrl || o.imageUrl || o.links?.length);
                          return (
                            <>
                              <button
                                type="button"
                                title={hasMedia ? "Editar mídia configurada" : "Adicionar mídia opcional"}
                                aria-label={hasMedia ? "Editar mídia configurada" : "Adicionar mídia opcional"}
                                aria-expanded={mediaOpen}
                                className={`inline-flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-xs font-semibold transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mint-dark/40 ${mediaOpen ? "border-mint-dark bg-mint/25 text-mint-dark shadow-glow" : hasMedia ? "border-mint-dark/40 bg-mint/15 text-mint-dark hover:bg-mint/25" : "border-mint/20 bg-card/70 text-ink-muted hover:border-mint/40 hover:bg-mint/10 hover:text-mint-dark"}`}
                                onClick={() => setOpenMediaOptionId((current) => current === o.id ? null : o.id)}
                              >
                                <ImageIcon className="h-4 w-4" />
                                <span>Mídia</span>
                                <ChevronDown className={`h-3.5 w-3.5 transition-transform duration-200 ${mediaOpen ? "rotate-180" : ""}`} />
                              </button>
                              {mediaOpen && (
                                <div className="basis-full rounded-xl border border-mint/15 bg-card/40 p-3">
                                  <AssetUploadControl
                                    label={`Imagem da opção ${o.label}`}
                                    currentUrl={o.imageAssetUrl ?? o.imageUrl}
                                    fallbackUrl={o.imageAssetUrl ? o.imageUrl : undefined}
                                    uploadPath={`/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/image`}
                                    removePath={`/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/image/remove`}
                                    disabled={!mediaEditable}
                                    compact
                                    onChanged={() => void load(draft.event.id)}
                                  />
                                  <p className="mt-2 text-xs text-ink-muted">URL externa (opcional)</p>
                                  <Input
                                    aria-label={`Imagem da opção ${o.label}`}
                                    className="mt-2"
                                    placeholder="URL da imagem (https://…)"
                                    value={media.imageUrl}
                                    onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, imageUrl: event.target.value } }))}
                                  />
                                  {media.links.map((link, linkIndex) => (
                                    <div className="mt-3 rounded-xl border border-mint/15 bg-card/30 p-3" key={`${o.id}-link-${linkIndex}`}>
                                      <div className="mb-3 flex items-center justify-between gap-3">
                                        <p className="text-sm font-semibold text-ink">Link editorial {linkIndex + 1}</p>
                                        <Button size="sm" variant="outline" onClick={() => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.filter((_, index) => index !== linkIndex) } }))}>Remover</Button>
                                      </div>
                                      <div className="grid gap-3 sm:grid-cols-[11rem_1fr]">
                                        <div>
                                          <Label htmlFor={`${o.id}-link-kind-${linkIndex}`}>Tipo de conteúdo</Label>
                                          <Select
                                            id={`${o.id}-link-kind-${linkIndex}`}
                                            aria-label={`Tipo de conteúdo do link ${o.label} ${linkIndex + 1}`}
                                            value={link.kind}
                                            onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, kind: event.target.value as OptionLink["kind"] } : entry) } }))}
                                          >
                                            {optionLinkKinds.map((kind) => <option key={kind.value} value={kind.value}>{kind.label}</option>)}
                                          </Select>
                                        </div>
                                        <div>
                                          <Label htmlFor={`${o.id}-link-label-${linkIndex}`}>Nome exibido</Label>
                                          <Input id={`${o.id}-link-label-${linkIndex}`} aria-label={`Nome exibido do link ${o.label} ${linkIndex + 1}`} value={link.label} placeholder="Ex.: Ver vídeo oficial" onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, label: event.target.value } : entry) } }))} />
                                        </div>
                                        <div className="sm:col-span-2">
                                          <Label htmlFor={`${o.id}-link-url-${linkIndex}`}>Endereço do link</Label>
                                          <Input id={`${o.id}-link-url-${linkIndex}`} aria-label={`Endereço do link ${o.label} ${linkIndex + 1}`} value={link.url} placeholder="Cole aqui uma URL começando com https://" onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, url: event.target.value } : entry) } }))} />
                                        </div>
                                      </div>
                                    </div>
                                  ))}
                                  <div className="mt-2 flex flex-wrap gap-2">
                                    <Button size="sm" variant="outline" onClick={() => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: [...media.links, { kind: "other", label: "", url: "", sortOrder: media.links.length }] } }))}>Adicionar link</Button>
                                    <Button size="sm" variant="secondary" disabled={busy} onClick={() => void saveOptionMedia(item, o)}>Salvar mídia</Button>
                                  </div>
                                </div>
                              )}
                            </>
                          );
                        })()}
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
                {!editable && item.kind === "single_choice" && (
                  <div className="mt-5 rounded-xl border border-sky/25 bg-sky/5 p-4">
                    <div className="flex items-start gap-3">
                      <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-sky/15 text-sky-dark">
                        <Trophy className="h-4 w-4" />
                      </span>
                      <div>
                        <p className="font-semibold text-ink">Resultado oficial</p>
                        <p className="mt-0.5 text-xs text-ink-muted">Selecione o vencedor desta pergunta para fechar a apuração.</p>
                      </div>
                    </div>
                    <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:items-center">
                    <Select
                      aria-label={`Resultado oficial: ${item.title}`}
                      value={results[item.id] ?? item.correctOptionId ?? ""}
                      onChange={(e) =>
                        setResults((v) => ({ ...v, [item.id]: e.target.value }))
                      }
                    >
                      <option value="">Selecione o vencedor</option>
                      {item.options.map((o) => (
                        <option key={o.id} value={o.id}>
                          {o.label}
                        </option>
                      ))}
                    </Select>
                    <Button
                      size="sm"
                      className="w-full sm:w-auto"
                      disabled={!(results[item.id] ?? item.correctOptionId)}
                      onClick={() =>
                        action(`/admin/custom/questions/${item.id}/result`, {
                          optionId: results[item.id] ?? item.correctOptionId,
                        })
                      }
                    >
                      <Check className="h-4 w-4" />
                      Salvar resultado
                    </Button>
                    </div>
                  </div>
                )}
                {!editable && item.kind === "multiple_choice" && (
                  <div className="mt-3 space-y-2">
                    <p className="text-sm font-semibold">Resultado oficial</p>
                    {item.options.map((option) => {
                      const selected = multipleResults[item.id] ?? [];
                      return (
                        <label
                          key={option.id}
                          className="flex items-center gap-2"
                        >
                          <input
                            type="checkbox"
                            checked={selected.includes(option.id)}
                            onChange={() =>
                              setMultipleResults((all) => ({
                                ...all,
                                [item.id]: selected.includes(option.id)
                                  ? selected.filter((id) => id !== option.id)
                                  : [...selected, option.id],
                              }))
                            }
                          />
                          {option.label}
                        </label>
                      );
                    })}
                    <Button
                      size="sm"
                      disabled={
                        (multipleResults[item.id]?.length ?? 0) <
                        (item.minSelections ?? 1)
                      }
                      onClick={() =>
                        action(
                          `/admin/custom/multiple-choice/${item.id}/result`,
                          { optionIds: multipleResults[item.id] ?? [] },
                        )
                      }
                    >
                      Salvar resultado
                    </Button>
                  </div>
                )}
              </>
            )}
            {item.kind === "numeric" && !editable && (
              <div className="mt-3 flex gap-2">
                <Input
                  inputMode="decimal"
                  aria-label={`Resultado oficial: ${item.title}`}
                  value={results[item.id] ?? item.resultValue ?? ""}
                  onChange={(e) =>
                    setResults((v) => ({ ...v, [item.id]: e.target.value }))
                  }
                />
                <Button
                  size="sm"
                  disabled={!(results[item.id] ?? item.resultValue)}
                  onClick={() =>
                    action(`/admin/custom/numeric/${item.id}/result`, {
                      value: results[item.id] ?? item.resultValue,
                    })
                  }
                >
                  Salvar resultado
                </Button>
              </div>
            )}
          </Card>
        ))}
      </div>
  );
}
