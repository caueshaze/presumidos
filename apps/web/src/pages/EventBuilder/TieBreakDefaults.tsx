import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ChevronDown } from "lucide-react";
import { useState } from "react";

export function EventTieBreakDefaults({ items, busy, save }: { items: { id: string; title: string; tieBreakPriority?: number | null }[]; busy: boolean; save: (itemIds: string[]) => Promise<boolean> }) {
  const [open, setOpen] = useState(false);
  const selected = items.filter((item) => item.tieBreakPriority != null).sort((a, b) => (a.tieBreakPriority ?? 0) - (b.tieBreakPriority ?? 0)).map((item) => item.id);
  const ordered = selected.map((id) => items.find((item) => item.id === id)).filter((item): item is (typeof items)[number] => Boolean(item));
  const available = items.filter((item) => !selected.includes(item.id));
  const set = (next: string[]) => void save(next);
  const toggle = (id: string) => set(selected.includes(id) ? selected.filter((item) => item !== id) : [...selected, id]);
  const move = (index: number, direction: number) => { const next = [...selected]; const target = index + direction; if (target < 0 || target >= next.length) return; [next[index], next[target]] = [next[target], next[index]]; set(next); };
  return <Card className="mt-5"><button type="button" className="flex w-full items-center justify-between gap-3 text-left" aria-expanded={open} onClick={() => setOpen((current) => !current)}><span><span className="block text-xl font-heading font-semibold">Padrão de desempate</span><span className="mt-1 block text-sm text-ink-muted">{ordered.length ? `${ordered.length} pergunta${ordered.length === 1 ? "" : "s"} definida${ordered.length === 1 ? "" : "s"}` : "Nenhuma pergunta definida"}</span></span><ChevronDown className={`h-5 w-5 shrink-0 text-ink-muted transition-transform ${open ? "rotate-180" : ""}`} /></button>{open && <div className="mt-4 border-t border-mint/15 pt-4"><p className="text-sm text-ink-muted">A prioridade 1 é avaliada primeiro. Cada pergunta vale apenas por acerto exato.</p><div className="mt-4 space-y-2">{ordered.map((item, index) => <div key={item.id} className="flex items-center gap-2 text-sm"><input aria-label={`Remover ${item.title} do desempate`} type="checkbox" checked disabled={busy} onChange={() => toggle(item.id)} /><span className="flex min-w-0 flex-1 items-center gap-2"><strong className="rounded-full bg-mint/20 px-2 py-0.5 text-mint-dark">{index + 1}º</strong>{item.title}</span><Button size="sm" variant="outline" disabled={busy || index === 0} onClick={() => move(index, -1)}>Subir</Button><Button size="sm" variant="outline" disabled={busy || index === ordered.length - 1} onClick={() => move(index, 1)}>Descer</Button></div>)}{available.map((item) => <label key={item.id} className="flex items-center gap-2 text-sm text-ink-muted"><input aria-label={`Usar ${item.title} no desempate`} type="checkbox" disabled={busy} onChange={() => toggle(item.id)} />{item.title}</label>)}</div></div>}</Card>;
}
