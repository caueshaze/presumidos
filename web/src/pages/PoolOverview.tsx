import { ArrowRight, BookOpenText, CheckCircle2, Flag, LogOut, MoreHorizontal, Share2, Trash2, Trophy, Users, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { PageShell } from "@/components/PageShell";
import { PoolShareModal } from "@/components/PoolShareModal";
import { PredictionReuseModal } from "@/components/PredictionReuseModal";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { useAuth } from "@/hooks/useAuth";
import { useCopyPredictionsReuse, useCreatePoolReport, useDashboardPools, useDeletePool, useEventShowcase, useLeavePool, useLeaderboard, usePools, usePredictionReuseSuggestion, useStartPredictionsEmpty } from "@/hooks/queries";
import type { PoolReportCategory, PredictionReuseSuggestion } from "@/types";

type PoolAction = "report" | "reportSubmitted" | "leave" | "delete";

const reportCategoryOptions: Array<{ value: PoolReportCategory; label: string }> = [
  { value: "inappropriate_content", label: "Conteúdo inadequado" },
  { value: "spam_or_fraud", label: "Spam ou fraude" },
  { value: "harassment", label: "Assédio" },
  { value: "other", label: "Outro" },
];

function PoolActionModal({
  action,
  poolName,
  reportCategory,
  reportDetails,
  reportPending,
  actionPending,
  error,
  onCategoryChange,
  onDetailsChange,
  onReport,
  onLeave,
  onDelete,
  onClose,
}: {
  action: PoolAction;
  poolName: string;
  reportCategory: PoolReportCategory;
  reportDetails: string;
  reportPending: boolean;
  actionPending: boolean;
  error: string;
  onCategoryChange: (value: PoolReportCategory) => void;
  onDetailsChange: (value: string) => void;
  onReport: () => void;
  onLeave: () => void;
  onDelete: () => void;
  onClose: () => void;
}) {
  const submitted = action === "reportSubmitted";
  const report = action === "report" || submitted;
  const destructive = action === "leave" || action === "delete";
  const title = action === "delete" ? "Excluir bolão" : action === "leave" ? "Sair do bolão" : "Denunciar bolão";
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/45 p-4 backdrop-blur-sm" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !actionPending && !reportPending) onClose(); }}>
      <div className="w-full max-w-lg rounded-[28px] border border-mint/20 bg-card p-6 shadow-2xl shadow-black/25 sm:p-7" role="dialog" aria-modal="true" aria-labelledby="pool-action-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="flex items-start gap-4">
          <div className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl ${destructive ? "bg-danger/15 text-danger" : "bg-mint/20 text-mint-dark"}`}>
            {action === "delete" ? <Trash2 className="h-6 w-6" /> : action === "leave" ? <LogOut className="h-6 w-6" /> : <Flag className="h-6 w-6" />}
          </div>
          <div className="min-w-0 flex-1">
            <h2 id="pool-action-title" className="text-2xl">{title}</h2>
            <p className="mt-1 text-sm text-ink-muted">{poolName}</p>
          </div>
          <Button variant="link" size="sm" className="h-10 w-10 shrink-0 rounded-full p-0 text-ink-muted hover:bg-mint/10 hover:no-underline" aria-label="Fechar" onClick={onClose} disabled={actionPending || reportPending}><X className="h-5 w-5" /></Button>
        </div>

        {submitted ? (
          <div className="mt-6 rounded-2xl border border-success/35 bg-mint/15 p-4 text-sm font-semibold text-mint-dark"><CheckCircle2 className="mr-2 inline h-5 w-5" />Denúncia enviada para análise.</div>
        ) : report ? (
          <form className="mt-6 space-y-4" onSubmit={(event) => { event.preventDefault(); onReport(); }}>
            <div>
              <Label htmlFor="pool-report-category">Motivo</Label>
              <Select id="pool-report-category" value={reportCategory} onChange={(event) => onCategoryChange(event.target.value as PoolReportCategory)}>
                {reportCategoryOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </Select>
            </div>
            <div>
              <Label htmlFor="pool-report-details">Detalhes (opcional)</Label>
              <textarea id="pool-report-details" value={reportDetails} maxLength={1000} onChange={(event) => onDetailsChange(event.target.value)} placeholder="Conte o que aconteceu..." className="min-h-28 w-full resize-y rounded-md border-2 border-mint/40 bg-card px-4 py-3 text-sm text-ink focus:border-mint-dark focus:outline-none focus:shadow-glow" />
              <p className="mt-1 text-right text-xs text-ink-muted">{reportDetails.length}/1000</p>
            </div>
            {error && <ErrorBanner>{error}</ErrorBanner>}
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
              <Button type="button" variant="outline" className="justify-center" onClick={onClose} disabled={reportPending}>Cancelar</Button>
              <Button type="submit" className="justify-center" disabled={reportPending}>{reportPending ? "Enviando..." : "Enviar denúncia"}</Button>
            </div>
          </form>
        ) : (
          <div className="mt-6 space-y-5">
            <div className={`rounded-2xl border px-4 py-4 text-sm ${destructive ? "border-danger/25 bg-danger-bg" : "border-mint/15 bg-bg/35"}`}>
              {action === "leave" ? "Você perderá o acesso ao bolão. Seus palpites e dados serão preservados caso entre novamente pelo código." : <><strong>Esta ação não pode ser desfeita automaticamente.</strong> Todos os participantes perderão o acesso e o bolão será excluído.</>}
            </div>
            {error && <ErrorBanner>{error}</ErrorBanner>}
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
              <Button variant="outline" className="justify-center" onClick={onClose} disabled={actionPending}>Cancelar</Button>
              <Button variant={action === "delete" ? "primary" : "outline"} className={action === "delete" ? "justify-center bg-danger text-white hover:bg-danger/90" : "justify-center"} onClick={action === "delete" ? onDelete : onLeave} disabled={actionPending}>{actionPending ? "Processando..." : action === "delete" ? "Excluir bolão" : "Sair do bolão"}</Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export function PoolOverviewPage() {
  const { poolId = "" } = useParams();
  const navigate = useNavigate();
  const { user } = useAuth();
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
