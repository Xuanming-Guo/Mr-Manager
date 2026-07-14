import type { PortProtocol } from "./system";

export type DockerAvailability =
  "cliMissing" | "installedStopped" | "starting" | "inaccessible" | "running" | "error";

export type DockerDesktopProcessState = "running" | "notDetected" | "unknown";
export type DockerIssueSeverity = "error" | "warning" | "information";
export type DockerContainerActionKind = "start" | "stop" | "restart";
export type ComposeParseSource = "dockerComposeConfig" | "fallbackParser";

export interface DockerDiagnostic {
  code: string;
  severity: DockerIssueSeverity;
  message: string;
  remediation: string | null;
  evidence: string[];
}

export interface DockerStatus {
  availability: DockerAvailability;
  cliDetected: boolean;
  daemonReachable: boolean;
  clientVersion: string | null;
  serverVersion: string | null;
  context: string | null;
  dockerDesktopProcess: DockerDesktopProcessState;
  collectedAtMs: number;
  diagnostics: DockerDiagnostic[];
}

export interface DockerLabel {
  key: string;
  value: string;
}

export interface DockerPortMapping {
  hostIp: string | null;
  hostPort: number | null;
  containerPort: number;
  protocol: PortProtocol;
}

export interface DockerMount {
  kind: string;
  source: string | null;
  destination: string;
  mode: string | null;
  readWrite: boolean | null;
  name: string | null;
}

export interface DockerResourceUsage {
  cpuPercent: string | null;
  memoryUsage: string | null;
  memoryPercent: string | null;
  networkIo: string | null;
  blockIo: string | null;
}

export interface DockerProjectAssociation {
  projectId: string;
  projectName: string;
  projectRoot: string;
  confidence: string;
  evidence: string[];
}

export interface DockerContainer {
  id: string;
  shortId: string;
  name: string;
  image: string;
  state: string;
  status: string;
  health: string | null;
  createdAt: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  ports: DockerPortMapping[];
  networks: string[];
  mounts: DockerMount[];
  labels: DockerLabel[];
  composeProject: string | null;
  composeService: string | null;
  composeWorkingDir: string | null;
  user: string | null;
  restartPolicy: string | null;
  resourceUsage: DockerResourceUsage | null;
  associatedProject: DockerProjectAssociation | null;
}

export interface DockerContainerActionRequest {
  containerId: string;
  action: DockerContainerActionKind;
  confirmation: string;
}

export interface DockerContainerActionResult {
  action: DockerContainerActionKind;
  container: DockerContainer;
  stdout: string;
}

export interface DockerLogEntry {
  sequence: number;
  timestamp: string | null;
  line: string;
}

export interface DockerNetwork {
  name: string;
  id: string | null;
  driver: string | null;
  scope: string | null;
}

export interface DockerVolume {
  name: string;
  driver: string | null;
  scope: string | null;
  mountpoint: string | null;
}

export interface DockerInventory {
  status: DockerStatus;
  containers: DockerContainer[];
  networks: DockerNetwork[];
  volumes: DockerVolume[];
}

export interface ComposePortMapping {
  hostIp: string | null;
  hostPort: number | null;
  containerPort: number;
  protocol: PortProtocol;
  raw: string;
}

export interface ComposeVolumeMount {
  source: string | null;
  target: string | null;
  mode: string | null;
  raw: string;
}

export interface ComposeService {
  name: string;
  image: string | null;
  build: string | null;
  containerName: string | null;
  command: string | null;
  user: string | null;
  restart: string | null;
  ports: ComposePortMapping[];
  volumes: ComposeVolumeMount[];
  environmentKeys: string[];
  dependsOn: string[];
  networks: string[];
  profiles: string[];
  healthcheckPresent: boolean;
}

export interface ComposeNetwork {
  name: string;
  external: boolean;
}

export interface ComposeVolume {
  name: string;
  external: boolean;
}

export interface ComposeDoctorIssue {
  code: string;
  severity: DockerIssueSeverity;
  service: string | null;
  message: string;
  remediation: string | null;
  evidence: string[];
}

export interface ComposeProject {
  id: string;
  projectId: string;
  projectName: string;
  projectRoot: string;
  composeFile: string;
  source: ComposeParseSource;
  services: ComposeService[];
  networks: ComposeNetwork[];
  volumes: ComposeVolume[];
  unresolvedInterpolation: string[];
  parseDiagnostics: DockerDiagnostic[];
  doctor: ComposeDoctorIssue[];
}
