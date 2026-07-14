import type { FeatureAvailability, ProcessKey } from "./system";

export type NetworkDiagnosticState = "pass" | "warn" | "fail" | "unavailable";
export type NetworkDiagnosticKind =
  | "gatewayReachability"
  | "dnsStatus"
  | "internetLatency"
  | "internetDnsResolution"
  | "packetLoss"
  | "downloadSpeed"
  | "uploadSpeed"
  | "routeVpnBehavior";

export interface NetworkEvidence {
  source: string;
  detail: string;
}

export interface NetworkTimelinePoint {
  collectedAtMs: number;
  receivedBytesPerSecond: number;
  transmittedBytesPerSecond: number;
}

export interface AdapterThroughput {
  receivedBytesPerSecond: number;
  transmittedBytesPerSecond: number;
  sessionReceivedBytes: number;
  sessionTransmittedBytes: number;
  totalReceivedBytes: number;
  totalTransmittedBytes: number;
  peakReceivedBytesPerSecond: number;
  peakTransmittedBytesPerSecond: number;
  timeline: NetworkTimelinePoint[];
}

export interface NetworkAdapterSnapshot {
  id: string;
  name: string;
  displayName: string;
  description: string | null;
  adapterType: string;
  operationalStatus: string;
  ipv4Addresses: string[];
  ipv6Addresses: string[];
  gatewayAddresses: string[];
  dnsServerCount: number;
  linkSpeedBitsPerSecond: number | null;
  wifiSignalQualityPercent: number | null;
  interfaceMetric: number | null;
  lanIpCandidates: string[];
  throughput: AdapterThroughput;
  evidence: NetworkEvidence[];
}

export interface VpnState {
  likelyActive: boolean;
  confidence: string;
  label: string;
  evidence: NetworkEvidence[];
}

export interface GatewayStatus {
  state: NetworkDiagnosticState;
  gateway: string | null;
  latencyMs: number | null;
  localOnly: boolean;
  evidence: NetworkEvidence[];
}

export interface DnsStatus {
  state: NetworkDiagnosticState;
  localOnly: boolean;
  configuredServerCount: number;
  evidence: NetworkEvidence[];
}

export interface LocalDevServerWarning {
  port: number;
  address: string;
  processName: string | null;
  message: string;
  remediation: string;
  lanUrls: string[];
}

export interface PerProcessNetworkEntry {
  key: ProcessKey;
  name: string;
  receivedBytesPerSecond: number | null;
  transmittedBytesPerSecond: number | null;
  evidence: NetworkEvidence[];
}

export interface PerProcessNetworkUsage {
  availability: FeatureAvailability;
  entries: PerProcessNetworkEntry[];
}

export interface NetworkDashboardSnapshot {
  collectedAtMs: number;
  externalDiagnosticsEnabled: boolean;
  combined: AdapterThroughput;
  adapters: NetworkAdapterSnapshot[];
  gatewayReachability: GatewayStatus;
  dnsStatus: DnsStatus;
  vpnState: VpnState;
  lanIpCandidates: string[];
  localDevServerWarnings: LocalDevServerWarning[];
  perProcessUsage: PerProcessNetworkUsage;
  privacyNote: string;
}

export interface NetworkDiagnosticRequest {
  kind: NetworkDiagnosticKind;
  consentToExternal: boolean;
}

export interface NetworkDiagnosticResult {
  label: string;
  state: NetworkDiagnosticState;
  value: string | null;
  localOnly: boolean;
  contactedInternet: boolean;
  evidence: NetworkEvidence[];
}

export interface NetworkDiagnosticReport {
  kind: NetworkDiagnosticKind;
  startedAtMs: number;
  completedAtMs: number;
  localOnly: boolean;
  contactedInternet: boolean;
  endpointsContacted: string[];
  results: NetworkDiagnosticResult[];
  warnings: string[];
}
