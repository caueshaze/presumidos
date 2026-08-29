import type { EventSummary } from "./events";
import type { PredictionReactionGroup } from "./matches-scoring";

export interface UserPublic {
  id: string;
  username: string;
  email: string;
  isAdmin: boolean;
  blockedAt: string | null;
  blockedReason: string | null;
}
export interface AuthResult {
  user: UserPublic;
  token: string;
  csrfToken: string;
}
export interface SessionState {
  user: UserPublic | null;
  csrfToken: string;
}
export interface PoolSummary {
  id: string;
  name: string;
  eventId: string;
  event: EventSummary;
  inviteCode: string;
  memberCount: number;
  createdBy: string;
  description: string;
  visibleRules: string;
  joinClosedAt: string | null;
}
export type PoolReportCategory =
  | "inappropriate_content"
  | "spam_or_fraud"
  | "harassment"
  | "other";
export type PoolReportStatus = "open" | "reviewing" | "resolved" | "dismissed";
export interface PoolReport {
  id: string;
  poolId: string;
  poolName: string;
  inviteCode: string;
  reporterUserId: string | null;
  reporterUsername: string | null;
  category: PoolReportCategory;
  details: string;
  status: PoolReportStatus;
  reviewedBy: string | null;
  reviewedByUsername: string | null;
  reviewedAt: string | null;
  createdAt: string;
  updatedAt: string;
}
export type InviteJoinStatus = "joinable" | "already_member" | "closed" | "invalid";
export interface PublicPoolInvitePreview {
  poolName: string | null;
  eventName: string | null;
  eventDescription: string | null;
  coverAssetUrl: string | null;
  coverUrl: string | null;
  creatorDisplayName: string | null;
  memberCount: number | null;
  lockDeadline: string | null;
  joinStatus: InviteJoinStatus;
  poolId: string | null;
}
export interface PoolDashboardSummary {
  pool: PoolSummary;
  answeredCount: number;
  itemCount: number;
}
export interface PoolPredictionRecord {
  matchId: string;
  homeScore: number;
  awayScore: number;
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHomeScore: number | null;
  penaltyAwayScore: number | null;
  reactions: PredictionReactionGroup[];
  viewerReaction: string | null;
  unreadReactionCount: number;
}
export interface FixtureCheckResult {
  eventId: number;
  found: boolean;
  label: string;
  status: string | null;
  kickoff: string | null;
  homeTeam: string | null;
  awayTeam: string | null;
}
