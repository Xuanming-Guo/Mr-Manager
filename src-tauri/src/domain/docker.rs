use serde::{Deserialize, Serialize};

use super::PortProtocol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerAvailability {
    CliMissing,
    InstalledStopped,
    Starting,
    Inaccessible,
    Running,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerDesktopProcessState {
    Running,
    NotDetected,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerIssueSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerDiagnostic {
    pub code: String,
    pub severity: DockerIssueSeverity,
    pub message: String,
    pub remediation: Option<String>,
    pub evidence: Vec<String>,
}

impl DockerDiagnostic {
    pub fn new(
        code: impl Into<String>,
        severity: DockerIssueSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            remediation: None,
            evidence: Vec::new(),
        }
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub availability: DockerAvailability,
    pub cli_detected: bool,
    pub daemon_reachable: bool,
    pub client_version: Option<String>,
    pub server_version: Option<String>,
    pub context: Option<String>,
    pub docker_desktop_process: DockerDesktopProcessState,
    pub collected_at_ms: u64,
    pub diagnostics: Vec<DockerDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerLabel {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerPortMapping {
    pub host_ip: Option<String>,
    pub host_port: Option<u16>,
    pub container_port: u16,
    pub protocol: PortProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerMount {
    pub kind: String,
    pub source: Option<String>,
    pub destination: String,
    pub mode: Option<String>,
    pub read_write: Option<bool>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerResourceUsage {
    pub cpu_percent: Option<String>,
    pub memory_usage: Option<String>,
    pub memory_percent: Option<String>,
    pub network_io: Option<String>,
    pub block_io: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerProjectAssociation {
    pub project_id: String,
    pub project_name: String,
    pub project_root: String,
    pub confidence: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerContainer {
    pub id: String,
    pub short_id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub health: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub ports: Vec<DockerPortMapping>,
    pub networks: Vec<String>,
    pub mounts: Vec<DockerMount>,
    pub labels: Vec<DockerLabel>,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
    pub compose_working_dir: Option<String>,
    pub user: Option<String>,
    pub restart_policy: Option<String>,
    pub resource_usage: Option<DockerResourceUsage>,
    pub associated_project: Option<DockerProjectAssociation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerContainerActionKind {
    Start,
    Stop,
    Restart,
}

impl DockerContainerActionKind {
    pub const fn as_command(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DockerContainerActionRequest {
    pub container_id: String,
    pub action: DockerContainerActionKind,
    pub confirmation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerContainerActionResult {
    pub action: DockerContainerActionKind,
    pub container: DockerContainer,
    pub stdout: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerLogEntry {
    pub sequence: u64,
    pub timestamp: Option<String>,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerNetwork {
    pub name: String,
    pub id: Option<String>,
    pub driver: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerVolume {
    pub name: String,
    pub driver: Option<String>,
    pub scope: Option<String>,
    pub mountpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerInventory {
    pub status: DockerStatus,
    pub containers: Vec<DockerContainer>,
    pub networks: Vec<DockerNetwork>,
    pub volumes: Vec<DockerVolume>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComposeParseSource {
    DockerComposeConfig,
    FallbackParser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposePortMapping {
    pub host_ip: Option<String>,
    pub host_port: Option<u16>,
    pub container_port: u16,
    pub protocol: PortProtocol,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeVolumeMount {
    pub source: Option<String>,
    pub target: Option<String>,
    pub mode: Option<String>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeService {
    pub name: String,
    pub image: Option<String>,
    pub build: Option<String>,
    pub container_name: Option<String>,
    pub command: Option<String>,
    pub user: Option<String>,
    pub restart: Option<String>,
    pub ports: Vec<ComposePortMapping>,
    pub volumes: Vec<ComposeVolumeMount>,
    pub environment_keys: Vec<String>,
    pub depends_on: Vec<String>,
    pub networks: Vec<String>,
    pub profiles: Vec<String>,
    pub healthcheck_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeNetwork {
    pub name: String,
    pub external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeVolume {
    pub name: String,
    pub external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProject {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub project_root: String,
    pub compose_file: String,
    pub source: ComposeParseSource,
    pub services: Vec<ComposeService>,
    pub networks: Vec<ComposeNetwork>,
    pub volumes: Vec<ComposeVolume>,
    pub unresolved_interpolation: Vec<String>,
    pub parse_diagnostics: Vec<DockerDiagnostic>,
    pub doctor: Vec<ComposeDoctorIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeDoctorIssue {
    pub code: String,
    pub severity: DockerIssueSeverity,
    pub service: Option<String>,
    pub message: String,
    pub remediation: Option<String>,
    pub evidence: Vec<String>,
}

impl ComposeDoctorIssue {
    pub fn new(
        code: impl Into<String>,
        severity: DockerIssueSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            service: None,
            message: message.into(),
            remediation: None,
            evidence: Vec::new(),
        }
    }

    pub fn for_service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }
}
