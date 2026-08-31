import { ChevronDown, ChevronUp, Plus, RotateCcw, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { ErrorState, LoadingState } from "@/components/ui/states";
import { usePoolEditorial, useReplacePoolOptionLinks, useResetPoolOptionLinks, useUpdatePoolEditorialName } from "@/hooks/queries";
import type { OptionLink, PoolEditorialOption } from "@/types";

type DraftLink = Omit<OptionLink, "sortOrder">;
const blankLink = (): DraftLink => ({ kind: "official", label: "", url: "" });

export function PoolEditorialPage() {
  const { poolId = "" } = useParams();
  const navigate = useNavigate();
  const editorial = usePoolEditorial(poolId || null);
  const updateName = useUpdatePoolEditorialName();
  const replaceLinks = useReplacePoolOptionLinks();
  const resetLinks = useResetPoolOptionLinks();
  const [name, setName] = useState("");
  const [openOption, setOpenOption] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<Record<string, DraftLink[]>>({});
  const [error, setError] = useState("");
  useEffect(() => { if (editorial.data) setName(editorial.data.name); }, [editorial.data]);
  const grouped = useMemo(() => {
    const groups = new Map<string, PoolEditorialOption[]>();
    editorial.data?.options.forEach((option) => groups.set(option.itemTitle, [...(groups.get(option.itemTitle) ?? []), option]));
    return [...groups.entries()];
  }, [editorial.data]);
  const draftFor = (option: PoolEditorialOption) => drafts[option.optionId] ?? option.links.map(({ kind, label, url }) => ({ kind: kind as DraftLink["kind"], label, url }));
  const saveName = async (event: FormEvent) => { event.preventDefault(); setError(""); try { await updateName.mutateAsync({ poolId, name }); } catch (cause) { setError(cause instanceof Error ? cause.message : "Não foi possível salvar o nome."); } };
  const saveLinks = async (option: PoolEditorialOption) => { setError(""); try { await replaceLinks.mutateAsync({ poolId, optionId: option.optionId, links: draftFor(option) }); setDrafts((all) => { const next = { ...all }; delete next[option.optionId]; return next; }); } catch (cause) { setError(cause instanceof Error ? cause.message : "Não foi possível salvar os links."); } };
  const reset = async (option: PoolEditorialOption) => { setError(""); try { await resetLinks.mutateAsync({ poolId, optionId: option.optionId }); setDrafts((all) => { const next = { ...all }; delete next[option.optionId]; return next; }); } catch (cause) { setError(cause instanceof Error ? cause.message : "Não foi possível restaurar os links."); } };
  const changeDraft = (id: string, update: (links: DraftLink[]) => DraftLink[]) => setDrafts((all) => ({ ...all, [id]: update(drafts[id] ?? editorial.data?.options.find((option) => option.optionId === id)?.links.map(({ kind, label, url }) => ({ kind: kind as DraftLink["kind"], label, url })) ?? []) }));
  if (editorial.isLoading) return <main className="mx-auto max-w-[1100px] px-5 py-10"><LoadingState label="Carregando personalização..." /></main>;
  if (editorial.isError || !editorial.data) return <main className="mx-auto max-w-[1100px] px-5 py-10"><ErrorState onRetry={() => void editorial.refetch()}>Você não pode personalizar este bolão.</ErrorState></main>;
  return <main className="mx-auto max-w-[800px] px-5 py-8 sm:py-12"><Button variant="link" size="sm" onClick={() => navigate(`/pools/${poolId}`)}>← Voltar ao bolão</Button><header className="mt-4"><h1 className="text-3xl">Personalizar bolão</h1><p className="mt-2 text-ink-muted">Essas mudanças valem apenas neste bolão e ficam congeladas ao encerrar os palpites.</p></header>{error && <div className="mt-4"><ErrorBanner>{error}</ErrorBanner></div>}<Card className="mt-6"><h2 className="text-xl">Nome do bolão</h2><form onSubmit={saveName} className="mt-4 flex flex-col gap-3 sm:flex-row"><Input value={name} onChange={(event) => setName(event.target.value)} aria-label="Nome do bolão" required minLength={3} maxLength={80} /><Button type="submit" disabled={updateName.isPending}>{updateName.isPending ? "Salvando..." : "Salvar nome"}</Button></form></Card><section className="mt-6"><h2 className="text-xl">Links editoriais</h2><p className="mt-1 text-sm text-ink-muted">Personalize os links de uma opção sem alterar o evento original.</p>{grouped.length === 0 ? <Card className="mt-4"><p className="text-ink-muted">Este bolão não possui opções com links editoriais.</p></Card> : <div className="mt-4 space-y-3">{grouped.map(([title, options]) => <Card key={title}><h3 className="font-semibold">{title}</h3><div className="mt-3 space-y-2">{options.map((option) => <OptionEditor key={option.optionId} option={option} open={openOption === option.optionId} links={draftFor(option)} pending={replaceLinks.isPending || resetLinks.isPending} onToggle={() => setOpenOption((current) => current === option.optionId ? null : option.optionId)} onChange={(update) => changeDraft(option.optionId, update)} onSave={() => void saveLinks(option)} onReset={() => void reset(option)} />)}</div></Card>)}</div>}</section></main>;
}

function OptionEditor({ option, open, links, pending, onToggle, onChange, onSave, onReset }: { option: PoolEditorialOption; open: boolean; links: DraftLink[]; pending: boolean; onToggle: () => void; onChange: (update: (links: DraftLink[]) => DraftLink[]) => void; onSave: () => void; onReset: () => void }) {
  return <div className="rounded-xl border border-mint/15 p-3"><div className="flex items-center justify-between gap-3"><div><p className="font-semibold">{option.optionLabel}</p><p className="text-xs text-ink-muted">{option.isCustomized ? "Personalizado neste bolão" : "Padrão do evento"}</p></div><Button variant="outline" size="sm" onClick={onToggle}>{open ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}{open ? "Fechar" : "Editar"}</Button></div>{open && <div className="mt-4 space-y-3 border-t border-mint/15 pt-4">{links.map((link, index) => <div key={index} className="rounded-xl bg-bg/45 p-3"><div className="grid gap-2 sm:grid-cols-[150px_1fr]"><Select value={link.kind} onChange={(event) => onChange((all) => all.map((value, i) => i === index ? { ...value, kind: event.target.value as DraftLink["kind"] } : value))}><option value="official">Link oficial</option><option value="video">Vídeo</option><option value="audio">Áudio</option><option value="other">Outro</option></Select><Input value={link.label} placeholder="Rótulo" onChange={(event) => onChange((all) => all.map((value, i) => i === index ? { ...value, label: event.target.value } : value))} /></div><Input className="mt-2" value={link.url} type="url" placeholder="https://..." onChange={(event) => onChange((all) => all.map((value, i) => i === index ? { ...value, url: event.target.value } : value))} /><div className="mt-2 flex gap-2"><Button type="button" variant="link" size="sm" disabled={index === 0} onClick={() => onChange((all) => { const next = [...all]; [next[index - 1], next[index]] = [next[index], next[index - 1]]; return next; })}>Subir</Button><Button type="button" variant="link" size="sm" disabled={index === links.length - 1} onClick={() => onChange((all) => { const next = [...all]; [next[index + 1], next[index]] = [next[index], next[index + 1]]; return next; })}>Descer</Button><Button type="button" variant="link" size="sm" className="text-danger" onClick={() => onChange((all) => all.filter((_, i) => i !== index))}><Trash2 className="h-3.5 w-3.5" />Remover</Button></div></div>)}<Button type="button" variant="outline" size="sm" onClick={() => onChange((all) => [...all, blankLink()])}><Plus className="h-4 w-4" />Adicionar link</Button><div className="flex flex-col gap-2 pt-1 sm:flex-row"><Button type="button" onClick={onSave} disabled={pending}>{pending ? "Salvando..." : "Salvar links"}</Button>{option.isCustomized && <Button type="button" variant="outline" onClick={onReset} disabled={pending}><RotateCcw className="h-4 w-4" />Restaurar padrão</Button>}</div></div>}</div>;
}
