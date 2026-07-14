use serde::{Deserialize, Serialize};

use super::{
    AppError, CleanupScanResult, NetworkDiagnosticReport, ProjectDiscoveryResult,
    QuarantineManifest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackgroundTaskKind {
    CleanupScan,
    Quarantine,
    ProjectDiscovery,
    InternetDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackgroundTaskState {
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTask {
    pub id: String,
    pub kind: BackgroundTaskKind,
    pub label: String,
    pub route: String,
    pub state: BackgroundTaskState,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub cancellable: bool,
    pub progress_percent: Option<u8>,
    pub summary: Option<String>,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum BackgroundTaskOutput {
    CleanupScan(CleanupScanResult),
    Quarantine(QuarantineManifest),
    ProjectDiscovery(ProjectDiscoveryResult),
    InternetDiagnostics(Vec<NetworkDiagnosticReport>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTaskDetail {
    pub task: BackgroundTask,
    pub output: Option<BackgroundTaskOutput>,
}
