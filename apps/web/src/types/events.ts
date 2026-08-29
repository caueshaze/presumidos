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
export interface EventRecord {
  id: string;
  name: string;
  slug: string;
  kind: "football" | "custom";
  origin: "system" | "user";
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
  archivedAt?: string | null;
}
export interface EventVersionHistory {
  id: string;
  versionNumber: number;
  state: "working" | "published";
  isCurrentPublished: boolean;
  name: string;
  fingerprint: string;
  baseFingerprint: string | null;
  createdAt: string;
  updatedAt: string;
  itemCount: number;
  optionCount: number;
  poolCount: number;
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

export interface PackageExportPreview {
  assetCount: number;
  externalImageCount: number;
  externalImages: { question: string; optionLabel: string | null; url: string }[];
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
export interface CustomMemberPredictions { userId: string; username: string; predictions: { itemId: string; title: string; optionLabel: string; points: number | null }[]; }
