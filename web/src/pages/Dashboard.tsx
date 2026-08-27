import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { AnimatePresence, motion } from "framer-motion";
import { Plus, Ticket, X } from "lucide-react";
import { useAvailableEvents, useCreatePool, useDashboardPools, useJoinPool, useMyEvents, type MyEvent } from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card, MotionCard } from "@/components/ui/card";
import { ErrorBanner, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { EmptyState, ErrorState, LoadingState, ProgressBar } from "@/components/ui/states";
import { useAuth } from "@/hooks/useAuth";
import { poolPresentationStatus, presentationStatusLabel } from "@/lib/lifecycle";
import { cn } from "@/lib/utils";

type Mode = "create" | "join" | null;
type DashboardPool = NonNullable<ReturnType<typeof useDashboardPools>["data"]>[number];

export function DashboardPage() {
  const navigate = useNavigate();
  const [search] = useSearchParams();
  const { user } = useAuth();
  const pools = useDashboardPools();
  const events = useMyEvents();
  const availableEvents = useAvailableEvents();
  const createPool = useCreatePool();
  const joinPool = useJoinPool();
  const [mode, setMode] = useState<Mode>(null);
  const [newPoolName, setNewPoolName] = useState("");
  const [chosenEventId, setChosenEventId] = useState(search.get("eventId") ?? "");
  const [joinCode, setJoinCode] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    const invite = search.get("invite")?.trim().toUpperCase();
    const requestedMode = search.get("mode");
    if (invite && /^[A-Z0-9]{6}$/.test(invite)) {
      setJoinCode(invite);
      setMode("join");
    } else if (requestedMode === "create" || requestedMode === "join") {
      setMode(requestedMode);
    }
  }, [search]);

  const openMode = (next: Exclude<Mode, null>) => {
    setError("");
    if (next === "create") {
      void availableEvents.refetch();
      void events.refetch();
    }
    setMode((current) => (current === next ? null : next));
  };
  const onCreate = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    try {
      await createPool.mutateAsync({ name: newPoolName, eventId: chosenEventId || undefined });
      setNewPoolName("");
      setMode(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Não foi possível criar o bolão.");
    }
  };
  const onJoin = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    try {
      await joinPool.mutateAsync(joinCode);
      setJoinCode("");
      setMode(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Não foi possível entrar no bolão.");
    }
  };
  const activePools = (pools.data ?? []).filter(({ pool }) => !pool.event.isHistorical);
  const publishedEvents = availableEvents.data ?? [];
  const draftEvents = (events.data ?? []).filter((item) => item.status === "draft");

  return <PageShell className="py-9 sm:py-12">
    <header className="max-w-2xl">
      <p className="text-sm font-semibold text-mint-dark">Olá, {user?.username}</p>
      <h1 className="mt-3 max-w-xl text-4xl leading-[1.08] sm:text-5xl">O que vamos presumir hoje?</h1>
      <p className="mt-4 max-w-lg text-base leading-relaxed text-ink-muted sm:text-lg">Crie um novo bolão ou entre com o código dos seus amigos.</p>
    </header>

    <section className="mt-12" aria-labelledby="active-pools-title">
      <div className="flex items-baseline justify-between gap-3"><h2 id="active-pools-title" className="text-2xl">Em andamento</h2>{activePools.length > 0 && <span className="text-sm text-ink-muted">{activePools.length}</span>}</div>
      {pools.isLoading ? <div className="mt-4"><LoadingState label="Carregando bolões em andamento..." /></div> : pools.isError ? <div className="mt-4"><ErrorState onRetry={() => void pools.refetch()}>{(pools.error as Error).message}</ErrorState></div> : activePools.length === 0 ? <div className="mt-4"><EmptyState title="Nenhum bolão em andamento agora." action={<Button size="lg" onClick={() => openMode("create")}><Plus className="h-4 w-4" /> Criar bolão</Button>}>Crie um bolão ou entre com um código para começar.</EmptyState><div className="mt-3 flex justify-center"><Button variant="link" size="sm" aria-pressed={mode === "join"} onClick={() => openMode("join")} className={cn(mode === "join" && "ring-2 ring-mint-dark/25 ring-offset-2 ring-offset-bg")}><Ticket className="h-4 w-4" /> Já tenho um código</Button></div></div> : <><div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">{activePools.map((summary, index) => <ActivePoolCard key={summary.pool.id} summary={summary} index={index} onOpen={() => navigate(`/pools/${summary.pool.id}`)} />)}</div><div className="mt-6 flex flex-col gap-2 sm:flex-row sm:items-center"><Button variant="outline" size="sm" aria-pressed={mode === "create"} onClick={() => openMode("create")} className={cn("w-full justify-center sm:w-auto", mode === "create" && "ring-2 ring-mint-dark/30 ring-offset-2 ring-offset-bg")}><Plus className="h-4 w-4" /> Criar bolão</Button><Button variant="link" size="sm" aria-pressed={mode === "join"} onClick={() => openMode("join")} className={cn("w-full justify-center sm:w-auto", mode === "join" && "ring-2 ring-mint-dark/25 ring-offset-2 ring-offset-bg")}><Ticket className="h-4 w-4" /> Entrar com código</Button></div></>}
    </section>

    <AnimatePresence initial={false} mode="wait">
      {mode === "create" && <motion.div key="create" initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: "auto" }} exit={{ opacity: 0, height: 0 }} transition={{ duration: 0.22 }} className="mt-5 overflow-hidden"><Card className="border border-mint/30"><PanelHeader title="Criar um bolão" onClose={() => setMode(null)}>Você vira o dono e convida a galera com o código que o app gera.</PanelHeader>{availableEvents.isLoading || events.isLoading ? <p className="mt-4 text-sm text-ink-muted">Carregando eventos disponíveis...</p> : availableEvents.isError ? <div className="mt-4"><ErrorState onRetry={() => void availableEvents.refetch()}>Não foi possível carregar os eventos publicados.</ErrorState></div> : publishedEvents.length > 0 ? <CreatePoolForm events={publishedEvents} chosenEventId={chosenEventId} setChosenEventId={setChosenEventId} newPoolName={newPoolName} setNewPoolName={setNewPoolName} isPending={createPool.isPending} onSubmit={onCreate} onCreateEvent={() => navigate("/events/new")} /> : draftEvents.length > 0 ? <EventRequiredState title="Seus eventos ainda são rascunhos" description="Publique um rascunho para usá-lo em um bolão." primaryLabel="Ver meus rascunhos" onPrimary={() => navigate("/events")} secondaryLabel="Criar novo evento" onSecondary={() => navigate("/events/new")} /> : <EventRequiredState title="Antes, você precisa de um evento" description="O evento define sobre o que os participantes vão palpitar." primaryLabel="Criar meu primeiro evento" onPrimary={() => navigate("/events/new")} />}</Card></motion.div>}
      {mode === "join" && <motion.div key="join" initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: "auto" }} exit={{ opacity: 0, height: 0 }} transition={{ duration: 0.22 }} className="mt-5 overflow-hidden"><Card className="border border-mint-dark/25"><PanelHeader title="Entrar com um código" onClose={() => setMode(null)}>Recebeu um convite? Digite o código de 6 caracteres para entrar no bolão.</PanelHeader><form onSubmit={onJoin} className="mt-4 flex flex-col gap-3"><Input className="text-center font-heading text-2xl font-semibold uppercase tracking-[0.4em]" placeholder="Ex.: 3F9A2C" value={joinCode} onChange={(event) => setJoinCode(event.target.value.toUpperCase().slice(0, 12))} autoCapitalize="characters" autoComplete="off" spellCheck={false} autoFocus required /><Button type="submit" disabled={joinPool.isPending}>{joinPool.isPending ? "Entrando..." : "Entrar no bolão"}</Button></form></Card></motion.div>}
    </AnimatePresence>
    {error && <div className="mt-4"><ErrorBanner>{error}</ErrorBanner></div>}
  </PageShell>;
}

