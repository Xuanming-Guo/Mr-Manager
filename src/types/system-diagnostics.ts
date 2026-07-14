import type { NetworkDashboardSnapshot, AdapterThroughput } from "./network";
import type {
  BatterySnapshot,
  FeatureAvailability,
  MemorySnapshot,
  ProcessKey,
  RefreshMode,
  SystemSnapshot,
} from "./system";

export interface GpuAdapterSnapshot {
  name: string;
  provider: string;
  utilizationPercent: number | null;
  vramUsedBytes: number | null;
  vramTotalBytes: number | null;
  temperatureCelsius: number | null;
  evidence: string[];
}

export interface GpuSnapshot {
  availability: FeatureAvailability;
  provider: string;
  adapters: GpuAdapterSnapshot[];
  collectedAtMs: number;
}

export interface RankedProcess {
  key: ProcessKey;
  name: string;
  executablePath: string | null;
  cpuPercent: number;
  memoryBytes: number;
  diskReadBytes: number;
  diskWriteBytes: number;
  listeningPortCount: number;
}

export interface RankedProcesses {
  topCpu: RankedProcess[];
  topMemory: RankedProcess[];
  topDiskIo: RankedProcess[];
  topGpu: FeatureAvailability;
}

export interface DockerActivitySnapshot {
  availability: FeatureAvailability;
  dockerProcessCount: number;
  dockerProcessNames: string[];
  evidence: string[];
}

export interface LocalDevServerSnapshot {
  port: number;
  localUrl: string | null;
  lanUrls: string[];
  processName: string | null;
  bindingScope: string;
}

export interface CollectorDiagnostics {
  collectionDurationMs: number;
  processCount: number;
  portCount: number;
  adapterCount: number;
  droppedRecordingSamples: number;
  warnings: string[];
}

export interface RecordingSystemSample {
  cpuTotalPercent: number;
  memory: MemorySnapshot;
  battery: BatterySnapshot;
  network: AdapterThroughput;
  diskReadBytes: number;
  diskWriteBytes: number;
}

export interface RecordingSample {
  sequence: number;
  collectedAtMs: number;
  localOnly: boolean;
  includedInternetDiagnostics: boolean;
  system: RecordingSystemSample;
  gpu: GpuSnapshot;
  topProcesses: RankedProcess[];
  network: NetworkDashboardSnapshot;
  docker: DockerActivitySnapshot;
  localDevServers: LocalDevServerSnapshot[];
}

export interface RecordingAnnotation {
  id: string;
  atMs: number;
  label: string;
}

export type RecordingStatus = "active" | "completed";

export interface CorrelationFinding {
  title: string;
  detail: string;
  evidence: string[];
  nonCausalDisclaimer: string;
}

export interface RecordingSessionSummary {
  id: string;
  name: string;
  status: RecordingStatus;
  startedAtMs: number;
  stoppedAtMs: number | null;
  sampleCount: number;
  annotationCount: number;
  downsampled: boolean;
  localOnly: boolean;
  findings: CorrelationFinding[];
}

export interface StartMetricRecordingRequest {
  name: string;
}

export interface AddMetricAnnotationRequest {
  label: string;
}

export interface MetricRecordingDetail {
  summary: RecordingSessionSummary;
  samples: RecordingSample[];
  annotations: RecordingAnnotation[];
}

export interface MetricRecordingExport {
  exportedAtMs: number;
  redactedByDefault: boolean;
  detail: MetricRecordingDetail;
}

export interface SystemDiagnosticsSnapshot {
  collectedAtMs: number;
  refreshMode: RefreshMode;
  system: SystemSnapshot;
  gpu: GpuSnapshot;
  rankedProcesses: RankedProcesses;
  network: NetworkDashboardSnapshot;
  docker: DockerActivitySnapshot;
  localDevServers: LocalDevServerSnapshot[];
  collectorDiagnostics: CollectorDiagnostics;
  activeRecording: RecordingSessionSummary | null;
  recentFindings: CorrelationFinding[];
}
