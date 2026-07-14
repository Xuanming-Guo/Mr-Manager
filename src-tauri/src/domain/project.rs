use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: String,
    pub text: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStack {
    Node,
    Python,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectManifestKind {
    GitRepository,
    NodePackage,
    PythonProject,
    PythonRequirements,
    UvLock,
    PoetryLock,
    Pipfile,
    PipfileLock,
    RustCargo,
    CargoLock,
    DockerCompose,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub kind: ProjectManifestKind,
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageManagerSummary {
    pub name: String,
    pub evidence: Vec<String>,
    pub conflicting_lockfiles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScript {
    pub id: String,
    pub label: String,
    pub source: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitAvailability {
    Available,
    Unavailable,
    NotRepository,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSummary {
    pub availability: GitAvailability,
    pub branch: Option<String>,
    pub detached_head: bool,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub staged: u32,
    pub modified: u32,
    pub deleted: u32,
    pub renamed: u32,
    pub conflicted: u32,
    pub untracked: u32,
    pub last_commit: Option<String>,
    pub remotes: Vec<String>,
}

impl GitSummary {
    #[allow(dead_code)] // Later topology and association flows reuse this constructor directly.
    pub fn not_repository() -> Self {
        Self {
            availability: GitAvailability::NotRepository,
            branch: None,
            detached_head: false,
            ahead: None,
            behind: None,
            staged: 0,
            modified: 0,
            deleted: 0,
            renamed: 0,
            conflicted: 0,
            untracked: 0,
            last_commit: None,
            remotes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentFileSummary {
    pub relative_path: String,
    pub key_names: Vec<String>,
    pub example: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectIssueSeverity {
    Error,
    Warning,
    Information,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIssue {
    pub code: String,
    pub severity: ProjectIssueSeverity,
    pub message: String,
    pub remediation: Option<String>,
}

impl ProjectIssue {
    pub fn new(
        code: impl Into<String>,
        severity: ProjectIssueSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            remediation: None,
        }
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectScanState {
    Healthy,
    Warning,
    Error,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScanHealth {
    pub state: ProjectScanState,
    pub issues: Vec<ProjectIssue>,
}

impl ProjectScanHealth {
    pub fn from_issues(issues: Vec<ProjectIssue>) -> Self {
        let state = if issues
            .iter()
            .any(|issue| issue.severity == ProjectIssueSeverity::Error)
        {
            ProjectScanState::Error
        } else if issues.iter().any(|issue| {
            matches!(
                issue.severity,
                ProjectIssueSeverity::Warning | ProjectIssueSeverity::Unsupported
            )
        }) {
            ProjectScanState::Warning
        } else {
            ProjectScanState::Healthy
        };
        Self { state, issues }
    }

    #[allow(dead_code)] // Used by later long-running scan/probe states.
    pub fn unavailable(issue: ProjectIssue) -> Self {
        Self {
            state: ProjectScanState::Unavailable,
            issues: vec![issue],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub canonical_root_path: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub checklist: Vec<ChecklistItem>,
    pub pinned: bool,
    pub archived: bool,
    pub detected_stacks: Vec<ProjectStack>,
    pub manifests: Vec<ProjectManifest>,
    pub package_manager: Option<PackageManagerSummary>,
    pub scripts: Vec<ProjectScript>,
    pub git_summary: Option<GitSummary>,
    pub compose_files: Vec<String>,
    pub environment_files: Vec<EnvironmentFileSummary>,
    pub local_database_hints: Vec<String>,
    /// Unix epoch milliseconds. Kept numeric to match the snapshot contracts.
    pub last_scanned_at: Option<u64>,
    pub scan_health: ProjectScanHealth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDiscoveryResult {
    pub operation_id: String,
    pub projects: Vec<Project>,
    pub scanned_directories: u32,
    pub skipped_directories: u32,
    pub cancelled: bool,
    pub issues: Vec<ProjectIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectMetadataUpdate {
    pub tags: Vec<String>,
    pub notes: String,
    pub checklist: Vec<ChecklistItem>,
    pub pinned: bool,
    pub archived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ManagedCommandState {
    Starting,
    Running,
    Stopping,
    Exited,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ManagedCommandStream {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedCommandLogEntry {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub stream: ManagedCommandStream,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedCommand {
    pub run_id: String,
    pub project_id: String,
    pub script_id: String,
    pub label: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: String,
    pub pid: Option<u32>,
    pub started_at_ms: u64,
    pub ended_at_ms: Option<u64>,
    pub state: ManagedCommandState,
    pub exit_code: Option<i32>,
    pub stop_requested: bool,
    pub log_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_contract_uses_frontend_field_names() {
        let script = ProjectScript {
            id: "node:dev".to_owned(),
            label: "dev".to_owned(),
            source: "package.json".to_owned(),
            executable: "pnpm".to_owned(),
            arguments: vec!["run".to_owned(), "dev".to_owned()],
            working_directory: "C:\\fixture".to_owned(),
            verified: false,
        };
        let value = serde_json::to_value(script).expect("script should serialize");

        assert_eq!(value["workingDirectory"], "C:\\fixture");
        assert_eq!(value["arguments"][0], "run");
        assert!(value.get("working_directory").is_none());
    }
}
