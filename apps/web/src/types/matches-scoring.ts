import type { PoolPredictionRecord } from "./identity-pools";

export interface PredictionReuseSuggestion {
  available: boolean;
  sourcePool: { name: string } | null;
  answered: number;
  copyable: number;
  total: number;
  locked: number;
}
export interface PredictionReuseResult {
  copiedCount: number;
  alreadyInitialized: boolean;
}
export interface FootballScoringConfig {
  exactScorePoints: number;
  correctResultExactSidePoints: number;
  correctResultPoints: number;
  incorrectResultPoints: number;
  knockoutBonusPoints: number;
}
export interface MatchRecord {
  id: string;
  homeTeam: string;
  awayTeam: string;
  kickoff: string;
  groupName: string | null;
  phase: string | null;
  homeScore: number | null;
  awayScore: number | null;
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHomeScore: number | null;
  penaltyAwayScore: number | null;
  finished: boolean;
  // Placar ao vivo (parcial) vindo do poller da API-Football. Só exibição.
  liveHomeScore?: number | null;
  liveAwayScore?: number | null;
  liveStatus?: string | null;
  liveElapsed?: number | null;
  resultSource?: string | null;
  resultSyncedAt?: string | null;
  resultExternalRawStatus?: string | null;
  liveUpdatedAt?: string | null;
}
export interface PredictionRecord {
  matchId: string;
  homeScore: number;
  awayScore: number;
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHomeScore: number | null;
  penaltyAwayScore: number | null;
}
export interface PredictionReactionGroup {
  emoji: string;
  count: number;
  reactedByViewer: boolean;
}
export interface KnockoutEntry {
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHome: number | null;
  penaltyAway: number | null;
}
export interface LeaderboardEntry {
  position: number;
  userId: string;
  username: string;
  points: number;
  /** Critérios de desempate (não incluem ajustes manuais). */
  exactScores: number;
  correctResults: number;
  bonusPoints: number;
}
export interface PointAdjustment {
  id: string;
  userId: string;
  username: string;
  delta: number;
  reason: string;
  createdAt: string;
}
export interface MemberPredictions {
  userId: string;
  username: string;
  unreadReactionCount: number;
  predictions: PoolPredictionRecord[];
}
export interface PredictionScoreBreakdown {
  poolId: string;
  poolName: string;
  userId: string;
  username: string;
  matchId: string;
  homeTeam: string;
  awayTeam: string;
  exactScorePoints: number;
  outcomePoints: number;
  goalBonusPoints: number;
  qualifierPoints: number;
  penaltiesPoints: number;
  totalPoints: number;
  eligible: boolean;
  eligibilityReason: string;
  officialSource: string | null;
  computedAt: string;
}
export interface MatchPointsSummary {
  matchId: string;
  exactScorePoints: number;
  outcomePoints: number;
  goalBonusPoints: number;
  qualifierPoints: number;
  penaltiesPoints: number;
  totalPoints: number;
  eligible: boolean;
}
