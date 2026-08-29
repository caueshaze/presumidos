import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { ArrowLeft, Info } from "lucide-react";
import {
  usePools,
  useLeaderboard,
  usePoolAdjustments,
  useAddAdjustment,
  useCustomQuestions,
  useFootballScoring,
  useRemoveAdjustment,
} from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/ui/field";
import { AdjustmentPanels } from "./leaderboard/AdjustmentPanels";
import { LeaderboardRanking } from "./leaderboard/LeaderboardRanking";
import { ScoringInfoModal } from "./leaderboard/ScoringInfoModal";

export function LeaderboardPage() {
  const { user, isAdmin } = useAuth();
  const { poolId: routePoolId } = useParams();
  const [searchParams] = useSearchParams();
  const pools = usePools();
  const requestedPoolId = routePoolId ?? searchParams.get("poolId");
  const openedFromClosing = searchParams.get("from") === "closing";
  const [selectedPool, setSelectedPool] = useState(requestedPoolId ?? "");
  const [scoringOpen, setScoringOpen] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    if (!pools.data || pools.data.length === 0) return;
    const requestedPoolExists =
      requestedPoolId && pools.data.some((pool) => pool.id === requestedPoolId);
    if (requestedPoolExists && selectedPool !== requestedPoolId)
      setSelectedPool(requestedPoolId);
    else if (
      !selectedPool ||
      !pools.data.some((pool) => pool.id === selectedPool)
    )
      setSelectedPool(pools.data[0].id);
  }, [pools.data, requestedPoolId, selectedPool]);

  const leaderboard = useLeaderboard(selectedPool || null);
  const adjustments = usePoolAdjustments(selectedPool || null);
  const addAdjustment = useAddAdjustment();
  const removeAdjustment = useRemoveAdjustment();
  const currentPool = pools.data?.find((pool) => pool.id === selectedPool);
  const isFootball = currentPool?.event.kind !== "custom";
  const footballScoring = useFootballScoring(
    isFootball ? selectedPool || null : null,
  );
  const customQuestions = useCustomQuestions(
    !isFootball ? selectedPool || null : null,
  );
  const isOrganizer =
    !!currentPool && (currentPool.createdBy === user?.id || isAdmin);
  const entries = leaderboard.data ?? [];
  const adjustmentList = adjustments.data ?? [];

  const [adjUser, setAdjUser] = useState("");
  const [adjMode, setAdjMode] = useState<"add" | "subtract">("add");
  const [adjPoints, setAdjPoints] = useState("");
  const [adjReason, setAdjReason] = useState("");
  const [adjError, setAdjError] = useState("");

  useEffect(() => {
    setAdjUser("");
    setAdjMode("add");
    setAdjPoints("");
    setAdjReason("");
    setAdjError("");
  }, [selectedPool]);

  const onAdjust = async (event: FormEvent) => {
    event.preventDefault();
    setAdjError("");
    const points = parseInt(adjPoints, 10);
    if (!adjUser || Number.isNaN(points) || points <= 0) {
      setAdjError(
        "Escolha um membro e uma quantidade de pontos maior que zero.",
      );
      return;
    }
    try {
      await addAdjustment.mutateAsync({
        poolId: selectedPool,
        userId: adjUser,
        delta: adjMode === "subtract" ? -points : points,
        reason: adjReason.trim(),
      });
      setAdjUser("");
      setAdjMode("add");
      setAdjPoints("");
      setAdjReason("");
    } catch (error) {
      setAdjError(
        error instanceof Error ? error.message : "Falha ao lançar ajuste.",
      );
    }
  };
  const onRemoveAdjustment = async (adjustmentId: string) => {
    setAdjError("");
    try {
      await removeAdjustment.mutateAsync({
        poolId: selectedPool,
        adjustmentId,
      });
    } catch (error) {
      setAdjError(
        error instanceof Error ? error.message : "Falha ao remover ajuste.",
      );
    }
  };
  const openMemberPredictions = (userId: string) => {
    const from = openedFromClosing ? "&from=closing" : "";
    navigate(`/pools/${encodeURIComponent(selectedPool)}/members?memberId=${encodeURIComponent(userId)}${from}`);
  };

  return (
    <PageShell>
      <Button
        variant="link"
        size="sm"
        onClick={() =>
          navigate(routePoolId ? `/pools/${routePoolId}` : "/pools")
        }
      >
        <ArrowLeft className="h-4 w-4" /> Voltar ao bolão
      </Button>
      <h1 className="text-3xl">
        {currentPool?.event.isHistorical ? "Ranking final" : "Ranking"}
      </h1>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        {openedFromClosing && (
          <Button variant="outline" size="sm" onClick={() => navigate("/")}>
            <ArrowLeft className="h-4 w-4" /> Voltar ao resumo final
          </Button>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={() => setScoringOpen(true)}
        >
          <Info className="h-4 w-4" /> Como funciona a pontuação
        </Button>
      </div>
      {pools.isLoading ? (
        <Card className="mt-6">
          <p className="text-ink-muted">Carregando...</p>
        </Card>
      ) : pools.isError ? (
        <div className="mt-6">
          <ErrorBanner>
            Erro ao carregar bolões: {(pools.error as Error).message}
          </ErrorBanner>
        </div>
      ) : pools.data?.length === 0 ? (
        <Card className="mt-6">
          <h3 className="text-lg">Ainda não há ranking por aqui.</h3>
          <p className="mt-1 text-ink-muted">
            Crie um bolão ou entre com um código e deixe a disputa começar.
          </p>
        </Card>
      ) : (
        <>
          <LeaderboardRanking
            pools={pools.data ?? []}
            selectedPool={selectedPool}
            onSelectPool={setSelectedPool}
            showPoolSelector={!routePoolId}
            entries={entries}
            isFootball={isFootball}
            isLoading={leaderboard.isLoading}
            error={
              leaderboard.isError ? (leaderboard.error as Error).message : null
            }
            onOpenMember={openMemberPredictions}
          />
          <AdjustmentPanels
            entries={entries}
            adjustments={adjustmentList}
            isOrganizer={isOrganizer}
            isHistorical={!!currentPool?.event.isHistorical}
            form={{ adjUser, adjMode, adjPoints, adjReason, adjError }}
            onFormChange={{
              setAdjUser,
              setAdjMode,
              setAdjPoints,
              setAdjReason,
            }}
            onSubmit={onAdjust}
            onRemove={onRemoveAdjustment}
            isAdding={addAdjustment.isPending}
            isRemoving={removeAdjustment.isPending}
          />
        </>
      )}
      <ScoringInfoModal
        open={scoringOpen}
        onClose={() => setScoringOpen(false)}
        isFootball={isFootball}
        footballScoring={footballScoring.data}
        customQuestions={customQuestions.data}
      />
    </PageShell>
  );
}
