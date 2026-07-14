export type CleanupCategory =
  | "dependencyCache"
  | "frameworkCache"
  | "buildOutput"
  | "pythonEnvironment"
  | "generatedLog"
  | "archive"
  | "largeFile";

export type CleanupConfidence = "certain" | "strong" | "inferred";
export type CleanupRisk = "low" | "review" | "high";
export type CleanupLockState = "unknown" | "available" | "inUse";

export interface CleanupCandidate {
  id: string;
  rootPath: string;
  canonicalPath: string;
  displayName: string;
  category: CleanupCategory;
  reason: string;
  confidence: CleanupConfidence;
  risk: CleanupRisk;
  estimatedSizeBytes: number;
  fileCount: number;
  lockState: CleanupLockState;
  selected: boolean;
  regenerationInstructions: string;
  identityFingerprint: string;
  isDirectory: boolean;
}

export interface CleanupScanIssue {
  path: string;
  code: string;
  message: string;
}

export interface CleanupScanResult {
  scanId: string;
  operationId: string;
  roots: string[];
  candidates: CleanupCandidate[];
  issues: CleanupScanIssue[];
  visitedEntries: number;
  totalCandidateBytes: number;
  cancelled: boolean;
  limitsReached: boolean;
}

export interface CleanupScanRequest {
  operationId: string;
  rootPaths: string[];
}

export interface CleanupPlan {
  id: string;
  scanId: string;
  createdAtMs: number;
  roots: string[];
  items: CleanupCandidate[];
  totalSizeBytes: number;
  totalFileCount: number;
  state: "reviewed" | "executed";
  confirmationPhrase: string;
  manifestId: string | null;
}

export type QuarantineItemState =
  "pending" | "quarantined" | "partial" | "failed" | "restored" | "purged";
export type VerificationState =
  "pending" | "atomicMove" | "copyVerified" | "restoreVerified" | "failed";

export interface QuarantineItem {
  id: string;
  originalCanonicalPath: string;
  quarantinePath: string;
  restoredPath: string | null;
  sizeBytes: number;
  fileCount: number;
  category: CleanupCategory;
  reason: string;
  projectAssociation: string | null;
  state: QuarantineItemState;
  verification: VerificationState;
  purgeEligible: boolean;
  error: string | null;
  quarantinedAtMs: number | null;
  restoredAtMs: number | null;
  purgedAtMs: number | null;
  purgeConfirmationPhrase: string;
}

export interface QuarantineManifest {
  id: string;
  planId: string;
  createdAtMs: number;
  updatedAtMs: number;
  state: "inProgress" | "complete" | "partial" | "restored" | "purged";
  items: QuarantineItem[];
}

export interface PurgeQuarantineManifestRequest {
  manifestId: string;
  confirmation: string;
}

export type RestoreConflictStrategy = "fail" | "safeAlternative";