function PanelHeader({ title, children, onClose }: { title: string; children: React.ReactNode; onClose: () => void }) {
  return <div className="flex items-start justify-between gap-3"><div><h2 className="text-xl">{title}</h2><p className="mt-1 text-sm text-ink-muted">{children}</p></div><button type="button" aria-label="Fechar" onClick={onClose} className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-ink-muted transition-colors hover:bg-ink/5 hover:text-ink focus-visible:outline-none focus-visible:shadow-glow"><X className="h-5 w-5" /></button></div>;
}

function CreatePoolForm({ events, chosenEventId, setChosenEventId, newPoolName, setNewPoolName, isPending, onSubmit, onCreateEvent }: { events: MyEvent[]; chosenEventId: string; setChosenEventId: (value: string) => void; newPoolName: string; setNewPoolName: (value: string) => void; isPending: boolean; onSubmit: (event: FormEvent) => void; onCreateEvent: () => void }) {
  return <form onSubmit={onSubmit} className="mt-4 flex flex-col gap-3 sm:flex-row sm:flex-wrap"><Select value={chosenEventId} onChange={(event) => setChosenEventId(event.target.value)} className="sm:w-64" aria-label="Evento do bolão" required><option value="">Escolha um evento publicado</option>{events.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</Select><Input className="flex-1" placeholder="Nome do bolão (ex.: Bolão da firma)" value={newPoolName} onChange={(event) => setNewPoolName(event.target.value)} autoFocus required /><Button type="submit" disabled={isPending || !chosenEventId}>{isPending ? "Criando..." : "Criar bolão"}</Button><Button type="button" variant="link" size="sm" className="w-fit sm:basis-full" onClick={onCreateEvent}>Não encontrou o que queria? Criar um evento →</Button></form>;
}

function EventRequiredState({ title, description, primaryLabel, onPrimary, secondaryLabel, onSecondary }: { title: string; description: string; primaryLabel: string; onPrimary: () => void; secondaryLabel?: string; onSecondary?: () => void }) {
  return <div className="mt-5 rounded-xl border border-mint/20 bg-mint/5 p-5"><h3 className="text-lg">{title}</h3><p className="mt-1 max-w-xl text-sm text-ink-muted">{description}</p><div className="mt-4 flex flex-col gap-2 sm:flex-row"><Button onClick={onPrimary}>{primaryLabel}</Button>{secondaryLabel && onSecondary && <Button variant="outline" onClick={onSecondary}>{secondaryLabel}</Button>}</div></div>;
}

function ActivePoolCard({ summary, index, onOpen }: { summary: DashboardPool; index: number; onOpen: () => void }) {
  const { pool, answeredCount, itemCount } = summary;
  return <MotionCard transition={{ delay: index * 0.05, duration: 0.25 }} className="flex flex-col"><div className="flex items-start justify-between gap-3"><div><h3 className="text-lg">{pool.name}</h3><p className="mt-1 text-sm font-semibold text-mint-dark">{pool.event.name}</p></div><span className="shrink-0 rounded-pill bg-mint/20 px-2.5 py-1 text-xs font-semibold">{presentationStatusLabel[poolPresentationStatus(pool.event)]}</span></div><ProgressBar value={answeredCount} total={itemCount} /><p className="mt-3 text-sm text-ink-muted">{pool.memberCount} participante(s)</p><Button variant="outline" size="sm" className="mt-5 w-fit" onClick={onOpen}>Abrir bolão</Button></MotionCard>;
}
