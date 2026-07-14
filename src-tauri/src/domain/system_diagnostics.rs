use serde::{Deserialize, Serialize};

use super::{
    AdapterThroughput, BatterySnapshot, FeatureAvailability, MemorySnapshot,
    NetworkDashboardSnapshot, ProcessKey, RefreshMode, SystemSnapshot,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuAdapterSnapshot {
    pub name: String,
    pub provider: String,
    pub utilization_percent: Option<f64>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub temperature_celsius: Option<f64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSnapshot {
    pub availability: FeatureAvailability,
    pub provider: String,
    pub adapters: Vec<GpuAdapterSnapshot>,
    pub collected_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedProcess {
    pub key: ProcessKey,
    pub name: String,
    pub executable_path: Option<String>,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub listening_port_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedProcesses {
    pub top_cpu: Vec<RankedProcess>,
    pub top_memory: Vec<RankedProcess>,
    pub top_disk_io: Vec<RankedProcess>,
    pub top_gpu: FeatureAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerActivitySnapshot {
    pub availability: FeatureAvailability,
    pub docker_process_count: u32,
    pub docker_process_names: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDevServerSnapshot {
    pub port: u16,
    pub local_url: Option<String>,
    pub lan_urls: Vec<String>,
    pub process_name: Option<String>,
    pub binding_scope: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorDiagnostics {
    pub collection_duration_ms: u64,
    pub process_count: u32,
    pub port_count: u32,
    pub adapter_count: u32,
    pub dropped_recording_samples: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSystemSample {
    pub cpu_total_percent: f64,
    pub memory: MemorySnapshot,
    pub battery: BatterySnapshot,
    pub network: AdapterThroughput,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSample {
    pub sequence: u64,
    pub collected_at_ms: u64,
    pub local_only: bool,
    pub included_internet_diagnostics: bool,
    pub system: RecordingSystemSample,
    pub gpu: GpuSnapshot,
    pub top_processes: Vec<RankedProcess>,
    pub network: NetworkDashboardSnapshot,
    pub docker: DockerActivitySnapshot,
    pub local_dev_servers: Vec<LocalDevServerSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAnnotation {
    pub id: String,
    pub at_ms: u64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordingStatus {
    Active,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationFinding {
    pub title: String,
    pub detail: String,
    pub evidence: Vec<String>,
    pub non_causal_disclaimer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSessionSummary {
    pub id: String,
    pub name: String,
    pub status: RecordingStatus,
    pub started_at_ms: u64,
    pub stopped_at_ms: Option<u64>,
    pub sample_count: u32,
    pub annotation_count: u32,
    pub downsampled: bool,
    pub local_only: bool,
    pub findings: Vec<CorrelationFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartMetricRecordingRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddMetricAnnotationRequest {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricRecordingDetail {
    pub summary: RecordingSessionSummary,
    pub samples: Vec<RecordingSample>,
    pub annotations: Vec<RecordingAnnotation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricRecordingExport {
    pub exported_at_ms: u64,
    pub redacted_by_default: bool,
    pub detail: MetricRecordingDetail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnosticsSnapshot {
    pub collected_at_ms: u64,
    pub refresh_mode: RefreshMode,
    pub system: SystemSnapshot,
    pub gpu: GpuSnapshot,
    pub ranked_processes: RankedProcesses,
    pub network: NetworkDashboardSnapshot,
    pub docker: DockerActivitySnapshot,
    pub local_dev_servers: Vec<LocalDevServerSnapshot>,
    pub collector_diagnostics: CollectorDiagnostics,
    pub active_recording: Option<RecordingSessionSummary>,
    pub recent_findings: Vec<CorrelationFinding>,
}
