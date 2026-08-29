import { formatKickoff } from "@/lib/utils";
import { AdminManifestPanel } from "@/components/AdminManifestPanel";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";
import type { AdminEventRecord } from "@/types";

type Props = {
  events: AdminEventRecord[] | undefined;
  isLoading: boolean;
  isError: boolean;
  onApplied: () => void;
  onDownloadManifest: (eventId: string, slug: string) => void | Promise<void>;
  onDownloadPackage: (eventId: string, slug: string) => void | Promise<void>;
  onOpen: (eventId: string) => void;
  onPublish: (eventId: string, versionId: string) => void | Promise<void>;
  publishPending: boolean;
  onSetPoolCreation: (eventId: string, enabled: boolean) => void | Promise<void>;
  poolCreationPending: boolean;
  onFinish: (eventId: string, name: string) => void | Promise<void>;
  finishPending: boolean;
  onDelete: (event: AdminEventRecord) => void | Promise<void>;
  deletePending: boolean;
};

export function AdminEventsPanel({
  events,
  isLoading,
  isError,
  onApplied,
  onDownloadManifest,
  onDownloadPackage,
  onOpen,
  onPublish,
  publishPending,
  onSetPoolCreation,
  poolCreationPending,
  onFinish,
  finishPending,
  onDelete,
  deletePending,
}: Props) {
  return (

        <div className="mt-6 space-y-4">
          <AdminManifestPanel onApplied={() => void onApplied()} />
          <Card>
            <h2 className="text-xl">Edições do Presumidos</h2>
            <p className="mt-1 text-sm text-ink-muted">Eventos customizados podem ser exportados para promoção ou reabertos no Builder quando ainda forem drafts.</p>
          </Card>
          {isLoading ? <Card><p className="text-ink-muted">Carregando edições...</p></Card> : isError ? <ErrorBanner>Não foi possível carregar as edições.</ErrorBanner> : events?.map((event: AdminEventRecord) => {
            const historical = event.status === "finished" || (event.endsAt != null && new Date(event.endsAt).getTime() <= Date.now());
            return <Card key={event.id} className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between"><div><h3 className="text-lg">{event.name}</h3><p className="mt-1 text-sm text-ink-muted">{event.kind === "football" ? "Futebol" : "Evento customizado"} · {event.slug}{event.endsAt ? ` · termina em ${formatKickoff(event.endsAt)}` : " · sem data de término"}</p><p className="mt-2 text-sm font-semibold text-mint-dark">{event.archivedAt ? "Arquivado" : historical ? "Encerrado / histórico" : event.status === "draft" ? "Rascunho" : "Em andamento"}</p><p className="mt-1 text-xs text-ink-muted">Origem: {event.origin === "system" ? "Padrão Presumidos" : `Criado por @${event.createdByUsername ?? "usuário"}`} · {event.itemCount} perguntas · {event.optionCount} opções · {event.poolCount} pools</p><p className="mt-1 text-xs text-ink-muted">Versão publicada: {event.currentVersionNumber ? `V${event.currentVersionNumber}` : "nenhuma"} · {event.workingVersionId ? "revisão pendente" : "sem revisão pendente"}</p><p className="mt-1 text-xs text-ink-muted">Atualizado em {formatKickoff(event.updatedAt)}</p></div><div className="flex flex-wrap gap-2">{!event.archivedAt && event.kind === "custom" && <Button size="sm" variant="outline" onClick={() => void onDownloadManifest(event.id, event.slug)}>Exportar JSON</Button>}{!event.archivedAt && event.kind === "custom" && <Button size="sm" variant="outline" onClick={() => void onDownloadPackage(event.id, event.slug)}>Exportar pacote</Button>}{!event.archivedAt && event.kind === "custom" && <Button size="sm" variant="outline" onClick={() => onOpen(event.id)}>{event.status === "draft" ? "Abrir Builder" : "Abrir / editar"}</Button>}{!event.archivedAt && event.kind === "custom" && event.workingVersionId && <Button size="sm" onClick={() => void onPublish(event.id, event.workingVersionId!)} disabled={publishPending}>Publicar revisão</Button>}{!event.archivedAt && event.kind === "custom" && <Button size="sm" variant="outline" onClick={() => void onSetPoolCreation(event.id, !event.poolCreationEnabled)} disabled={poolCreationPending}>{event.poolCreationEnabled ? "Desativar novos pools" : "Permitir novos pools"}</Button>}{event.archivedAt ? <span className="rounded-pill bg-mint/25 px-3 py-1 text-sm font-semibold">Arquivado</span> : historical ? <span className="rounded-pill bg-mint/25 px-3 py-1 text-sm font-semibold">Encerrado</span> : <Button variant="outline" onClick={() => onFinish(event.id, event.name)} disabled={finishPending || !event.endsAt}>Encerrar edição</Button>} {!event.archivedAt && <Button variant="outline" className="text-danger" onClick={() => void onDelete(event)} disabled={deletePending}>{event.status === "draft" && event.poolCount === 0 ? "Excluir evento" : "Arquivar evento"}</Button>}</div></Card>;
          })}
        </div>
  );
}

