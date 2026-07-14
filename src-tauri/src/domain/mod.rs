use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

mod cleaner;
mod docker;
mod integration;
mod network;
mod project;
mod system_diagnostics;
mod task;
mod topology;

pub use cleaner::*;
pub use docker::*;
pub use integration::*;
pub use network::*;
pub use project::*;
pub use system_diagnostics::*;
pub use task::*;
pub use topology::*;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AvailabilityState {
    Available,
    Unavailable,
    Unsupported,
    PermissionDenied,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureAvailability {
    pub state: AvailabilityState,
    pub reason: Option<String>,
    pub remediation: Option<String>,
}

impl FeatureAvailability {
    pub fn available(reason: impl Into<String>) -> Self {
        Self {
            state: AvailabilityState::Available,
            reason: Some(reason.into()),
            remediation: None,
        }
    }

    #[allow(dead_code)] // Used by detector states introduced in later milestones.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            state: AvailabilityState::Unavailable,
            reason: Some(reason.into()),
            remediation: None,
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            state: AvailabilityState::Unsupported,
            reason: Some(reason.into()),
            remediation: None,
        }
    }

    pub fn error(reason: impl Into<String>, remediation: impl Into<String>) -> Self {
        Self {
            state: AvailabilityState::Error,
            reason: Some(reason.into()),
            remediation: Some(remediation.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorIssue {
    pub code: String,
    pub message: String,
    pub remediation: Option<String>,
    pub permission_relevant: bool,
}

impl CollectorIssue {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            remediation: None,
            permission_relevant: false,
        }
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    pub fn permission_relevant(mut self) -> Self {
        self.permission_relevant = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSnapshot {
    pub total_usage_percent: f64,
    pub logical_core_count: u32,
    pub physical_core_count: Option<u32>,
    pub per_core_usage_percent: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskSnapshot {
    pub name: String,
    pub mount_point: String,
    pub kind: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub removable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkThroughputSnapshot {
    pub received_bytes_per_second: u64,
    pub transmitted_bytes_per_second: u64,
    pub total_received_bytes: u64,
    pub total_transmitted_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatterySnapshot {
    pub availability: FeatureAvailability,
    pub percentage: Option<u8>,
    pub ac_online: Option<bool>,
    pub remaining_seconds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    pub collected_at_ms: u64,
    pub sequence: u64,
    pub operating_system: String,
    pub operating_system_version: Option<String>,
    pub kernel_version: Option<String>,
    pub host_name: Option<String>,
    pub uptime_seconds: u64,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub network: NetworkThroughputSnapshot,
    pub battery: BatterySnapshot,
    pub gpu: FeatureAvailability,
    pub issues: Vec<CollectorIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessKey {
    pub pid: u32,
    pub start_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProtectedState {
    Unknown,
    Accessible,
    Protected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSnapshot {
    pub key: ProcessKey,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub executable_path: Option<String>,
    pub cwd: Option<String>,
    pub command_line_redacted: Option<String>,
    pub status: String,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub protected_state: ProtectedState,
    pub managed_by_mr_manager: bool,
    pub listening_port_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BindingScope {
    Loopback,
    AllInterfaces,
    SpecificInterface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReachabilityState {
    NotTested,
    LocalSelfTestOnly,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortEndpoint {
    pub protocol: PortProtocol,
    pub local_address: String,
    pub local_port: u16,
    pub state: String,
    pub owning_process_key: Option<ProcessKey>,
    pub owning_process_name: Option<String>,
    pub binding_scope: BindingScope,
    pub inferred_scheme: Option<String>,
    pub local_url: Option<String>,
    pub lan_urls: Vec<String>,
    pub reachability_state: ReachabilityState,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSummary {
    pub total: u32,
    pub accessible: u32,
    pub top_cpu: Vec<ProcessSnapshot>,
    pub top_memory: Vec<ProcessSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortSummary {
    pub total_listening: u32,
    pub development_listeners: u32,
    pub endpoints: Vec<PortEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSnapshot {
    pub system: SystemSnapshot,
    pub processes: ProcessSummary,
    pub ports: PortSummary,
    pub collector_issues: Vec<CollectorIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RefreshMode {
    Normal,
    Fast,
}

impl RefreshMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Fast => "fast",
        }
    }
}

impl fmt::Display for RefreshMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for RefreshMode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "normal" => Ok(Self::Normal),
            "fast" => Ok(Self::Fast),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub refresh_mode: RefreshMode,
    pub external_network_checks: bool,
    pub metric_history_enabled: bool,
    pub reduced_motion: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_mode: RefreshMode::Normal,
            external_network_checks: false,
            metric_history_enabled: false,
            reduced_motion: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityEntry {
    pub id: String,
    pub label: String,
    pub availability: FeatureAvailability,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityReport {
    pub platform: String,
    pub standard_user_mode: bool,
    pub features: Vec<CapabilityEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub remediation: Option<String>,
    pub technical_details: Option<String>,
    pub retryable: bool,
    pub permission_relevant: bool,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            remediation: None,
            technical_details: None,
            retryable: false,
            permission_relevant: false,
        }
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    pub fn with_safe_details(mut self, details: impl Into<String>) -> Self {
        self.technical_details = Some(details.into());
        self
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    #[allow(dead_code)] // Used by later permission-aware command boundaries.
    pub fn permission_relevant(mut self) -> Self {
        self.permission_relevant = true;
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_uses_camel_case_fields_and_values() {
        let value = serde_json::to_value(AppSettings {
            refresh_mode: RefreshMode::Fast,
            external_network_checks: false,
            metric_history_enabled: true,
            reduced_motion: false,
        })
        .expect("the DTO has a serializable shape");

        assert_eq!(value["refreshMode"], "fast");
        assert_eq!(value["externalNetworkChecks"], false);
        assert_eq!(value["metricHistoryEnabled"], true);
        assert!(value.get("refresh_mode").is_none());
    }

    #[test]
    fn error_contract_does_not_serialize_snake_case() {
        let value = serde_json::to_value(
            AppError::new("TEST", "safe message")
                .with_safe_details("safe details")
                .permission_relevant(),
        )
        .expect("the error has a serializable shape");

        assert_eq!(value["technicalDetails"], "safe details");
        assert_eq!(value["permissionRelevant"], true);
        assert!(value.get("technical_details").is_none());
    }

    #[test]
    fn process_identity_includes_start_time() {
        let first = ProcessKey {
            pid: 42,
            start_time: 100,
        };
        let reused = ProcessKey {
            pid: 42,
            start_time: 200,
        };

        assert_ne!(first, reused);
    }
}
