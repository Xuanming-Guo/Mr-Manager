import type { FeatureAvailability, ProcessKey } from "./system";

export type IntegrationCategory =
  | "runtime"
  | "packageManager"
  | "editor"
  | "container"
  | "localAi"
  | "database"
  | "shell"
  | "vpn"
  | "localService";

export type IntegrationInstalledState = "installed" | "notFound" | "unknown" | "error";
export type IntegrationRunningState = "running" | "stopped" | "unknown" | "unsupported";
export type EvidenceConfidence = "certain" | "strong" | "inferred";

export interface IntegrationEvidence {
  source: string;
  detail: string;
  confidence: EvidenceConfidence;
}

export interface IntegrationEndpoint {
  label: string;
  url: string | null;
  port: number | null;
  localOnly: boolean;
  evidence: string;
}

export interface IntegrationProcessRef {
  key: ProcessKey;
  name: string;
  executablePath: string | null;
}

export interface IntegrationStatus {
  detectorId: string;
  displayName: string;
  category: IntegrationCategory;
  installedState: IntegrationInstalledState;
  runningState: IntegrationRunningState;
  version: string | null;
  executablePaths: string[];
  processes: IntegrationProcessRef[];
  endpoints: IntegrationEndpoint[];
  capabilities: string[];
  evidence: IntegrationEvidence[];
  lastCheckedAtMs: number;
  errors: string[];
}

export interface OllamaModel {
  name: string;
  model: string | null;
  sizeBytes: number | null;
  digest: string | null;
  modifiedAt: string | null;
  format: string | null;
  family: string | null;
  parameterSize: string | null;
  quantizationLevel: string | null;
  loaded: boolean;
  expiresAt: string | null;
  sizeVramBytes: number | null;
}

export interface OllamaStatus {
  availability: FeatureAvailability;
  endpoint: string | null;
  version: string | null;
  installedModels: OllamaModel[];
  runningModels: OllamaModel[];
  processes: IntegrationProcessRef[];
  evidence: IntegrationEvidence[];
  lastCheckedAtMs: number;
  errors: string[];
}

export interface WslDistribution {
  name: string;
  state: string;
  version: number | null;
}

export interface WslStatus {
  availability: FeatureAvailability;
  distros: WslDistribution[];
  evidence: IntegrationEvidence[];
  lastCheckedAtMs: number;
  errors: string[];
}
