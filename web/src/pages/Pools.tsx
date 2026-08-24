import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, Ticket } from "lucide-react";
import { useDashboardPools, useLeaderboard } from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { EmptyState, ErrorState, LoadingState, ProgressBar } from "@/components/ui/states";
import { poolPresentationStatus, presentationStatusLabel } from "@/lib/lifecycle";

export function PoolsPage() {
  const navigate = useNavigate();
  const pools = useDashboardPools();
  const active = (pools.data ?? []).filter(({ pool }) => !pool.event.isHistorical);
  const historical = (pools.data ?? []).filter(({ pool }) => pool.event.isHistorical);
  return <PageShell>
    <header><div><h1 className="text-3xl">Meus bolões</h1><p className="mt-1 text-ink-muted">Acompanhe seus palpites, resultados e a disputa com a galera.</p></div><div className="mt-5 flex w-full flex-col gap-2 sm:w-auto sm:flex-row sm:flex-wrap sm:items-center"><Button className="w-full justify-center sm:w-auto" onClick={() => navigate("/dashboard?mode=create")}><Plus className="h-4 w-4" /> Criar bolão</Button><Button className="w-full justify-center sm:w-auto" variant="outline" onClick={() => navigate("/dashboard?mode=join")}><Ticket className="h-4 w-4" /> Entrar com código</Button><Button className="w-full justify-center sm:w-auto" variant="link" size="sm" onClick={() => navigate("/events/new")}>Criar evento</Button></div></header>
    {pools.isLoading ? <div className="mt-6"><LoadingState label="Carregando seus bolões..." /></div> : pools.isError ? <div className="mt-6"><ErrorState onRetry={() => void pools.refetch()}>{(pools.error as Error).message}</ErrorState></div> : pools.data?.length === 0 ? <div className="mt-6"><EmptyState title="Você ainda não participa de nenhum bolão." action={<Button onClick={() => navigate("/dashboard?mode=create")}>Criar bolão</Button>}>Crie um novo ou entre pelo convite de um amigo.</EmptyState></div> : <div className="mt-8 space-y-9"><PoolList title="Em andamento" pools={active} /><HistoricalPools pools={historical} /></div>}
  </PageShell>;
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
  return <section>{title && <h2 className="text-2xl">{title}</h2>}{pools.length === 0 ? <Card className="mt-3"><p className="text-ink-muted">{title === "Em andamento" ? "Nenhum bolão em andamento agora." : "Quando uma edição terminar, ela aparecerá aqui."}</p></Card> : <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">{pools.map(({ pool, answeredCount, itemCount }) => { const historical = pool.event.isHistorical; return <Card key={pool.id} className="flex flex-col"><div className="flex items-start justify-between gap-3"><div><h3 className="text-lg">{pool.name}</h3><p className="mt-1 text-sm font-semibold text-mint-dark">{pool.event.name}</p></div><span className="rounded-pill bg-mint/20 px-2.5 py-1 text-xs font-semibold">{presentationStatusLabel[poolPresentationStatus(pool.event)]}</span></div>{!historical && <ProgressBar value={answeredCount} total={itemCount} />}<p className="mt-3 text-sm text-ink-muted">{pool.memberCount} participante(s)</p><div className="mt-5 flex flex-wrap gap-2"><Button size="sm" onClick={() => navigate(`/pools/${pool.id}`)}>{historical ? "Ver resultados" : "Continuar"}</Button><Button size="sm" variant="outline" onClick={() => navigate(`/pools/${pool.id}/leaderboard`)}>{historical ? "Ranking final" : "Ranking"}</Button></div></Card>; })}</div>}</section>;
}
