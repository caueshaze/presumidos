import { ChevronDown, Image as ImageIcon } from "lucide-react";
import { AssetUploadControl } from "@/components/AssetUploadControl";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label, Select } from "@/components/ui/field";
import type { OptionLink } from "@/types";
import type { EventBuilderItemsProps, Item, Option } from "./types";

const optionLinkKinds: Array<{ value: OptionLink["kind"]; label: string }> = [
  { value: "video", label: "Vídeo" },
  { value: "audio", label: "Áudio" },
  { value: "official", label: "Link oficial" },
  { value: "other", label: "Outro" },
];

type OptionMediaState = Pick<
  EventBuilderItemsProps,
  | "draft"
  | "mediaEditable"
  | "busy"
  | "editingOptionId"
  | "openMediaOptionId"
  | "setOpenMediaOptionId"
  | "mediaDrafts"
  | "setMediaDrafts"
  | "load"
  | "saveOptionMedia"
>;

export function OptionMediaEditor({ item, option, state }: { item: Item; option: Option; state: OptionMediaState }) {
  const { draft, mediaEditable, busy, editingOptionId, openMediaOptionId, setOpenMediaOptionId, mediaDrafts, setMediaDrafts, load, saveOptionMedia } = state;

  return <>
                        {mediaEditable && editingOptionId === option.id && (() => {
                          const media = mediaDrafts[option.id] ?? { imageUrl: option.imageUrl ?? "", links: option.links ?? [] };
                          const mediaOpen = openMediaOptionId === option.id;
                          const hasMedia = Boolean(option.imageAssetUrl || option.imageUrl || option.links?.length);
                          return (
                            <>
                              <button
                                type="button"
                                title={hasMedia ? "Editar mídia configurada" : "Adicionar mídia opcional"}
                                aria-label={hasMedia ? "Editar mídia configurada" : "Adicionar mídia opcional"}
                                aria-expanded={mediaOpen}
                                className={`inline-flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-xs font-semibold transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mint-dark/40 ${mediaOpen ? "border-mint-dark bg-mint/25 text-mint-dark shadow-glow" : hasMedia ? "border-mint-dark/40 bg-mint/15 text-mint-dark hover:bg-mint/25" : "border-mint/20 bg-card/70 text-ink-muted hover:border-mint/40 hover:bg-mint/10 hover:text-mint-dark"}`}
                                onClick={() => setOpenMediaOptionId((current) => current === option.id ? null : option.id)}
                              >
                                <ImageIcon className="h-4 w-4" />
                                <span>Mídia</span>
                                <ChevronDown className={`h-3.5 w-3.5 transition-transform duration-200 ${mediaOpen ? "rotate-180" : ""}`} />
                              </button>
                              {mediaOpen && (
                                <div className="basis-full rounded-xl border border-mint/15 bg-card/40 p-3">
                                  <AssetUploadControl
                                    label={`Imagem da opçãoption ${option.label}`}
                                    currentUrl={option.imageAssetUrl ?? option.imageUrl}
                                    fallbackUrl={option.imageAssetUrl ? option.imageUrl : undefined}
                                    uploadPath={`/custom/events/${draft.event.id}/items/${item.id}/options/${option.id}/image`}
                                    removePath={`/custom/events/${draft.event.id}/items/${item.id}/options/${option.id}/image/remove`}
                                    disabled={!mediaEditable}
                                    compact
                                    onChanged={() => void load(draft.event.id)}
                                  />
                                  <p className="mt-2 text-xs text-ink-muted">URL externa (opcional)</p>
                                  <Input
                                    aria-label={`Imagem da opçãoption ${option.label}`}
                                    className="mt-2"
                                    placeholder="URL da imagem (https://…)"
                                    value={media.imageUrl}
                                    onChange={(event) => setMediaDrafts((current) => ({ ...current, [option.id]: { ...media, imageUrl: event.target.value } }))}
                                  />
                                  {media.links.map((link, linkIndex) => (
                                    <div className="mt-3 rounded-xl border border-mint/15 bg-card/30 p-3" key={`${option.id}-link-${linkIndex}`}>
                                      <div className="mb-3 flex items-center justify-between gap-3">
                                        <p className="text-sm font-semibold text-ink">Link editorial {linkIndex + 1}</p>
                                        <Button size="sm" variant="outline" onClick={() => setMediaDrafts((current) => ({ ...current, [option.id]: { ...media, links: media.links.filter((_, index) => index !== linkIndex) } }))}>Remover</Button>
                                      </div>
                                      <div className="grid gap-3 sm:grid-cols-[11rem_1fr]">
                                        <div>
                                          <Label htmlFor={`${option.id}-link-kind-${linkIndex}`}>Tipo de conteúdo</Label>
                                          <Select
                                            id={`${option.id}-link-kind-${linkIndex}`}
                                            aria-label={`Tipo de conteúdo do link ${option.label} ${linkIndex + 1}`}
                                            value={link.kind}
                                            onChange={(event) => setMediaDrafts((current) => ({ ...current, [option.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, kind: event.target.value as OptionLink["kind"] } : entry) } }))}
                                          >
                                            {optionLinkKinds.map((kind) => <option key={kind.value} value={kind.value}>{kind.label}</option>)}
                                          </Select>
                                        </div>
                                        <div>
                                          <Label htmlFor={`${option.id}-link-label-${linkIndex}`}>Nome exibido</Label>
                                          <Input id={`${option.id}-link-label-${linkIndex}`} aria-label={`Nome exibido do link ${option.label} ${linkIndex + 1}`} value={link.label} placeholder="Ex.: Ver vídeo oficial" onChange={(event) => setMediaDrafts((current) => ({ ...current, [option.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, label: event.target.value } : entry) } }))} />
                                        </div>
                                        <div className="sm:col-span-2">
                                          <Label htmlFor={`${option.id}-link-url-${linkIndex}`}>Endereçoption do link</Label>
                                          <Input id={`${option.id}-link-url-${linkIndex}`} aria-label={`Endereçoption do link ${option.label} ${linkIndex + 1}`} value={link.url} placeholder="Cole aqui uma URL começando com https://" onChange={(event) => setMediaDrafts((current) => ({ ...current, [option.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, url: event.target.value } : entry) } }))} />
                                        </div>
                                      </div>
                                    </div>
                                  ))}
                                  <div className="mt-2 flex flex-wrap gap-2">
                                    <Button size="sm" variant="outline" onClick={() => setMediaDrafts((current) => ({ ...current, [option.id]: { ...media, links: [...media.links, { kind: "other", label: "", url: "", sortOrder: media.links.length }] } }))}>Adicionar link</Button>
                                    <Button size="sm" variant="secondary" disabled={busy} onClick={() => void saveOptionMedia(item, option)}>Salvar mídia</Button>
                                  </div>
                                </div>
                              )}
                            </>
                          );
                        })()}
  </>;
}
