import type { EventRecord } from "./events";
import type { MatchRecord, PredictionRecord } from "./matches-scoring";
import type { UserPublic } from "./identity-pools";

export interface AdminEventRecord extends EventRecord {
  createdByUsername: string | null;
  itemCount: number;
  optionCount: number;
  poolCount: number;
  poolCreationEnabled: boolean;
  currentPublishedVersionId: string | null;
  workingVersionId: string | null;
  currentVersionNumber: number | null;
}
export interface AdminPushRequest {
  title: string;
  body: string;
  url?: string;
}
export interface AdminPushResult {
  targetUserId: string | null;
  targetUserCount: number;
  activeSubscriptionCount: number;
  attemptedCount: number;
  successfulCount: number;
  failedCount: number;
  deactivatedCount: number;
}
export interface AdminActivityItem {
  id: string;
  action: string;
  label: string;
  at: string;
  targetId: string | null;
}
export interface SyncStatus {
  id: string;
  status: string;
  triggerSource: string;
  startedAt: string;
  finishedAt: string | null;
  summaryJson: string;
}
export interface AdminOverview {
  scheduledMatches: number;
  liveMatches: number;
  finalizedMatches: number;
  manuallyCorrectedMatches: number;
  overdueMatches: number;
  apiConflicts: number;
  usersWithoutPredictionsSoon: number;
  poolCount: number;
  userCount: number;
  blockedUserCount: number;
  lastSync: SyncStatus | null;
  syncEnabled: boolean;
  activityFeed: AdminActivityItem[];
}
export interface AdminMatchRecord {
  matchRecord: MatchRecord;
  adminStatus: string;
  lastAuditAt: string | null;
  /** Id do evento no provedor externo de placares (mapeamento manual do admin). */
  externalFixtureId?: number | null;
  /** Sugestão de mata-mata auto-detectada pelo poller (aguardando confirmação). */
  autoHomeScore?: number | null;
  autoAwayScore?: number | null;
  autoPenaltyHomeScore?: number | null;
  autoPenaltyAwayScore?: number | null;
  autoQualifier?: string | null;
  autoStatus?: string | null;
  autoDetectedAt?: string | null;
  sourceLastCheckedAt?: string | null;
  sourceLastStatus?: string | null;
}
export interface PredictionReopenOverride {
  id: string;
  matchId: string;
  userId: string;
  reason: string;
  reopenedBy: string;
  expiresAt: string;
  usedAt: string | null;
  createdAt: string;
  revokedAt: string | null;
}
export interface AdminPredictionRow {
  userId: string;
  username: string;
  poolId: string | null;
  poolName: string | null;
  matchId: string;
  homeTeam: string;
  awayTeam: string;
  kickoff: string;
  phase: string | null;
  prediction: PredictionRecord | null;
  locked: boolean;
  missing: boolean;
  overrideInfo: PredictionReopenOverride | null;
}
export interface ScoringJob {
  id: string;
  scopeType: string;
  scopeId: string | null;
  triggeredBy: string | null;
  status: string;
  startedAt: string;
  finishedAt: string | null;
  summaryJson: string;
}
export interface AdminUserRecord {
  user: UserPublic;
  poolCount: number;
}
export interface AuditLogEntry {
  id: string;
  actorUserId: string | null;
  actorUsername: string | null;
  action: string;
  targetType: string;
  targetId: string | null;
  ipAddress: string | null;
  detailsJson: string;
  createdAt: string;
}
export interface AdminSettings {
  knockoutReleased: boolean;
  autoSyncEnabled: boolean;
  syncIntervalMinutes: number;
  predictionLockMinutes: number;
  globalBannerEnabled: boolean;
  globalBannerText: string;
  finalThemeEnabled: boolean;
  closingScreenEnabled: boolean;
  featuredPoolId: string | null;
  featuredPool?: FeaturedPool | null;
}
export interface FeaturedPool {
  poolId: string;
  poolName: string;
  eventName: string;
  eventKind: "football" | "custom";
  isHistorical: boolean;
  memberCount: number;
  canJoin: boolean;
  joinCode?: string | null;
}
