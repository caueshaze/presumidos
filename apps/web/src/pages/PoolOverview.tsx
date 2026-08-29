import { ArrowRight, BookOpenText, ClipboardCheck, Flag, LogOut, MoreHorizontal, Share2, Trash2, Trophy, Users } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { PageShell } from "@/components/PageShell";
import { PoolShareModal } from "@/components/PoolShareModal";
import { PredictionReuseModal } from "@/components/PredictionReuseModal";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";
import { useAuth } from "@/hooks/useAuth";
import { useCopyPredictionsReuse, useCreatePoolReport, useDashboardPools, useDeletePool, useEventShowcase, useLeavePool, useLeaderboard, usePools, usePredictionReuseSuggestion, useStartPredictionsEmpty } from "@/hooks/queries";
import type { PoolReportCategory, PredictionReuseSuggestion } from "@/types";

import { PoolActionModal, type PoolAction } from "./pool-overview/PoolActionModal";

export function PoolOverviewPage() {
  const { poolId = "" } = useParams();
  const navigate = useNavigate();
  const { user, isAdmin } = useAuth();
  const pools = usePools();
  const dashboard = useDashboardPools();
  const leaderboard = useLeaderboard(poolId || null);
  const [copied, setCopied] = useState<"link" | "code" | null>(null);
  const [shareModalOpen, setShareModalOpen] = useState(false);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [action, setAction] = useState<PoolAction | null>(null);
  const [reuseModalOpen, setReuseModalOpen] = useState(false);
  const [reuseError, setReuseError] = useState("");
  const [reuseOffer, setReuseOffer] = useState<PredictionReuseSuggestion | null>(null);
  const [reportCategory, setReportCategory] = useState<PoolReportCategory>("inappropriate_content");
  const [reportDetails, setReportDetails] = useState("");
  const optionsRef = useRef<HTMLDivElement>(null);
  const leavePool = useLeavePool();
  const deletePool = useDeletePool();
  const createReport = useCreatePoolReport();
  const reuseSuggestion = usePredictionReuseSuggestion(poolId || null);
  const copyPredictions = useCopyPredictionsReuse();
  const startEmpty = useStartPredictionsEmpty();
  const pool = pools.data?.find((item) => item.id === poolId);
  const summary = dashboard.data?.find((item) => item.pool.id === poolId);
  const showcase = useEventShowcase(poolId || null);
  useEffect(() => {
    if (!shareModalOpen && !optionsOpen && !action && !reuseModalOpen) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (action) setAction(null);
      else if (reuseModalOpen) setReuseModalOpen(false);
      else if (shareModalOpen) setShareModalOpen(false);
      else setOptionsOpen(false);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [action, optionsOpen, reuseModalOpen, shareModalOpen]);
  useEffect(() => {
    if (!optionsOpen) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!optionsRef.current?.contains(event.target as Node)) setOptionsOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [optionsOpen]);

  if (pools.isLoading || dashboard.isLoading) {
    return <PageShell><Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button><Card className="mt-4"><p className="text-ink-muted">Carregando bolão...</p></Card></PageShell>;
  }
  if (!pool) {
    return <PageShell><Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button><div className="mt-4"><ErrorBanner>Bolão não encontrado ou sem acesso.</ErrorBanner></div></PageShell>;
  }
  const historical = pool.event.isHistorical;
  const event = showcase.data;
  const myPosition = leaderboard.data?.findIndex((entry) => entry.userId === user?.id);
  const winner = leaderboard.data?.[0];
  const inviteUrl = `${window.location.origin}/pools/join/${pool.inviteCode}`;
  const canShare = typeof navigator !== "undefined" && "share" in navigator;
  const owner = user?.id === pool.createdBy;
  const canManageEventResults = owner || isAdmin;
  const actionError = action === "report" ? (createReport.error instanceof Error ? createReport.error.message : "") : action === "leave" ? (leavePool.error instanceof Error ? leavePool.error.message : "") : action === "delete" ? (deletePool.error instanceof Error ? deletePool.error.message : "") : "";

  const copyShareValue = async (value: string, target: "link" | "code") => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(target);
      window.setTimeout(() => setCopied(null), 2200);
    } catch {
      setCopied(null);
    }
  };

  const shareInvite = async () => {
    if (navigator.share) {
      try {
        await navigator.share({
          title: `${pool.name} — Presumidos`,
          text: `Entre no meu bolão "${pool.name}" no Presumidos.`,
          url: inviteUrl,
        });
      } catch (error) {
        if (error instanceof Error && error.name === "AbortError") return;
        await copyShareValue(inviteUrl, "link");
      }
      return;
    }
    await copyShareValue(inviteUrl, "link");
  };

  const openAction = (nextAction: PoolAction) => {
    setOptionsOpen(false);
    setAction(nextAction);
    if (nextAction === "report") {
      setReportCategory("inappropriate_content");
      setReportDetails("");
      createReport.reset();
    }
  };

  const handleLeave = async () => {
    try {
      await leavePool.mutateAsync(pool.id);
      navigate("/pools", { replace: true });
    } catch {
      // O erro da mutation aparece no modal.
    }
  };

  const handleDelete = async () => {
    try {
      await deletePool.mutateAsync(pool.id);
      navigate("/pools", { replace: true });
    } catch {
      // O erro da mutation aparece no modal.
    }
  };

  const handleReport = async () => {
    try {
      await createReport.mutateAsync({ poolId: pool.id, category: reportCategory, details: reportDetails });
      setAction("reportSubmitted");
    } catch {
      // O erro da mutation aparece no modal.
    }
  };

  const startPredictions = async () => {
    setReuseError("");
    try {
      const result = await reuseSuggestion.refetch();
      if (result.data?.available) {
        setReuseOffer(result.data);
        setReuseModalOpen(true);
      }
      else navigate(`/pools/${pool.id}/predictions`);
    } catch {
      navigate(`/pools/${pool.id}/predictions`);
    }
  };

  const reusePredictions = async () => {
    setReuseError("");
    try {
      await copyPredictions.mutateAsync(pool.id);
      setReuseModalOpen(false);
      navigate(`/pools/${pool.id}/predictions`);
    } catch (error) {
      setReuseError(error instanceof Error ? error.message : "Não foi possível copiar os palpites.");
    }
  };

  const beginEmpty = async () => {
    setReuseError("");
    try {
      await startEmpty.mutateAsync(pool.id);
      setReuseModalOpen(false);
      navigate(`/pools/${pool.id}/predictions`);
    } catch (error) {
      setReuseError(error instanceof Error ? error.message : "Não foi possível iniciar os palpites.");
    }
  };

  return <PageShell>
    <Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button>
    {(event?.coverAssetUrl ?? event?.coverUrl) && <div className="mt-4 overflow-hidden rounded-2xl border border-mint/20 bg-card"><img src={event.coverAssetUrl ?? event.coverUrl ?? undefined} alt="" loading="lazy" className="aspect-[3/1] w-full object-cover" onError={(item) => {
      if (event.coverAssetUrl && event.coverUrl && item.currentTarget.dataset.fallback !== "used") {
        item.currentTarget.dataset.fallback = "used";
        item.currentTarget.src = event.coverUrl;
      } else {
        item.currentTarget.parentElement?.remove();
      }
    }} /></div>}
    <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div><p className="text-sm font-semibold text-mint-dark">{event?.name ?? pool.event.name}</p><h1 className="text-3xl">{pool.name}</h1>{historical && <p className="mt-2 text-ink-muted">Edição encerrada — resultados preservados para consulta.</p>}{event?.description && <p className="mt-3 max-w-2xl text-sm text-ink-muted">{event.description}</p>}{event?.externalUrl && <a href={event.externalUrl} target="_blank" rel="noopener noreferrer" className="mt-3 inline-block text-sm font-semibold text-mint-dark underline-offset-2 hover:underline">Site oficial ↗</a>}</div>
      {historical && <span className="w-fit rounded-pill bg-mint/25 px-3 py-1 text-sm font-semibold">Encerrado</span>}
    </div>
    <div className="mt-6 grid grid-cols-2 gap-3 sm:grid-cols-3 sm:gap-4">
      <Card className="p-4 sm:p-6"><p className="text-xs text-ink-muted sm:text-sm">Participantes</p><p className="mt-1 text-xl font-semibold sm:text-2xl"><Users className="mr-1 inline h-5 w-5 text-mint-dark" />{pool.memberCount}</p></Card>
      {!historical && <Card className="p-4 sm:p-6"><p className="text-xs text-ink-muted sm:text-sm">Seus palpites</p><p className="mt-1 text-xl font-semibold sm:text-2xl">{event?.answeredCount ?? summary?.answeredCount ?? 0} de {event?.itemCount ?? summary?.itemCount ?? 0}</p></Card>}
      <Card className="col-span-2 p-4 sm:col-span-1 sm:p-6"><p className="text-xs text-ink-muted sm:text-sm">{historical ? "Campeão" : "Liderança"}</p><p className="mt-1 break-words text-base font-semibold sm:text-lg"><Trophy className="mr-1 inline h-5 w-5 text-yellow-dark" />{winner ? `${winner.username} · ${winner.points} pts` : "Ainda sem ranking"}</p></Card>
    </div>
    {historical && myPosition != null && <Card className="mt-4"><p className="text-sm text-ink-muted">Sua colocação final</p><p className="mt-1 text-xl font-semibold">{myPosition + 1}º de {leaderboard.data?.length ?? 0}</p></Card>}
    <Card className="mt-6">
      <h2 className="text-xl">{historical ? "Consultar edição" : "Ações do bolão"}</h2>
      <div className="mt-4 grid grid-cols-2 gap-3 sm:flex sm:flex-wrap">
        <Button className="col-span-2 h-[52px] w-full justify-center sm:w-full" disabled={reuseSuggestion.isFetching} onClick={() => historical ? navigate(`/pools/${pool.id}/leaderboard`) : void startPredictions()}>{historical ? "Ver resultados" : reuseSuggestion.isFetching ? "Verificando palpites…" : "Palpitar"}<ArrowRight className="h-4 w-4" /></Button>
        <Button variant="outline" className="h-[52px] w-full justify-start rounded-[14px] border border-mint/15 bg-card/55 px-4 text-left text-ink hover:border-mint/30 hover:bg-card hover:text-ink sm:w-auto" onClick={() => navigate(`/pools/${pool.id}/leaderboard`)}><Trophy className="h-4 w-4 shrink-0 text-yellow-dark" />{historical ? "Ranking final" : "Ranking"}</Button>
        <Button variant="outline" className="h-[52px] w-full justify-start rounded-[14px] border border-mint/15 bg-card/55 px-4 text-left text-ink hover:border-mint/30 hover:bg-card hover:text-ink sm:w-auto" onClick={() => navigate(`/pools/${pool.id}/members`)}><Users className="h-4 w-4 shrink-0 text-mint-dark" />Participantes</Button>
        <Button variant="outline" className="h-[52px] w-full justify-start rounded-[14px] border border-mint/15 bg-card/55 px-4 text-left text-ink hover:border-mint/30 hover:bg-card hover:text-ink sm:w-auto" onClick={() => navigate(`/pools/${pool.id}/scoring`)}><BookOpenText className="h-4 w-4 shrink-0 text-mint-dark" />Regras</Button>
        {pool.event.kind === "custom" && canManageEventResults && <Button variant="outline" className="h-[52px] w-full justify-start rounded-[14px] border border-mint/15 bg-card/55 px-4 text-left text-ink hover:border-mint/30 hover:bg-card hover:text-ink sm:w-auto" onClick={() => navigate(`/pools/${pool.id}/scoring?section=results`)}><ClipboardCheck className="h-4 w-4 shrink-0 text-mint-dark" />Resultados oficiais</Button>}
        {!historical && <Button variant="outline" className="h-[52px] w-full justify-start rounded-[14px] border border-mint/15 bg-card/55 px-4 text-left text-ink hover:border-mint/30 hover:bg-card hover:text-ink sm:w-auto" onClick={() => setShareModalOpen(true)}><Share2 className="h-4 w-4 shrink-0 text-mint-dark" />Compartilhar</Button>}
        <div ref={optionsRef} className="relative col-span-2 w-full sm:col-span-1 sm:w-auto">
          <Button variant="outline" className="h-[52px] w-full justify-start rounded-[14px] border border-mint/15 bg-card/55 px-4 text-left text-ink hover:border-mint/30 hover:bg-card hover:text-ink sm:w-auto" aria-haspopup="menu" aria-expanded={optionsOpen} onClick={() => setOptionsOpen((open) => !open)}><MoreHorizontal className="h-4 w-4 shrink-0 text-mint-dark" />Opções</Button>
          {optionsOpen && <div role="menu" aria-label="Opções do bolão" className="absolute left-0 top-full z-30 mt-2 w-56 max-w-[calc(100vw-2rem)] rounded-2xl border border-mint/20 bg-card p-2 shadow-card sm:left-auto sm:right-0 sm:max-w-none">
            <button type="button" role="menuitem" className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm font-semibold text-ink transition-colors hover:bg-mint/10" onClick={() => openAction("report")}><Flag className="h-4 w-4 text-yellow-dark" />Denunciar bolão</button>
            {owner ? <button type="button" role="menuitem" className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm font-semibold text-danger transition-colors hover:bg-danger-bg" onClick={() => openAction("delete")}><Trash2 className="h-4 w-4" />Excluir bolão</button> : <button type="button" role="menuitem" className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm font-semibold text-ink transition-colors hover:bg-mint/10" onClick={() => openAction("leave")}><LogOut className="h-4 w-4 text-mint-dark" />Sair do bolão</button>}
          </div>}
        </div>
      </div>
    </Card>
    {shareModalOpen && !historical && <PoolShareModal inviteUrl={inviteUrl} inviteCode={pool.inviteCode} poolName={pool.name} copied={copied} canShare={canShare} onCopy={(value, target) => void copyShareValue(value, target)} onShare={() => void shareInvite()} onClose={() => setShareModalOpen(false)} />}
    {reuseModalOpen && reuseOffer?.available && <PredictionReuseModal suggestion={reuseOffer} pending={copyPredictions.isPending || startEmpty.isPending} error={reuseError} onCopy={() => void reusePredictions()} onStartEmpty={() => void beginEmpty()} onClose={() => setReuseModalOpen(false)} />}
    {action && <PoolActionModal action={action} poolName={pool.name} reportCategory={reportCategory} reportDetails={reportDetails} reportPending={createReport.isPending} actionPending={leavePool.isPending || deletePool.isPending} error={actionError} onCategoryChange={setReportCategory} onDetailsChange={setReportDetails} onReport={() => void handleReport()} onLeave={() => void handleLeave()} onDelete={() => void handleDelete()} onClose={() => setAction(null)} />}
  </PageShell>;
}
