export type AvailabilityState =
  "available" | "unavailable" | "unsupported" | "permissionDenied" | "error";

export interface FeatureAvailability {
  state: AvailabilityState;
  reason: string | null;
  remediation: string | null;
}

export interface CollectorIssue {
  code: string;
  message: string;
  remediation: string | null;
  permissionRelevant: boolean;
}

export interface CpuSnapshot {
  totalUsagePercent: number;
  logicalCoreCount: number;
  physicalCoreCount: number | null;
  perCoreUsagePercent: number[];
}

export interface MemorySnapshot {
  totalBytes: number;
  usedBytes: number;
  availableBytes: number;
  swapTotalBytes: number;
  swapUsedBytes: number;
}

export interface DiskSnapshot {
  name: string;
  mountPoint: string;
  kind: string;
  totalBytes: number;
  availableBytes: number;
  removable: boolean;
}

export interface NetworkThroughputSnapshot {
  receivedBytesPerSecond: number;
  transmittedBytesPerSecond: number;
  totalReceivedBytes: number;
  totalTransmittedBytes: number;
}

export interface BatterySnapshot {
  availability: FeatureAvailability;
  percentage: number | null;
  acOnline: boolean | null;
  remainingSeconds: number | null;
}

export interface SystemSnapshot {
  collectedAtMs: number;
  sequence: number;
  operatingSystem: string;
  operatingSystemVersion: string | null;
  kernelVersion: string | null;
  hostName: string | null;
  uptimeSeconds: number;
  cpu: CpuSnapshot;
  memory: MemorySnapshot;
  disks: DiskSnapshot[];
  network: NetworkThroughputSnapshot;
  battery: BatterySnapshot;
  gpu: FeatureAvailability;
  issues: CollectorIssue[];
}

export interface ProcessKey {
  pid: number;
  startTime: number;
}

export interface ProcessSnapshot {
  key: ProcessKey;
  parentPid: number | null;
  name: string;
  executablePath: string | null;
  cwd: string | null;
  commandLineRedacted: string | null;
  status: string;
  cpuPercent: number;
  memoryBytes: number;
  diskReadBytes: number;
  diskWriteBytes: number;
  protectedState: "unknown" | "accessible" | "protected";
  managedByMrManager: boolean;
  listeningPortCount: number;
}

export type PortProtocol = "tcp" | "udp";
export type BindingScope = "loopback" | "allInterfaces" | "specificInterface";

export interface PortEndpoint {
  protocol: PortProtocol;
  localAddress: string;
  localPort: number;
  state: string;
  owningProcessKey: ProcessKey | null;
  owningProcessName: string | null;
  bindingScope: BindingScope;
  inferredScheme: string | null;
  localUrl: string | null;
  lanUrls: string[];
  reachabilityState: "notTested" | "localSelfTestOnly" | "unreachable";
  evidence: string[];
}

export interface ProcessSummary {
  total: number;
  accessible: number;
  topCpu: ProcessSnapshot[];
  topMemory: ProcessSnapshot[];
}

export interface PortSummary {
  totalListening: number;
  developmentListeners: number;
  endpoints: PortEndpoint[];
}

export interface OverviewSnapshot {
  system: SystemSnapshot;
  processes: ProcessSummary;
  ports: PortSummary;
  collectorIssues: CollectorIssue[];
}

export type RefreshMode = "normal" | "fast";

export interface AppSettings {
  refreshMode: RefreshMode;
  externalNetworkChecks: boolean;
  metricHistoryEnabled: boolean;
  reducedMotion: boolean;
}

export interface CapabilityEntry {
  id: string;
  label: string;
  availability: FeatureAvailability;
  readOnly: boolean;
}

export interface CapabilityReport {
  platform: string;
  standardUserMode: boolean;
  features: CapabilityEntry[];
}

export interface AppError {
  code: string;
  message: string;
  remediation: string | null;
  technicalDetails: string | null;
  retryable: boolean;
  permissionRelevant: boolean;
}
