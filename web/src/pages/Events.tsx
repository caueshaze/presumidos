import { useNavigate } from "react-router-dom";
import { Plus } from "lucide-react";
import { useMyEvents } from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/states";
import { eventPresentationStatus, presentationStatusLabel } from "@/lib/lifecycle";

export function EventsPage() {
  const navigate = useNavigate(); const events = useMyEvents();
  return <PageShell><header className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between"><div><h1 className="text-3xl">Meus eventos</h1><p className="mt-1 text-ink-muted">Monte perguntas, publique seu evento e crie bolões com ele.</p></div><Button onClick={() => navigate("/events/new")}><Plus className="h-4 w-4" /> Criar evento</Button></header>{events.isLoading ? <div className="mt-6"><LoadingState label="Carregando seus eventos..." /></div> : events.isError ? <div className="mt-6"><ErrorState onRetry={() => void events.refetch()}>{(events.error as Error).message}</ErrorState></div> : events.data?.length ? <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">{events.data.map((event) => <Card key={event.id}><div className="flex items-start justify-between gap-3"><h2 className="text-xl">{event.name}</h2><span className="rounded-pill bg-mint/20 px-2.5 py-1 text-xs font-semibold">{presentationStatusLabel[eventPresentationStatus({ status: event.status, isHistorical: event.status === "finished" })]}</span></div><p className="mt-3 text-sm text-ink-muted">{event.status === "draft" ? "Continue montando perguntas e opções." : event.status === "finished" ? "Edição encerrada para consulta." : "Pronto para receber bolões."}</p><div className="mt-5 flex flex-wrap gap-2"><Button size="sm" onClick={() => navigate(`/events/${event.id}`)}>{event.status === "draft" ? "Continuar edição" : "Ver evento"}</Button>{event.status === "active" && <Button size="sm" variant="outline" onClick={() => navigate(`/dashboard?eventId=${event.id}&mode=create`)}>Criar bolão</Button>}</div></Card>)}</div> : <div className="mt-6"><EmptyState title="Você ainda não criou eventos." action={<Button onClick={() => navigate("/events/new")}>Criar evento</Button>}>Monte suas próprias perguntas e depois crie bolões com elas.</EmptyState></div>}</PageShell>;
}
