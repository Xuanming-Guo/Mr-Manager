use serde::{Deserialize, Serialize};

use super::{FeatureAvailability, ProcessKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkDiagnosticState {
    Pass,
    Warn,
    Fail,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkDiagnosticKind {
    GatewayReachability,
    DnsStatus,
    InternetLatency,
    InternetDnsResolution,
    PacketLoss,
    DownloadSpeed,
    UploadSpeed,
    RouteVpnBehavior,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkEvidence {
    pub source: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkTimelinePoint {
    pub collected_at_ms: u64,
    pub received_bytes_per_second: u64,
    pub transmitted_bytes_per_second: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterThroughput {
    pub received_bytes_per_second: u64,
    pub transmitted_bytes_per_second: u64,
    pub session_received_bytes: u64,
    pub session_transmitted_bytes: u64,
    pub total_received_bytes: u64,
    pub total_transmitted_bytes: u64,
    pub peak_received_bytes_per_second: u64,
    pub peak_transmitted_bytes_per_second: u64,
    pub timeline: Vec<NetworkTimelinePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapterSnapshot {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub adapter_type: String,
    pub operational_status: String,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub gateway_addresses: Vec<String>,
    pub dns_server_count: u32,
    pub link_speed_bits_per_second: Option<u64>,
    pub wifi_signal_quality_percent: Option<u8>,
    pub interface_metric: Option<u32>,
    pub lan_ip_candidates: Vec<String>,
    pub throughput: AdapterThroughput,
    pub evidence: Vec<NetworkEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpnState {
    pub likely_active: bool,
    pub confidence: String,
    pub label: String,
    pub evidence: Vec<NetworkEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub state: NetworkDiagnosticState,
    pub gateway: Option<String>,
    pub latency_ms: Option<u64>,
    pub local_only: bool,
    pub evidence: Vec<NetworkEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsStatus {
    pub state: NetworkDiagnosticState,
    pub local_only: bool,
    pub configured_server_count: u32,
    pub evidence: Vec<NetworkEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDevServerWarning {
    pub port: u16,
    pub address: String,
    pub process_name: Option<String>,
    pub message: String,
    pub remediation: String,
    pub lan_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerProcessNetworkEntry {
    pub key: ProcessKey,
    pub name: String,
    pub received_bytes_per_second: Option<u64>,
    pub transmitted_bytes_per_second: Option<u64>,
    pub evidence: Vec<NetworkEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerProcessNetworkUsage {
    pub availability: FeatureAvailability,
    pub entries: Vec<PerProcessNetworkEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDashboardSnapshot {
    pub collected_at_ms: u64,
    pub external_diagnostics_enabled: bool,
    pub combined: AdapterThroughput,
    pub adapters: Vec<NetworkAdapterSnapshot>,
    pub gateway_reachability: GatewayStatus,
    pub dns_status: DnsStatus,
    pub vpn_state: VpnState,
    pub lan_ip_candidates: Vec<String>,
    pub local_dev_server_warnings: Vec<LocalDevServerWarning>,
    pub per_process_usage: PerProcessNetworkUsage,
    pub privacy_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkDiagnosticRequest {
    pub kind: NetworkDiagnosticKind,
    pub consent_to_external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDiagnosticResult {
    pub label: String,
    pub state: NetworkDiagnosticState,
    pub value: Option<String>,
    pub local_only: bool,
    pub contacted_internet: bool,
    pub evidence: Vec<NetworkEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDiagnosticReport {
    pub kind: NetworkDiagnosticKind,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub local_only: bool,
    pub contacted_internet: bool,
    pub endpoints_contacted: Vec<String>,
    pub results: Vec<NetworkDiagnosticResult>,
    pub warnings: Vec<String>,
}
