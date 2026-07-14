export type ScanState = "healthy" | "warning" | "error" | "unavailable";
export type IssueSeverity = "error" | "warning" | "information" | "unsupported";

export interface ProjectManifest {
  kind: string;
  relativePath: string;
}

export interface PackageManagerSummary {
  name: string;
  evidence: string[];
  conflictingLockfiles: string[];
}

export interface ProjectScript {
  id: string;
  label: string;
  source: string;
  executable: string;
  arguments: string[];
  workingDirectory: string;
  verified: boolean;
}

export interface GitSummary {
  availability: "available" | "unavailable" | "notRepository" | "error";
  branch: string | null;
  detachedHead: boolean;
  ahead: number | null;
  behind: number | null;
  staged: number;
  modified: number;
  deleted: number;
  renamed: number;
  conflicted: number;
  untracked: number;
  lastCommit: string | null;
  remotes: string[];
}

export interface EnvironmentFileSummary {
  relativePath: string;
  keyNames: string[];
  example: boolean;
  sizeBytes: number;
}

export interface ProjectIssue {
  code: string;
  severity: IssueSeverity;
  message: string;
  remediation: string | null;
}

export interface ProjectScanHealth {
  state: ScanState;
  issues: ProjectIssue[];
}

export interface ChecklistItem {
  id: string;
  text: string;
  completed: boolean;
}

export interface Project {
  id: string;
  name: string;
  rootPath: string;
  canonicalRootPath: string;
  tags: string[];
  notes: string;
  checklist: ChecklistItem[];
  pinned: boolean;
  archived: boolean;
  detectedStacks: string[];
  manifests: ProjectManifest[];
  packageManager: PackageManagerSummary | null;
  scripts: ProjectScript[];
  gitSummary: GitSummary | null;
  composeFiles: string[];
  environmentFiles: EnvironmentFileSummary[];
  localDatabaseHints: string[];
  lastScannedAt: number | null;
  scanHealth: ProjectScanHealth;
}

export interface ProjectDiscoveryResult {
  operationId: string;
  projects: Project[];
  scannedDirectories: number;
  skippedDirectories: number;
  cancelled: boolean;
  issues: ProjectIssue[];
}

export type ManagedCommandState = "starting" | "running" | "stopping" | "exited" | "failed";
export type ManagedCommandStream = "stdout" | "stderr" | "system";

export interface ManagedCommandLogEntry {
  sequence: number;
  timestampMs: number;
  stream: ManagedCommandStream;
  line: string;
}

export interface ManagedCommand {
  runId: string;
  projectId: string;
  scriptId: string;
  label: string;
  executable: string;
  arguments: string[];
  workingDirectory: string;
  pid: number | null;
  startedAtMs: number;
  endedAtMs: number | null;
  state: ManagedCommandState;
  exitCode: number | null;
  stopRequested: boolean;
  logCount: number;
}
