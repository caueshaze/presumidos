import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Label, Select } from "@/components/ui/field";

export function TieBreakEditor({ mode: initialMode, items, selected: initialSelected, busy, save }: { mode: "inherit" | "custom" | "disabled"; items: { itemId: string; title: string }[]; selected: string[]; busy: boolean; save: (mode: "inherit" | "custom" | "disabled", itemIds: string[]) => Promise<unknown> }) {
  const [mode, setMode] = useState(initialMode);
  const [selected, setSelected] = useState(initialSelected);
  useEffect(() => { setMode(initialMode); setSelected(initialSelected); }, [initialMode, initialSelected]);
  const ordered = selected.map((id) => items.find((item) => item.itemId === id)).filter((item): item is (typeof items)[number] => Boolean(item));
  const available = items.filter((item) => !selected.includes(item.itemId));
  const toggle = (id: string) => setSelected((current) => current.includes(id) ? current.filter((item) => item !== id) : [...current, id]);
  const move = (index: number, direction: number) => setSelected((current) => { const next = [...current]; const target = index + direction; if (target < 0 || target >= next.length) return current; [next[index], next[target]] = [next[target], next[index]]; return next; });
  return <div className="mt-4 border-t border-mint/15 pt-4"><Label>Regra do Pool</Label><Select value={mode} onChange={(event) => setMode(event.target.value as typeof mode)}><option value="inherit">Usar padrão do evento</option><option value="custom">Personalizar</option><option value="disabled">Desativar</option></Select>{mode === "custom" && <div className="mt-3 space-y-2"><p className="text-xs text-ink-muted">A prioridade 1 é avaliada primeiro.</p>{ordered.map((item, index) => <div key={item.itemId} className="flex items-center gap-2 text-sm"><input type="checkbox" checked onChange={() => toggle(item.itemId)} /><span className="flex min-w-0 flex-1 items-center gap-2"><strong className="rounded-full bg-mint/20 px-2 py-0.5 text-mint-dark">{index + 1}º</strong>{item.title}</span><Button size="sm" variant="outline" disabled={index === 0} onClick={() => move(index, -1)}>Subir</Button><Button size="sm" variant="outline" disabled={index === ordered.length - 1} onClick={() => move(index, 1)}>Descer</Button></div>)}{available.map((item) => <label key={item.itemId} className="flex items-center gap-2 text-sm text-ink-muted"><input type="checkbox" onChange={() => toggle(item.itemId)} />{item.title}</label>)}</div>}<Button className="mt-4" disabled={busy || (mode === "custom" && selected.length === 0)} onClick={() => void save(mode, mode === "custom" ? selected : [])}>Salvar desempate</Button></div>;
}
