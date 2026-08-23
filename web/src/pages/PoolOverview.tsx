import { Check, Copy, Trophy, Users } from "lucide-react";
import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";
import { useAuth } from "@/hooks/useAuth";
import { useDashboardPools, useLeaderboard, usePools } from "@/hooks/queries";

export function PoolOverviewPage() {
  const { poolId = "" } = useParams();
  const navigate = useNavigate();
  const { user } = useAuth();
  const pools = usePools();
  const dashboard = useDashboardPools();
  const leaderboard = useLeaderboard(poolId || null);
  const [copied, setCopied] = useState(false);
  const pool = pools.data?.find((item) => item.id === poolId);
  const summary = dashboard.data?.find((item) => item.pool.id === poolId);

  if (pools.isLoading || dashboard.isLoading) {
    return <PageShell><Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button><Card className="mt-4"><p className="text-ink-muted">Carregando bolão...</p></Card></PageShell>;
  }
  if (!pool) {
    return <PageShell><Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button><div className="mt-4"><ErrorBanner>Bolão não encontrado ou sem acesso.</ErrorBanner></div></PageShell>;
  }
  const historical = pool.event.isHistorical;
  const myPosition = leaderboard.data?.findIndex((entry) => entry.userId === user?.id);
  const winner = leaderboard.data?.[0];
  const copyInvite = async () => {
    try {
      await navigator.clipboard.writeText(`${window.location.origin}/dashboard?invite=${pool.inviteCode}`);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2200);
    } catch {
      setCopied(false);
    }
  };

  return <PageShell>
    <Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button>
    <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div><p className="text-sm font-semibold text-mint-dark">{pool.event.name}</p><h1 className="text-3xl">{pool.name}</h1><p className="mt-2 text-ink-muted">{historical ? "Edição encerrada — resultados preservados para consulta." : "Em andamento"}</p></div>
      <span className="w-fit rounded-pill bg-mint/25 px-3 py-1 text-sm font-semibold">{historical ? "Encerrado" : "Em andamento"}</span>
    </div>
    <div className="mt-6 grid gap-4 sm:grid-cols-3">
      <Card><p className="text-sm text-ink-muted">Participantes</p><p className="mt-1 text-2xl font-semibold"><Users className="mr-1 inline h-5 w-5 text-mint-dark" />{pool.memberCount}</p></Card>
      {!historical && <Card><p className="text-sm text-ink-muted">Seu progresso</p><p className="mt-1 text-2xl font-semibold">{summary?.answeredCount ?? 0} de {summary?.itemCount ?? 0}</p></Card>}
      <Card><p className="text-sm text-ink-muted">{historical ? "Campeão" : "Liderança"}</p><p className="mt-1 text-lg font-semibold"><Trophy className="mr-1 inline h-5 w-5 text-yellow-dark" />{winner ? `${winner.username} · ${winner.points} pts` : "Ainda sem ranking"}</p></Card>
    </div>
    {historical && myPosition != null && <Card className="mt-4"><p className="text-sm text-ink-muted">Sua colocação final</p><p className="mt-1 text-xl font-semibold">{myPosition + 1}º de {leaderboard.data?.length ?? 0}</p></Card>}
    <Card className="mt-6"><h2 className="text-xl">{historical ? "Consultar edição" : "Ações do bolão"}</h2><div className="mt-4 flex flex-wrap gap-2"><Button onClick={() => navigate(`/pools/${pool.id}/${historical ? "leaderboard" : "predictions"}`)}>{historical ? "Ver resultados" : "Continuar palpites"}</Button><Button variant="secondary" onClick={() => navigate(`/pools/${pool.id}/leaderboard`)}>{historical ? "Ranking final" : "Ranking"}</Button><Button variant="outline" onClick={() => navigate(`/pools/${pool.id}/members`)}>Participantes</Button><Button variant="outline" onClick={() => navigate(`/pools/${pool.id}/scoring`)}>Regras</Button>{!historical && <Button variant="outline" onClick={copyInvite}>{copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}{copied ? "Convite copiado" : "Copiar convite"}</Button>}</div></Card>
  </PageShell>;
}
