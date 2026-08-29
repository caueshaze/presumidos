// @ts-nocheck
import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import {
  useMarkPredictionReactionsSeen, useMatches, useLeaderboard, usePoolBreakdowns,
  useCustomMemberPredictions, usePoolMemberPredictions, usePools, useReactToPrediction,
} from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import type { MatchRecord, MemberPredictions, PredictionScoreBreakdown } from "@/types";
import { CustomPredictionsView } from "./pool-predictions/CustomPredictionsView";
import { FootballPredictionsView } from "./pool-predictions/FootballPredictionsView";

export function PoolPredictionsPage() {
  const pools = usePools();
  const { user } = useAuth();
  const { poolId: routePoolId } = useParams();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const poolIdParam = routePoolId ?? searchParams.get("poolId");
  const memberIdParam = searchParams.get("memberId");
  const matchIdParam = searchParams.get("matchId");
  const openedFromClosing = searchParams.get("from") === "closing";
  const [selectedPool, setSelectedPool] = useState("");
  const [selectedMemberId, setSelectedMemberId] = useState<string | null>(null);
  const [openReactionMatchId, setOpenReactionMatchId] = useState<string | null>(null);
  const [lastSeenKey, setLastSeenKey] = useState("");

  useEffect(() => {
    if (selectedPool || !pools.data || pools.data.length === 0) return;
    const wanted =
      poolIdParam && pools.data.some((p) => p.id === poolIdParam) ? poolIdParam : pools.data[0].id;
    setSelectedPool(wanted);
  }, [pools.data, selectedPool, poolIdParam]);

  useEffect(() => {
    setSelectedMemberId(null);
    setOpenReactionMatchId(null);
    setLastSeenKey("");
  }, [selectedPool]);

  const members = usePoolMemberPredictions(selectedPool || null);
  const currentPool = pools.data?.find((pool) => pool.id === selectedPool);
  const customMembers = useCustomMemberPredictions(currentPool?.event.kind === "custom" ? selectedPool || null : null);
  const matches = useMatches();
  const breakdowns = usePoolBreakdowns(selectedPool || null);
  const leaderboard = useLeaderboard(selectedPool || null);
  const reactToPrediction = useReactToPrediction();
  const markSeen = useMarkPredictionReactionsSeen();

  const matchById = useMemo(() => {
    const map = new Map<string, MatchRecord>();
    for (const m of matches.data ?? []) map.set(m.id, m);
    return map;
  }, [matches.data]);

  const breakdownByKey = useMemo(() => {
    const map = new Map<string, PredictionScoreBreakdown>();
    for (const b of breakdowns.data ?? []) map.set(`${b.userId}:${b.matchId}`, b);
    return map;
  }, [breakdowns.data]);

  const entries: MemberPredictions[] = members.data ?? [];

  useEffect(() => {
    if (selectedMemberId || !memberIdParam || entries.length === 0) return;
    if (entries.some((entry) => entry.userId === memberIdParam)) {
      setSelectedMemberId(memberIdParam);
    }
  }, [entries, memberIdParam, selectedMemberId]);

  useEffect(() => {
    setOpenReactionMatchId(null);
  }, [selectedMemberId]);

  const selectedMember = entries.find((m) => m.userId === selectedMemberId) ?? null;
  const selectedMemberScore = (leaderboard.data ?? []).find(
    (entry) => entry.userId === selectedMember?.userId,
  );
  const settledPredictions = selectedMember
    ? selectedMember.predictions.filter((prediction) =>
        breakdownByKey.has(`${selectedMember.userId}:${prediction.matchId}`),
      ).length
    : 0;
  const correctPercentage = selectedMemberScore && settledPredictions > 0
    ? Math.round((selectedMemberScore.correctResults / settledPredictions) * 100)
    : 0;

  useEffect(() => {
    if (!selectedPool || !selectedMember || !user) return;
    if (selectedMember.userId !== user.id) return;
    if (selectedMember.unreadReactionCount <= 0) return;
    const seenKey = `${selectedPool}:${selectedMember.userId}:${selectedMember.unreadReactionCount}`;
    if (lastSeenKey === seenKey) return;
    if (markSeen.isPending) return;
    setLastSeenKey(seenKey);
    markSeen.mutate(selectedPool);
  }, [lastSeenKey, markSeen, selectedMember, selectedPool, user]);


  const context = {
    pools, user, navigate, selectedPool, setSelectedPool, currentPool, customMembers,
    members, matches, entries, selectedMember, selectedMemberScore, correctPercentage,
    selectedMemberId, setSelectedMemberId, openedFromClosing, matchIdParam, matchById,
    breakdownByKey, reactToPrediction, openReactionMatchId, setOpenReactionMatchId,
    showPoolSelector: !routePoolId,
  };

  if (currentPool?.event.kind === "custom") {
    return <CustomPredictionsView context={context} />;
  }

  return <FootballPredictionsView context={context} />;
}
