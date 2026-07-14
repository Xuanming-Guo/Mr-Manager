import type { CleanupScanResult, QuarantineManifest } from "./cleaner";
import type { NetworkDiagnosticReport } from "./network";
import type { ProjectDiscoveryResult } from "./projects";
import type { AppError } from "./system";

export type BackgroundTaskKind =
  "cleanupScan" | "quarantine" | "projectDiscovery" | "internetDiagnostics";

export type BackgroundTaskState = "running" | "cancelling" | "succeeded" | "failed" | "cancelled";

export interface BackgroundTask {
  id: string;
  kind: BackgroundTaskKind;
  label: string;
  route: string;
  state: BackgroundTaskState;
  startedAtMs: number;
  completedAtMs: number | null;
  cancellable: boolean;
  progressPercent: number | null;
  summary: string | null;
  error: AppError | null;
}

export type BackgroundTaskOutput =
  | { kind: "cleanupScan"; value: CleanupScanResult }
  | { kind: "quarantine"; value: QuarantineManifest }
  | { kind: "projectDiscovery"; value: ProjectDiscoveryResult }
  | { kind: "internetDiagnostics"; value: NetworkDiagnosticReport[] };

export interface BackgroundTaskDetail {
  task: BackgroundTask;
  output: BackgroundTaskOutput | null;
}

export interface StartProjectDiscoveryTaskRequest {
  rootPath: string;
  operationId: string;
  maximumDepth?: number;
}

export interface StartQuarantineTaskRequest {
  planId: string;
  confirmation: string;
}
