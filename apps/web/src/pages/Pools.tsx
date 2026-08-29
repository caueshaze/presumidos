import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { AnimatePresence, motion } from "framer-motion";
import { Plus, Ticket, X } from "lucide-react";
import { useAvailableEvents, useCreatePool, useDashboardPools, useJoinPool, useLeaderboard, useMyEvents, type MyEvent } from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Select } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { EmptyState, ErrorState, LoadingState, ProgressBar } from "@/components/ui/states";

export function PoolsPage() {
  const navigate = useNavigate();
  const [search, setSearch] = useSearchParams();
  const pools = useDashboardPools();
  const events = useMyEvents();
  const availableEvents = useAvailableEvents();
  const createPool = useCreatePool();
  const joinPool = useJoinPool();
  const [mode, setMode] = useState<"create" | "join" | null>(null);
  const [newPoolName, setNewPoolName] = useState("");
  const [chosenEventId, setChosenEventId] = useState("");
  const [joinCode, setJoinCode] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    const requestedMode = search.get("mode");
    setMode(requestedMode === "create" || requestedMode === "join" ? requestedMode : null);
  }, [search]);
  const openMode = (next: "create" | "join") => {
    setError("");
    if (next === "create") { void availableEvents.refetch(); void events.refetch(); }
    setSearch({ mode: mode === next ? "" : next });
  };
  const closeMode = () => { setError(""); setSearch({}); };
  const onCreate = async (event: FormEvent) => {
    event.preventDefault(); setError("");
    try { await createPool.mutateAsync({ name: newPoolName, eventId: chosenEventId || undefined }); setNewPoolName(""); closeMode(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "Não foi possível criar o bolão."); }
  };
  const onJoin = async (event: FormEvent) => {
    event.preventDefault(); setError("");
    try { await joinPool.mutateAsync(joinCode); setJoinCode(""); closeMode(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "Não foi possível entrar no bolão."); }
  };
  const active = (pools.data ?? []).filter(({ pool }) => !pool.event.isHistorical);
  const historical = (pools.data ?? []).filter(({ pool }) => pool.event.isHistorical);
  return <PageShell>
    <header><div><h1 className="text-3xl">Meus bolões</h1><p className="mt-1 text-ink-muted">Acompanhe seus palpites, resultados e a disputa com a galera.</p></div><div className="mt-5 flex w-full flex-col gap-2 sm:w-auto sm:flex-row sm:flex-wrap sm:items-center"><Button className="w-full justify-center sm:w-auto" onClick={() => openMode("create")}><Plus className="h-4 w-4" /> Criar bolão</Button><Button className="w-full justify-center sm:w-auto" variant="outline" onClick={() => openMode("join")}><Ticket className="h-4 w-4" /> Entrar com código</Button><Button className="w-full justify-center sm:w-auto" variant="link" size="sm" onClick={() => navigate("/events/new")}>Criar evento</Button></div></header>
    <AnimatePresence initial={false} mode="wait">
      {mode === "create" && <motion.div key="create" initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: "auto" }} exit={{ opacity: 0, height: 0 }} transition={{ duration: 0.22 }} className="mt-5 overflow-hidden"><Card className="border border-mint/30"><PoolPanelHeader title="Criar um bolão" onClose={closeMode}>Você vira o dono e convida a galera com o código que o app gera.</PoolPanelHeader>{availableEvents.isLoading || events.isLoading ? <p className="mt-4 text-sm text-ink-muted">Carregando eventos disponíveis...</p> : availableEvents.isError ? <div className="mt-4"><ErrorState onRetry={() => void availableEvents.refetch()}>Não foi possível carregar os eventos publicados.</ErrorState></div> : (availableEvents.data ?? []).length > 0 ? <CreatePoolForm events={availableEvents.data ?? []} chosenEventId={chosenEventId} setChosenEventId={setChosenEventId} newPoolName={newPoolName} setNewPoolName={setNewPoolName} isPending={createPool.isPending} onSubmit={onCreate} onCreateEvent={() => navigate("/events/new")} /> : <p className="mt-4 text-sm text-ink-muted">Publique um evento antes de criar o bolão.</p>}</Card></motion.div>}
      {mode === "join" && <motion.div key="join" initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: "auto" }} exit={{ opacity: 0, height: 0 }} transition={{ duration: 0.22 }} className="mt-5 overflow-hidden"><Card className="border border-mint-dark/25"><PoolPanelHeader title="Entrar com um código" onClose={closeMode}>Digite o código de 6 caracteres que você recebeu.</PoolPanelHeader><form onSubmit={onJoin} className="mt-4 flex flex-col gap-3 sm:flex-row"><Input className="text-center font-heading text-2xl font-semibold uppercase tracking-[0.4em] sm:flex-1" placeholder="Ex.: 3F9A2C" value={joinCode} onChange={(event) => setJoinCode(event.target.value.toUpperCase().slice(0, 12))} autoCapitalize="characters" autoComplete="off" spellCheck={false} autoFocus required /><Button type="submit" disabled={joinPool.isPending}>{joinPool.isPending ? "Entrando..." : "Entrar no bolão"}</Button></form></Card></motion.div>}
    </AnimatePresence>
    {error && <div className="mt-4"><ErrorBanner>{error}</ErrorBanner></div>}
    {pools.isLoading ? <div className="mt-6"><LoadingState label="Carregando seus bolões..." /></div> : pools.isError ? <div className="mt-6"><ErrorState onRetry={() => void pools.refetch()}>{(pools.error as Error).message}</ErrorState></div> : pools.data?.length === 0 ? <div className="mt-6"><EmptyState title="Você ainda não participa de nenhum bolão." action={<Button onClick={() => openMode("create")}>Criar bolão</Button>}>Crie um novo ou entre pelo convite de um amigo.</EmptyState></div> : <div className="mt-8 space-y-9"><PoolList title="Em andamento" pools={active} /><HistoricalPools pools={historical} /></div>}
  </PageShell>;
}

