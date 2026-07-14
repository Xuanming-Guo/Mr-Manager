use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupCategory {
    DependencyCache,
    FrameworkCache,
    BuildOutput,
    PythonEnvironment,
    GeneratedLog,
    Archive,
    LargeFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupConfidence {
    Certain,
    Strong,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupRisk {
    Low,
    Review,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupLockState {
    Unknown,
    Available,
    InUse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCandidate {
    pub id: String,
    pub root_path: String,
    pub canonical_path: String,
    pub display_name: String,
    pub category: CleanupCategory,
    pub reason: String,
    pub confidence: CleanupConfidence,
    pub risk: CleanupRisk,
    pub estimated_size_bytes: u64,
    pub file_count: u64,
    pub lock_state: CleanupLockState,
    pub selected: bool,
    pub regeneration_instructions: String,
    pub identity_fingerprint: String,
    pub is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanIssue {
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanResult {
    pub scan_id: String,
    pub operation_id: String,
    pub roots: Vec<String>,
    pub candidates: Vec<CleanupCandidate>,
    pub issues: Vec<CleanupScanIssue>,
    pub visited_entries: u64,
    pub total_candidate_bytes: u64,
    pub cancelled: bool,
    pub limits_reached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CleanupScanRequest {
    pub operation_id: String,
    pub root_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCleanupPlanRequest {
    pub scan_id: String,
    pub candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupPlanState {
    Reviewed,
    Executed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlan {
    pub id: String,
    pub scan_id: String,
    pub created_at_ms: u64,
    pub roots: Vec<String>,
    pub items: Vec<CleanupCandidate>,
    pub total_size_bytes: u64,
    pub total_file_count: u64,
    pub state: CleanupPlanState,
    pub confirmation_phrase: String,
    pub manifest_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecuteCleanupPlanRequest {
    pub plan_id: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuarantineItemState {
    Pending,
    Quarantined,
    Partial,
    Failed,
    Restored,
    Purged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationState {
    Pending,
    AtomicMove,
    CopyVerified,
    RestoreVerified,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineItem {
    pub id: String,
    pub original_canonical_path: String,
    pub quarantine_path: String,
    pub restored_path: Option<String>,
    pub size_bytes: u64,
    pub file_count: u64,
    pub category: CleanupCategory,
    pub reason: String,
    pub project_association: Option<String>,
    pub state: QuarantineItemState,
    pub verification: VerificationState,
    pub purge_eligible: bool,
    pub error: Option<String>,
    pub quarantined_at_ms: Option<u64>,
    pub restored_at_ms: Option<u64>,
    pub purged_at_ms: Option<u64>,
    pub purge_confirmation_phrase: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuarantineManifestState {
    InProgress,
    Complete,
    Partial,
    Restored,
    Purged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineManifest {
    pub id: String,
    pub plan_id: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub state: QuarantineManifestState,
    pub items: Vec<QuarantineItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreConflictStrategy {
    Fail,
    SafeAlternative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreQuarantineItemRequest {
    pub manifest_id: String,
    pub item_id: String,
    pub conflict_strategy: RestoreConflictStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PurgeQuarantineItemRequest {
    pub manifest_id: String,
    pub item_id: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PurgeQuarantineManifestRequest {
    pub manifest_id: String,
    pub confirmation: String,
}
