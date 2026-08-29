import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { EventVersionHistory } from "@/types";

type Props = { isAdmin: boolean; versions: EventVersionHistory[]; busy: boolean; restore: (version: EventVersionHistory) => Promise<void> };

export function VersionHistory({ isAdmin, versions, busy, restore }: Props) {
  if (!isAdmin || versions.filter((version) => version.state === "published").length <= 1) return null;
  return <Card className="mt-4 border-mint/20 bg-card/60"><h2 className="text-lg">Histórico de versões</h2><p className="mt-1 text-sm text-ink-muted">Restaure uma versão publicada como uma nova revisão. Pools existentes não serão alterados.</p><div className="mt-4 space-y-2">{versions.map((version) => <div key={version.id} className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-mint/15 bg-card/35 px-3 py-2.5"><div className="min-w-0"><p className="font-semibold">V{version.versionNumber} · {version.state === "working" ? "Revisão de trabalho" : version.isCurrentPublished ? "Publicada atual" : "Publicada"}</p><p className="text-xs text-ink-muted">{version.itemCount} perguntas · {version.optionCount} opções · {version.poolCount} Pools · atualizada em {version.updatedAt}</p></div>{version.state === "published" && !version.isCurrentPublished && <Button type="button" size="sm" variant="outline" className="rounded-lg whitespace-nowrap" disabled={busy} onClick={() => void restore(version)}>Restaurar como revisão</Button>}</div>)}</div></Card>;
}
