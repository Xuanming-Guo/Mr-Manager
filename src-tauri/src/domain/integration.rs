use serde::{Deserialize, Serialize};

use super::{FeatureAvailability, PortEndpoint, ProcessKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationCategory {
    Runtime,
    PackageManager,
    Editor,
    Container,
    LocalAi,
    Database,
    Shell,
    Vpn,
    LocalService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationInstalledState {
    Installed,
    NotFound,
    Unknown,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationRunningState {
    Running,
    Stopped,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceConfidence {
    Certain,
    Strong,
    Inferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationEvidence {
    pub source: String,
    pub detail: String,
    pub confidence: EvidenceConfidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationEndpoint {
    pub label: String,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub local_only: bool,
    pub evidence: String,
}

impl From<&PortEndpoint> for IntegrationEndpoint {
    fn from(endpoint: &PortEndpoint) -> Self {
        Self {
            label: endpoint
                .owning_process_name
                .clone()
                .unwrap_or_else(|| "local listener".to_owned()),
            url: endpoint.local_url.clone(),
            port: Some(endpoint.local_port),
            local_only: endpoint.lan_urls.is_empty(),
            evidence: format!(
                "{} listener on {}:{}",
                format!("{:?}", endpoint.protocol).to_lowercase(),
                endpoint.local_address,
                endpoint.local_port
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationProcessRef {
    pub key: ProcessKey,
    pub name: String,
    pub executable_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationStatus {
    pub detector_id: String,
    pub display_name: String,
    pub category: IntegrationCategory,
    pub installed_state: IntegrationInstalledState,
    pub running_state: IntegrationRunningState,
    pub version: Option<String>,
    pub executable_paths: Vec<String>,
    pub processes: Vec<IntegrationProcessRef>,
    pub endpoints: Vec<IntegrationEndpoint>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<IntegrationEvidence>,
    pub last_checked_at_ms: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
    pub name: String,
    pub model: Option<String>,
    pub size_bytes: Option<u64>,
    pub digest: Option<String>,
    pub modified_at: Option<String>,
    pub format: Option<String>,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub loaded: bool,
    pub expires_at: Option<String>,
    pub size_vram_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub availability: FeatureAvailability,
    pub endpoint: Option<String>,
    pub version: Option<String>,
    pub installed_models: Vec<OllamaModel>,
    pub running_models: Vec<OllamaModel>,
    pub processes: Vec<IntegrationProcessRef>,
    pub evidence: Vec<IntegrationEvidence>,
    pub last_checked_at_ms: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslDistribution {
    pub name: String,
    pub state: String,
    pub version: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslStatus {
    pub availability: FeatureAvailability,
    pub distros: Vec<WslDistribution>,
    pub evidence: Vec<IntegrationEvidence>,
    pub last_checked_at_ms: u64,
    pub errors: Vec<String>,
}
