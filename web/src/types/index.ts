// Tipos espelhando os models Rust (camelCase via serde rename_all).

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

export interface EventSummary {
  id: string;
  name: string;
  slug: string;
  kind: "football" | "custom";
  status: "draft" | "active" | "finished";
  endsAt: string | null;
  isHistorical: boolean;
  coverUrl?: string | null;
  coverAssetUrl?: string | null;
  externalUrl?: string | null;
}

export interface PoolDashboardSummary {
  pool: PoolSummary;
  answeredCount: number;
  itemCount: number;
}

export interface EventRecord {
  id: string;
  name: string;
  slug: string;
  kind: "football" | "custom";
  status: "draft" | "active" | "finished";
  createdBy: string | null;
  startsAt: string | null;
  endsAt: string | null;
  createdAt: string;
  updatedAt: string;
  description?: string | null;
  coverUrl?: string | null;
  coverAssetUrl?: string | null;
  externalUrl?: string | null;
  poolCreationEnabled?: boolean;
  currentPublishedVersionId?: string | null;
}

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

export type ManifestAction = "create" | "noChange" | "safeUpdate" | "conflict" | "rejected";
export interface ManifestDiffEntry { category: string; path: string; change: string; }
export interface ManifestPreview {
  action: ManifestAction;
  name: string;
  slug: string;
  schemaVersion: number;
  itemCount: number;
  optionCount: number;
  linkCount: number;
  manifestFingerprint: string;
  baseFingerprint: string;
  safeChanges: ManifestDiffEntry[];
  blockedChanges: ManifestDiffEntry[];
}
export interface ManifestApplyResult {
  action: ManifestAction;
  eventId: string | null;
  itemCount: number;
  optionCount: number;
  linkCount: number;
  versionId: string | null;
  state: "working" | "published";
}

export interface CustomQuestionOption {
  id: string;
  label: string;
  sortOrder: number;
  imageUrl?: string | null;
  imageAssetUrl?: string | null;
  links?: OptionLink[];
  mediaSeen?: boolean;
}
export interface OptionLink { kind: "video" | "audio" | "official" | "other"; label: string; url: string; sortOrder: number; }
export interface EventShowcase { name: string; description: string | null; coverUrl: string | null; coverAssetUrl?: string | null; externalUrl: string | null; startsAt: string | null; endsAt: string | null; itemCount: number; answeredCount: number; isHistorical: boolean; }

export interface AssetResponse {
  assetId: string;
  sha256: string;
  mediaType: string;
  width: number;
  height: number;
  byteSize: number;
  url: string;
  variants: Record<string, string>;
}

export interface PackagePreview {
  manifest: ManifestPreview;
  assetCount: number;
  existingAssetCount: number;
  addedAssetCount: number;
}

export interface PackageApplyResult {
  result: ManifestApplyResult;
  assetCount: number;
  addedAssetCount: number;
}

export interface CustomQuestion {
  itemId: string;
  kind: "single_choice" | "numeric" | "multiple_choice";
  title: string;
  lockAt: string;
  revealAt: string;
  sortOrder: number;
  status: "draft" | "open" | "locked" | "resolved";
  currentOptionId: string | null;
  correctOptionId: string | null;
  correctPoints: number;
  incorrectPoints: number;
  options: CustomQuestionOption[];
  decimalPlaces?: number;
  unitLabel?: string | null;
  minValue?: string | null;
  maxValue?: string | null;
  currentValue?: string | null;
  resultValue?: string | null;
  resultStatus?: "resolved" | "not_representable" | "pending_decision" | null;
  exactPoints?: number;
  tolerance?: string;
  withinTolerancePoints?: number;
  minSelections?: number;
  maxSelections?: number | null;
  currentOptionIds?: string[];
  correctOptionIds?: string[];
  partialPoints?: number;
}
export interface CustomMemberPredictions { userId: string; username: string; predictions: { itemId: string; title: string; optionLabel: string }[]; }

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
  liveHomeScore: number | null;
  liveAwayScore: number | null;
  liveStatus: string | null;
  liveElapsed: number | null;
  resultSource: string | null;
  resultSyncedAt: string | null;
  resultExternalRawStatus: string | null;
  liveUpdatedAt: string | null;
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

export interface KnockoutEntry {
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHome: number | null;
  penaltyAway: number | null;
}

export interface LeaderboardEntry {
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

export interface NotificationPreference {
  enabled: boolean;
  leadTimeMinutes: 10 | 20 | 30;
  reactionEnabled: boolean;
}

export interface WebPushSubscriptionKeys {
  p256dh: string;
  auth: string;
}

export interface WebPushSubscriptionInput {
  endpoint: string;
  expirationTime: number | null;
  keys: WebPushSubscriptionKeys;
  userAgent?: string | null;
  deviceLabel?: string | null;
}

export interface NotificationStatus {
  webPushEnabled: boolean;
  vapidPublicKey: string | null;
  preference: NotificationPreference;
  activeSubscriptionCount: number;
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
  externalFixtureId: number | null;
  /** Sugestão de mata-mata auto-detectada pelo poller (aguardando confirmação). */
  autoHomeScore: number | null;
  autoAwayScore: number | null;
  autoPenaltyHomeScore: number | null;
  autoPenaltyAwayScore: number | null;
  autoQualifier: string | null;
  autoStatus: string | null;
  autoDetectedAt: string | null;
  sourceLastCheckedAt: string | null;
  sourceLastStatus: string | null;
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