function PoolPanelHeader({ title, children, onClose }: { title: string; children: React.ReactNode; onClose: () => void }) {
  return <div className="flex items-start justify-between gap-3"><div><h2 className="text-xl">{title}</h2><p className="mt-1 text-sm text-ink-muted">{children}</p></div><button type="button" aria-label="Fechar" onClick={onClose} className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-ink-muted transition-colors hover:bg-ink/5 hover:text-ink"><X className="h-5 w-5" /></button></div>;
}

function CreatePoolForm({ events, chosenEventId, setChosenEventId, newPoolName, setNewPoolName, isPending, onSubmit, onCreateEvent }: { events: MyEvent[]; chosenEventId: string; setChosenEventId: (value: string) => void; newPoolName: string; setNewPoolName: (value: string) => void; isPending: boolean; onSubmit: (event: FormEvent) => void; onCreateEvent: () => void }) {
  return <form onSubmit={onSubmit} className="mt-4 flex flex-col gap-3 sm:flex-row sm:flex-wrap"><Select value={chosenEventId} onChange={(event) => setChosenEventId(event.target.value)} className="sm:w-64" aria-label="Evento do bolão" required><option value="">Escolha um evento publicado</option>{events.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</Select><Input className="flex-1" placeholder="Nome do bolão (ex.: Bolão da firma)" value={newPoolName} onChange={(event) => setNewPoolName(event.target.value)} autoFocus required /><Button type="submit" disabled={isPending || !chosenEventId}>{isPending ? "Criando..." : "Criar bolão"}</Button><Button type="button" variant="link" size="sm" className="w-fit sm:basis-full" onClick={onCreateEvent}>Não encontrou o que queria? Criar um evento →</Button></form>;
}

function HistoricalPools({ pools }: { pools: Summary[] }) {
  const navigate = useNavigate();
  const { user } = useAuth();
  const [showAll, setShowAll] = useState(false);
  const latest = pools[0];
  const leaderboard = useLeaderboard(latest?.pool.id ?? null);
  const myPosition = leaderboard.data?.findIndex((entry) => entry.userId === user?.id);
  const myEntry = myPosition == null || myPosition < 0 ? undefined : leaderboard.data?.[myPosition];
  const displayed = showAll ? pools : pools.slice(0, 1);

  return <section>
    <div className="flex items-baseline justify-between gap-3"><h2 className="text-2xl">Bolões anteriores</h2><span className="text-sm text-ink-muted">{pools.length === 1 ? "1 edição" : `${pools.length} edições`}</span></div>
    {pools.length === 0 ? <Card className="mt-4"><p className="text-ink-muted">Nenhum bolão terminou ainda. Quando uma edição acabar, ela ficará guardada aqui.</p></Card> : <><div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">{displayed.map((summary, index) => <HistoricalPoolCard key={summary.pool.id} summary={summary} position={index === 0 ? myPosition : undefined} points={index === 0 ? myEntry?.points : undefined} onOpen={() => navigate(`/pools/${summary.pool.id}`)} />)}</div>{pools.length > 1 && <Button variant="link" size="sm" className="mt-3" aria-expanded={showAll} onClick={() => setShowAll((shown) => !shown)}>{showAll ? "Ocultar histórico" : "Ver todo o histórico →"}</Button>}</>}
  </section>;
}

function HistoricalPoolCard({ summary, position, points, onOpen }: { summary: Summary; position?: number; points?: number; onOpen: () => void }) {
  const { pool } = summary;
  return <Card className="flex flex-col border-mint-dark/15 bg-card/85"><div className="flex items-start justify-between gap-3"><div><h3 className="text-lg">{pool.name}</h3><p className="mt-1 text-sm font-semibold text-mint-dark">{pool.event.name}</p></div><span className="rounded-pill bg-mint/15 px-2.5 py-1 text-xs font-semibold">Encerrado</span></div>{position != null && <div className="mt-4 grid grid-cols-2 gap-3 border-t border-mint/15 pt-3 text-sm"><div><p className="text-ink-muted">Sua colocação</p><p className="mt-0.5 font-semibold">{position + 1}º</p></div><div><p className="text-ink-muted">Seus pontos</p><p className="mt-0.5 font-semibold">{points ?? "—"}{points != null ? " pts" : ""}</p></div></div>}<Button size="sm" className="mt-5 w-fit" onClick={onOpen}>Ver resultados →</Button></Card>;
}

type Summary = NonNullable<ReturnType<typeof useDashboardPools>["data"]>[number];
function PoolList({ title, pools }: { title: string; pools: Summary[] }) {
  const navigate = useNavigate();
  return <section>{title && <h2 className="text-2xl">{title}</h2>}{pools.length === 0 ? <Card className="mt-3"><p className="text-ink-muted">{title === "Em andamento" ? "Nenhum bolão em andamento agora." : "Quando uma edição terminar, ela aparecerá aqui."}</p></Card> : <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">{pools.map(({ pool, answeredCount, itemCount }) => { const historical = pool.event.isHistorical; return <Card key={pool.id} className="flex flex-col"><div><h3 className="text-lg">{pool.name}</h3><p className="mt-1 text-sm font-semibold text-mint-dark">{pool.event.name}</p></div>{!historical && <ProgressBar value={answeredCount} total={itemCount} />}<p className="mt-3 text-sm text-ink-muted">{pool.memberCount} participante(s)</p><div className="mt-5 flex flex-wrap gap-2"><Button size="sm" onClick={() => navigate(`/pools/${pool.id}`)}>{historical ? "Ver resultados" : "Entrar"}</Button><Button size="sm" variant="outline" onClick={() => navigate(`/pools/${pool.id}/leaderboard`)}>{historical ? "Ranking final" : "Ranking"}</Button></div></Card>; })}</div>}</section>;
}
