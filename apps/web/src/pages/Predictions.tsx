import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { useMatches, useMyPredictions, useMyPredictionOverrides, useMyMatchPoints, useKnockoutReleased, useCustomQuestions, usePools } from "@/hooks/queries";
import type { MatchPointsSummary } from "@/types";
import { CustomPredictionsView } from "./predictions/CustomPredictionsView";
import { FootballPredictionsView } from "./predictions/FootballPredictionsView";
import { matchesSearch, phaseRank } from "./predictions/utils";

export function PredictionsPage() {
  const { isAdmin } = useAuth();
  const navigate = useNavigate();
  const { poolId: routePoolId } = useParams();
  const [searchParams] = useSearchParams();
  const pools = usePools();
  const poolId = routePoolId ?? searchParams.get("poolId") ?? pools.data?.[0]?.id ?? null;
  const currentPool = pools.data?.find((pool) => pool.id === poolId);
  const customQuestions = useCustomQuestions(currentPool?.event.kind === "custom" ? poolId : null);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [phaseFilter, setPhaseFilter] = useState<string | null>(null);
  const [hideFinished, setHideFinished] = useState(true);
  const matches = useMatches();
  const predictions = useMyPredictions(poolId);
  const overrides = useMyPredictionOverrides();
  const matchPoints = useMyMatchPoints();
  const knockout = useKnockoutReleased();

  const pointsByMatch = useMemo(() => {
    const map = new Map<string, MatchPointsSummary>();
    for (const p of matchPoints.data ?? []) map.set(p.matchId, p);
    return map;
  }, [matchPoints.data]);

  // Reaberturas administrativas liberam o palpite mesmo com a partida travada por horário.
  const reopenedMatchIds = useMemo(
    () => new Set((overrides.data ?? []).map((o) => o.matchId)),
    [overrides.data],
  );

  const isLoading = matches.isLoading || predictions.isLoading || knockout.isLoading;
  const error = matches.error || predictions.error || knockout.error;
  const targetMatchId = searchParams.get("matchId");
  const allMatches = useMemo(() => matches.data ?? [], [matches.data]);

  const finishedCount = useMemo(
    () => allMatches.filter((game) => game.finished).length,
    [allMatches],
  );

  // Fases presentes nos dados, na ordem natural da competição.
  const phases = useMemo(() => {
    const set = new Set<string>();
    for (const game of allMatches) if (game.phase) set.add(game.phase);
    return [...set].sort((a, b) => phaseRank(a) - phaseRank(b) || a.localeCompare(b));
  }, [allMatches]);

  // Um jogo passa quando bate todos os filtros — mas o jogo do deep-link
  // (?matchId=) é sempre mostrado, para não "sumir" ao chegar por um link.
  const visibleMatches = useMemo(
    () =>
      allMatches.filter((game) => {
        if (game.id === targetMatchId) return true;
        if (hideFinished && game.finished) return false;
        if (phaseFilter && game.phase !== phaseFilter) return false;
        if (!matchesSearch(game, search)) return false;
        return true;
      }),
    [allMatches, hideFinished, phaseFilter, search, targetMatchId],
  );

  const hiddenFinishedCount = useMemo(
    () =>
      hideFinished
        ? allMatches.filter((game) => game.finished && game.id !== targetMatchId).length
        : 0,
    [allMatches, hideFinished, targetMatchId],
  );

  const activeFilterCount =
    (search.trim() ? 1 : 0) + (phaseFilter ? 1 : 0) + (!hideFinished ? 1 : 0);

  const clearFilters = () => {
    setSearch("");
    setPhaseFilter(null);
    setHideFinished(true);
  };

  useEffect(() => {
    if (!targetMatchId || visibleMatches.length === 0) return;
    const element = document.getElementById(`match-card-${targetMatchId}`);
    if (!element) return;

    const timer = window.setTimeout(() => {
      element.scrollIntoView({ behavior: "smooth", block: "start" });
    }, 120);

    return () => window.clearTimeout(timer);
  }, [targetMatchId, visibleMatches]);


  if (currentPool?.event.kind === "custom") {
    return <CustomPredictionsView context={{ navigate, poolId, currentPool, customQuestions }} />;
  }

  return <FootballPredictionsView context={{ navigate, poolId, currentPool, isAdmin, knockout, isLoading, error, allMatches, filtersOpen, setFiltersOpen, activeFilterCount, visibleMatches, search, setSearch, phases, phaseFilter, setPhaseFilter, finishedCount, hideFinished, setHideFinished, hiddenFinishedCount, clearFilters, predictions, reopenedMatchIds, pointsByMatch, targetMatchId }} />;
}
