use std::collections::{HashMap, HashSet, VecDeque, hash_map::DefaultHasher};
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use sysinfo::Disks;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    CleanupCandidate, CleanupCategory, CleanupConfidence, CleanupLockState, CleanupPlan,
    CleanupPlanState, CleanupRisk, CleanupScanIssue, CleanupScanRequest, CleanupScanResult,
    CreateCleanupPlanRequest, QuarantineItem, QuarantineItemState, QuarantineManifest,
    QuarantineManifestState, RestoreConflictStrategy, VerificationState,
};

const MAX_ROOTS: usize = 8;
const MAX_DEPTH: usize = 24;
const MAX_VISITED_ENTRIES: u64 = 100_000;
const MAX_CANDIDATES: usize = 5_000;
const MAX_CACHED_SCANS: usize = 8;
const MAX_PATH_LENGTH: usize = 32_767;
const GENERATED_LOG_MIN_BYTES: u64 = 1_048_576;
const ARCHIVE_MIN_BYTES: u64 = 52_428_800;
const LARGE_FILE_MIN_BYTES: u64 = 262_144_000;

#[derive(Debug, Error)]
pub enum CleanerError {
    #[error("the cleanup request is invalid: {0}")]
    InvalidRequest(String),
    #[error("the selected cleanup root is unsafe: {0}")]
    UnsafeRoot(String),
    #[error("the cleanup scan was not found or has expired")]
    ScanNotFound,
    #[error("the cleanup plan was not found")]
    PlanNotFound,
    #[error("the quarantine manifest or item was not found")]
    ManifestNotFound,
    #[error("the cleanup confirmation did not match; expected: {0}")]
    ConfirmationMismatch(String),
    #[error("the reviewed cleanup plan has already been executed")]
    PlanAlreadyExecuted,
    #[error("the cleanup item changed after review: {0}")]
    ItemChanged(String),
    #[error("a verified quarantine copy exists, but the original could not be removed: {0}")]
    PartialFailure(String),
    #[error("copy fallback was refused: {0}")]
    CopyFallbackRefused(String),
    #[error("the original restore destination is occupied; safe alternative: {0}")]
    RestoreConflict(String),
    #[error("the cleanup operation could not access the filesystem while {action}")]
    Io {
        action: &'static str,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeIdentity {
    size_bytes: u64,
    file_count: u64,
    entry_count: u64,
    fingerprint: String,
    is_directory: bool,
}

#[derive(Debug)]
pub struct CleanerManager {
    quarantine_root: PathBuf,
    active_scans: HashMap<String, Arc<AtomicBool>>,
    cached_scans: VecDeque<CleanupScanResult>,
}

pub type CleanerRegistry = Arc<Mutex<CleanerManager>>;

impl CleanerManager {
    pub fn new(quarantine_root: PathBuf) -> Self {
        Self {
            quarantine_root,
            active_scans: HashMap::new(),
            cached_scans: VecDeque::new(),
        }
    }

    pub fn quarantine_root(&self) -> PathBuf {
        self.quarantine_root.clone()
    }

    pub fn begin_scan(&mut self, operation_id: &str) -> Result<Arc<AtomicBool>, CleanerError> {
        validate_identifier(operation_id, "operation id")?;
        if self.active_scans.contains_key(operation_id) {
            return Err(CleanerError::InvalidRequest(
                "a cleanup scan with this operation id is already active".to_owned(),
            ));
        }
        let cancellation = Arc::new(AtomicBool::new(false));
        self.active_scans
            .insert(operation_id.to_owned(), Arc::clone(&cancellation));
        Ok(cancellation)
    }

    pub fn finish_scan(&mut self, result: CleanupScanResult) {
        self.active_scans.remove(&result.operation_id);
        while self.cached_scans.len() >= MAX_CACHED_SCANS {
            self.cached_scans.pop_front();
        }
        self.cached_scans.push_back(result);
    }

    pub fn abandon_scan(&mut self, operation_id: &str) {
        self.active_scans.remove(operation_id);
    }

    pub fn cancel_scan(&self, operation_id: &str) -> bool {
        self.active_scans.get(operation_id).is_some_and(|token| {
            token.store(true, Ordering::Release);
            true
        })
    }

    pub fn create_plan(
        &self,
        request: &CreateCleanupPlanRequest,
    ) -> Result<CleanupPlan, CleanerError> {
        validate_identifier(&request.scan_id, "scan id")?;
        if request.candidate_ids.is_empty() {
            return Err(CleanerError::InvalidRequest(
                "select at least one reviewed candidate".to_owned(),
            ));
        }
        let scan = self
            .cached_scans
            .iter()
            .find(|scan| scan.scan_id == request.scan_id)
            .ok_or(CleanerError::ScanNotFound)?;
        if scan.cancelled || scan.limits_reached {
            return Err(CleanerError::InvalidRequest(
                "a cancelled or limit-truncated scan cannot authorize cleanup".to_owned(),
            ));
        }
        let ids = request
            .candidate_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        if ids.len() != request.candidate_ids.len() {
            return Err(CleanerError::InvalidRequest(
                "candidate ids must be unique".to_owned(),
            ));
        }
        let mut items = scan
            .candidates
            .iter()
            .filter(|candidate| ids.contains(&candidate.id))
            .cloned()
            .collect::<Vec<_>>();
        if items.len() != ids.len() {
            return Err(CleanerError::InvalidRequest(
                "one or more candidates are not part of the server-side scan".to_owned(),
            ));
        }
        items.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
        let id = Uuid::new_v4().to_string();
        Ok(CleanupPlan {
            id,
            scan_id: scan.scan_id.clone(),
            created_at_ms: now_ms(),
            roots: scan.roots.clone(),
            total_size_bytes: items.iter().fold(0_u64, |total, item| {
                total.saturating_add(item.estimated_size_bytes)
            }),
            total_file_count: items
                .iter()
                .fold(0_u64, |total, item| total.saturating_add(item.file_count)),
            confirmation_phrase: format!("QUARANTINE {} ITEMS", items.len()),
            items,
            state: CleanupPlanState::Reviewed,
            manifest_id: None,
        })
    }
}

pub fn scan_cleanup_candidates(
    request: CleanupScanRequest,
    cancellation: Arc<AtomicBool>,
    quarantine_root: &Path,
) -> Result<CleanupScanResult, CleanerError> {
    if request.root_paths.is_empty() || request.root_paths.len() > MAX_ROOTS {
        return Err(CleanerError::InvalidRequest(format!(
            "choose between 1 and {MAX_ROOTS} explicit roots"
        )));
    }

    let quarantine_root = canonical_or_absolute(quarantine_root)?;
    let mut roots = Vec::with_capacity(request.root_paths.len());
    for selected in &request.root_paths {
        let canonical = validate_selected_root(Path::new(selected), &quarantine_root)?;
        if roots.iter().any(|existing: &PathBuf| {
            canonical.starts_with(existing) || existing.starts_with(&canonical)
        }) {
            return Err(CleanerError::InvalidRequest(
                "selected cleanup roots must not overlap".to_owned(),
            ));
        }
        roots.push(canonical);
    }

    let mut result = CleanupScanResult {
        scan_id: Uuid::new_v4().to_string(),
        operation_id: request.operation_id,
        roots: roots.iter().map(|path| path_text(path)).collect(),
        candidates: Vec::new(),
        issues: Vec::new(),
        visited_entries: 0,
        total_candidate_bytes: 0,
        cancelled: false,
        limits_reached: false,
    };
    let mut stack = roots
        .iter()
        .cloned()
        .map(|root| (root.clone(), root, 0_usize))
        .collect::<Vec<_>>();

    while let Some((path, root, depth)) = stack.pop() {
        if cancellation.load(Ordering::Acquire) {
            result.cancelled = true;
            break;
        }
        if result.visited_entries >= MAX_VISITED_ENTRIES
            || result.candidates.len() >= MAX_CANDIDATES
        {
            result.limits_reached = true;
            break;
        }
        if depth > MAX_DEPTH {
            result.limits_reached = true;
            result.issues.push(issue(
                &path,
                "MAX_DEPTH_REACHED",
                "This subtree exceeded the bounded scan depth and was skipped.",
            ));
            continue;
        }
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(source) => {
                result.issues.push(issue(
                    &path,
                    "DIRECTORY_UNREADABLE",
                    &format!("The folder could not be read: {source}"),
                ));
                continue;
            }
        };
        for entry in entries {
            if cancellation.load(Ordering::Acquire) {
                result.cancelled = true;
                break;
            }
            result.visited_entries = result.visited_entries.saturating_add(1);
            if result.visited_entries > MAX_VISITED_ENTRIES {
                result.limits_reached = true;
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(source) => {
                    result.issues.push(CleanupScanIssue {
                        path: path_text(&path),
                        code: "ENTRY_UNREADABLE".to_owned(),
                        message: format!("A directory entry could not be read: {source}"),
                    });
                    continue;
                }
            };
            let child = entry.path();
            if path_text(&child).len() > MAX_PATH_LENGTH {
                result.issues.push(issue(
                    &child,
                    "PATH_TOO_LONG",
                    "The path exceeded the scan limit.",
                ));
                continue;
            }
            let metadata = match fs::symlink_metadata(&child) {
                Ok(metadata) => metadata,
                Err(source) => {
                    result.issues.push(issue(
                        &child,
                        "METADATA_UNAVAILABLE",
                        &format!("Metadata could not be read: {source}"),
                    ));
                    continue;
                }
            };
            if is_link_or_reparse(&metadata) {
                result.issues.push(issue(
                    &child,
                    "REPARSE_POINT_SKIPPED",
                    "Symlinks, junctions, mount points, and other reparse points are never followed.",
                ));
                continue;
            }
            let canonical = match child.canonicalize() {
                Ok(canonical) if canonical.starts_with(&root) => canonical,
                Ok(_) => {
                    result.issues.push(issue(
                        &child,
                        "ROOT_BOUNDARY_ESCAPE",
                        "The path resolved outside the selected root and was skipped.",
                    ));
                    continue;
                }
                Err(source) => {
                    result.issues.push(issue(
                        &child,
                        "CANONICALIZATION_FAILED",
                        &format!("The path could not be canonicalized: {source}"),
                    ));
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip_subtree(&name) {
                continue;
            }
            if let Some(rule) = classify(&canonical, &name, &metadata) {
                let remaining_entries = MAX_VISITED_ENTRIES.saturating_sub(result.visited_entries);
                match tree_identity(&canonical, &cancellation, remaining_entries) {
                    Ok(identity) => {
                        result.visited_entries =
                            result.visited_entries.saturating_add(identity.entry_count);
                        let candidate = CleanupCandidate {
                            id: Uuid::new_v4().to_string(),
                            root_path: path_text(&root),
                            canonical_path: path_text(&canonical),
                            display_name: name,
                            category: rule.category,
                            reason: rule.reason.to_owned(),
                            confidence: rule.confidence,
                            risk: rule.risk,
                            estimated_size_bytes: identity.size_bytes,
                            file_count: identity.file_count,
                            lock_state: practical_lock_state(&canonical, identity.is_directory),
                            selected: false,
                            regeneration_instructions: rule.regeneration.to_owned(),
                            identity_fingerprint: identity.fingerprint,
                            is_directory: identity.is_directory,
                        };
                        result.total_candidate_bytes = result
                            .total_candidate_bytes
                            .saturating_add(candidate.estimated_size_bytes);
                        result.candidates.push(candidate);
                    }
                    Err(CleanerError::InvalidRequest(message)) if message == "scan cancelled" => {
                        result.cancelled = true;
                        break;
                    }
                    Err(CleanerError::InvalidRequest(message))
                        if message.starts_with("candidate exceeded") =>
                    {
                        result.limits_reached = true;
                        result
                            .issues
                            .push(issue(&canonical, "CANDIDATE_LIMIT_REACHED", &message));
                        break;
                    }
                    Err(error) => result.issues.push(issue(
                        &canonical,
                        "CANDIDATE_INSPECTION_FAILED",
                        &error.to_string(),
                    )),
                }
                continue;
            }
            if metadata.is_dir() {
                stack.push((canonical, root.clone(), depth + 1));
            }
        }
        if result.cancelled || result.limits_reached {
            break;
        }
    }
    result
        .candidates
        .sort_by(|left, right| right.estimated_size_bytes.cmp(&left.estimated_size_bytes));
    Ok(result)
}

pub fn begin_manifest(plan: &CleanupPlan, quarantine_root: &Path) -> QuarantineManifest {
    let id = Uuid::new_v4().to_string();
    let created_at_ms = now_ms();
    let items = plan
        .items
        .iter()
        .map(|candidate| {
            let item_id = Uuid::new_v4().to_string();
            let quarantine_path = quarantine_root.join(&id).join(&item_id);
            QuarantineItem {
                purge_confirmation_phrase: format!("PURGE {item_id}"),
                id: item_id,
                original_canonical_path: candidate.canonical_path.clone(),
                quarantine_path: path_text(&quarantine_path),
                restored_path: None,
                size_bytes: candidate.estimated_size_bytes,
                file_count: candidate.file_count,
                category: candidate.category,
                reason: candidate.reason.clone(),
                project_association: None,
                state: QuarantineItemState::Pending,
                verification: VerificationState::Pending,
                purge_eligible: false,
                error: None,
                quarantined_at_ms: None,
                restored_at_ms: None,
                purged_at_ms: None,
            }
        })
        .collect();
    QuarantineManifest {
        id,
        plan_id: plan.id.clone(),
        created_at_ms,
        updated_at_ms: created_at_ms,
        state: QuarantineManifestState::InProgress,
        items,
    }
}

pub fn quarantine_item(
    candidate: &CleanupCandidate,
    item: &mut QuarantineItem,
    quarantine_root: &Path,
) -> Result<(), CleanerError> {
    let source = PathBuf::from(&candidate.canonical_path);
    let canonical = source.canonicalize().map_err(|source| CleanerError::Io {
        action: "revalidating a reviewed cleanup item",
        source,
    })?;
    let root = PathBuf::from(&candidate.root_path)
        .canonicalize()
        .map_err(|source| CleanerError::Io {
            action: "revalidating the selected cleanup root",
            source,
        })?;
    if canonical != source || !canonical.starts_with(&root) {
        return Err(CleanerError::ItemChanged(candidate.canonical_path.clone()));
    }
    ensure_no_reparse_ancestors(&canonical, &root)?;
    let current = tree_identity(
        &canonical,
        &Arc::new(AtomicBool::new(false)),
        MAX_VISITED_ENTRIES,
    )?;
    if current.size_bytes != candidate.estimated_size_bytes
        || current.file_count != candidate.file_count
        || current.fingerprint != candidate.identity_fingerprint
        || current.is_directory != candidate.is_directory
    {
        return Err(CleanerError::ItemChanged(candidate.canonical_path.clone()));
    }

    let destination = PathBuf::from(&item.quarantine_path);
    ensure_new_destination_below(&destination, quarantine_root)?;
    let verification = move_verified(&canonical, &destination, &current)?;
    item.state = QuarantineItemState::Quarantined;
    item.verification = verification;
    item.purge_eligible = true;
    item.quarantined_at_ms = Some(now_ms());
    item.error = None;
    Ok(())
}

pub fn restore_item(
    item: &mut QuarantineItem,
    quarantine_root: &Path,
    conflict_strategy: RestoreConflictStrategy,
) -> Result<(), CleanerError> {
    if item.state != QuarantineItemState::Quarantined {
        return Err(CleanerError::InvalidRequest(
            "only quarantined items can be restored".to_owned(),
        ));
    }
    let source =
        validate_existing_quarantine_path(Path::new(&item.quarantine_path), quarantine_root)?;
    let original = PathBuf::from(&item.original_canonical_path);
    let destination = if original.exists() {
        let alternative = safe_restore_alternative(&original, &item.id);
        match conflict_strategy {
            RestoreConflictStrategy::Fail => {
                return Err(CleanerError::RestoreConflict(path_text(&alternative)));
            }
            RestoreConflictStrategy::SafeAlternative => alternative,
        }
    } else {
        original
    };
    if destination.exists() {
        return Err(CleanerError::RestoreConflict(path_text(&destination)));
    }
    let current = tree_identity(
        &source,
        &Arc::new(AtomicBool::new(false)),
        MAX_VISITED_ENTRIES,
    )?;
    if current.size_bytes != item.size_bytes || current.file_count != item.file_count {
        return Err(CleanerError::ItemChanged(item.quarantine_path.clone()));
    }
    move_verified(&source, &destination, &current)?;
    item.state = QuarantineItemState::Restored;
    item.verification = VerificationState::RestoreVerified;
    item.purge_eligible = false;
    item.restored_path = Some(path_text(&destination));
    item.restored_at_ms = Some(now_ms());
    item.error = None;
    Ok(())
}

pub fn purge_item(
    item: &mut QuarantineItem,
    quarantine_root: &Path,
    confirmation: &str,
) -> Result<(), CleanerError> {
    if !matches!(
        item.state,
        QuarantineItemState::Quarantined | QuarantineItemState::Partial
    ) || !item.purge_eligible
    {
        return Err(CleanerError::InvalidRequest(
            "only currently quarantined items can be purged".to_owned(),
        ));
    }
    if confirmation != item.purge_confirmation_phrase {
        return Err(CleanerError::ConfirmationMismatch(
            item.purge_confirmation_phrase.clone(),
        ));
    }
    let path =
        validate_existing_quarantine_path(Path::new(&item.quarantine_path), quarantine_root)?;
    remove_node(&path)?;
    item.state = QuarantineItemState::Purged;
    item.verification = VerificationState::CopyVerified;
    item.purge_eligible = false;
    item.purged_at_ms = Some(now_ms());
    item.error = None;
    Ok(())
}

pub fn refresh_manifest_state(manifest: &mut QuarantineManifest) {
    manifest.updated_at_ms = now_ms();
    if manifest
        .items
        .iter()
        .all(|item| item.state == QuarantineItemState::Purged)
    {
        manifest.state = QuarantineManifestState::Purged;
    } else if manifest.items.iter().all(|item| {
        matches!(
            item.state,
            QuarantineItemState::Restored | QuarantineItemState::Purged
        )
    }) {
        manifest.state = QuarantineManifestState::Restored;
    } else if manifest.items.iter().any(|item| {
        matches!(
            item.state,
            QuarantineItemState::Failed | QuarantineItemState::Partial
        )
    }) {
        manifest.state = QuarantineManifestState::Partial;
    } else if manifest
        .items
        .iter()
        .all(|item| item.state == QuarantineItemState::Quarantined)
    {
        manifest.state = QuarantineManifestState::Complete;
    } else {
        manifest.state = QuarantineManifestState::InProgress;
    }
}

struct Rule {
    category: CleanupCategory,
    reason: &'static str,
    confidence: CleanupConfidence,
    risk: CleanupRisk,
    regeneration: &'static str,
}

fn classify(path: &Path, name: &str, metadata: &fs::Metadata) -> Option<Rule> {
    let lower = name.to_ascii_lowercase();
    if metadata.is_dir() {
        let rule = match lower.as_str() {
            "node_modules" => Rule {
                category: CleanupCategory::DependencyCache,
                reason: "Installed Node dependencies are normally reproducible from the project manifest and lockfile.",
                confidence: CleanupConfidence::Certain,
                risk: CleanupRisk::Low,
                regeneration: "Run the project's locked package-manager install command.",
            },
            ".next" | ".nuxt" | ".svelte-kit" | ".vite" | ".turbo" | ".parcel-cache"
            | "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".tox" | ".nox" => {
                Rule {
                    category: CleanupCategory::FrameworkCache,
                    reason: "This recognized framework or tool cache is regenerated by normal development commands.",
                    confidence: CleanupConfidence::Certain,
                    risk: CleanupRisk::Low,
                    regeneration: "Run the corresponding build, test, or development command again.",
                }
            }
            "target"
                if path
                    .parent()
                    .is_some_and(|parent| parent.join("Cargo.toml").is_file()) =>
            {
                Rule {
                    category: CleanupCategory::BuildOutput,
                    reason: "This Rust target directory is generated by Cargo for the adjacent Cargo.toml.",
                    confidence: CleanupConfidence::Certain,
                    risk: CleanupRisk::Low,
                    regeneration: "Run cargo build, cargo test, or the project's normal Cargo command.",
                }
            }
            "dist" | "build" | "out" => Rule {
                category: CleanupCategory::BuildOutput,
                reason: "This conventional output directory is often generated, but may contain manually copied release artifacts.",
                confidence: CleanupConfidence::Inferred,
                risk: CleanupRisk::Review,
                regeneration: "Confirm the project's build command and review the contents before quarantine.",
            },
            ".venv" | "venv" => Rule {
                category: CleanupCategory::PythonEnvironment,
                reason: "This appears to be a Python virtual environment and may be reproducible from project dependency metadata.",
                confidence: CleanupConfidence::Strong,
                risk: CleanupRisk::Review,
                regeneration: "Recreate the environment with the project's documented Python or uv command.",
            },
            _ => return None,
        };
        return Some(rule);
    }
    if !metadata.is_file() || is_never_candidate_file(&lower) {
        return None;
    }
    let size = metadata.len();
    if lower.ends_with(".log") && size >= GENERATED_LOG_MIN_BYTES {
        return Some(Rule {
            category: CleanupCategory::GeneratedLog,
            reason: "This generated log is at least 1 MiB; review it before quarantine.",
            confidence: CleanupConfidence::Strong,
            risk: CleanupRisk::Review,
            regeneration: "Logs are regenerated by the originating tool; preserve any needed diagnostics first.",
        });
    }
    if is_archive(&lower) && size >= ARCHIVE_MIN_BYTES {
        return Some(Rule {
            category: CleanupCategory::Archive,
            reason: "This archive is at least 50 MiB and may be a downloaded or generated artifact.",
            confidence: CleanupConfidence::Inferred,
            risk: CleanupRisk::High,
            regeneration: "Confirm the archive has another trusted source before quarantine.",
        });
    }
    (size >= LARGE_FILE_MIN_BYTES).then_some(Rule {
        category: CleanupCategory::LargeFile,
        reason: "This file is at least 250 MiB; size alone does not prove it is regenerable.",
        confidence: CleanupConfidence::Inferred,
        risk: CleanupRisk::High,
        regeneration: "No regeneration path is known. Verify the file manually before quarantine.",
    })
}

fn should_skip_subtree(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == ".git"
        || lower.starts_with(".env")
        || matches!(
            lower.as_str(),
            ".ssh" | ".gnupg" | "credentials" | "secrets"
        )
}

fn is_never_candidate_file(lower: &str) -> bool {
    lower.starts_with(".env")
        || matches!(
            Path::new(lower)
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("db" | "sqlite" | "sqlite3" | "pem" | "key" | "pfx" | "p12")
        )
}

fn is_archive(lower: &str) -> bool {
    [".zip", ".7z", ".rar", ".tar", ".tgz", ".gz", ".bz2", ".xz"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

fn validate_selected_root(path: &Path, quarantine_root: &Path) -> Result<PathBuf, CleanerError> {
    if path.as_os_str().is_empty() {
        return Err(CleanerError::UnsafeRoot("the path is empty".to_owned()));
    }
    let metadata = fs::symlink_metadata(path).map_err(|source| CleanerError::Io {
        action: "reading selected-root metadata",
        source,
    })?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err(CleanerError::UnsafeRoot(
            "the selected root must be a real directory, not a link or reparse point".to_owned(),
        ));
    }
    let canonical = path.canonicalize().map_err(|source| CleanerError::Io {
        action: "canonicalizing the selected cleanup root",
        source,
    })?;
    if is_dangerous_root(&canonical) {
        return Err(CleanerError::UnsafeRoot(path_text(&canonical)));
    }
    if canonical.starts_with(quarantine_root) || quarantine_root.starts_with(&canonical) {
        return Err(CleanerError::UnsafeRoot(
            "Mr Manager quarantine storage cannot be scanned".to_owned(),
        ));
    }
    Ok(canonical)
}

fn is_dangerous_root(path: &Path) -> bool {
    if path.parent().is_none()
        || path.components().all(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::CurDir
            )
        })
    {
        return true;
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))
        && canonical_eq(path, Path::new(&home))
    {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        let protected_trees = [
            std::env::var_os("SystemRoot"),
            std::env::var_os("ProgramFiles"),
            std::env::var_os("ProgramFiles(x86)"),
            std::env::var_os("ProgramData"),
        ];
        if protected_trees
            .into_iter()
            .flatten()
            .filter_map(|value| PathBuf::from(value).canonicalize().ok())
            .any(|dangerous| path.starts_with(dangerous))
        {
            return true;
        }
        let app_data_roots = [
            std::env::var_os("APPDATA"),
            std::env::var_os("LOCALAPPDATA"),
        ];
        if app_data_roots
            .into_iter()
            .flatten()
            .filter_map(|value| PathBuf::from(value).canonicalize().ok())
            .any(|dangerous| path == dangerous)
        {
            return true;
        }
    }
    false
}

fn canonical_eq(left: &Path, right: &Path) -> bool {
    right.canonicalize().is_ok_and(|right| right == left)
}

fn ensure_no_reparse_ancestors(path: &Path, root: &Path) -> Result<(), CleanerError> {
    if !path.starts_with(root) {
        return Err(CleanerError::UnsafeRoot(path_text(path)));
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|_| CleanerError::UnsafeRoot(path_text(path)))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|source| CleanerError::Io {
            action: "revalidating cleanup path components",
            source,
        })?;
        if is_link_or_reparse(&metadata) {
            return Err(CleanerError::ItemChanged(path_text(&current)));
        }
    }
    Ok(())
}

fn tree_identity(
    root: &Path,
    cancellation: &Arc<AtomicBool>,
    maximum_entries: u64,
) -> Result<TreeIdentity, CleanerError> {
    let root_metadata = fs::symlink_metadata(root).map_err(|source| CleanerError::Io {
        action: "reading cleanup item metadata",
        source,
    })?;
    if is_link_or_reparse(&root_metadata) {
        return Err(CleanerError::ItemChanged(path_text(root)));
    }
    let is_directory = root_metadata.is_dir();
    let mut stack = vec![(root.to_path_buf(), 0_usize)];
    let mut size_bytes = 0_u64;
    let mut file_count = 0_u64;
    let mut entry_count = 0_u64;
    let mut entries = Vec::new();
    while let Some((path, depth)) = stack.pop() {
        if cancellation.load(Ordering::Acquire) {
            return Err(CleanerError::InvalidRequest("scan cancelled".to_owned()));
        }
        entry_count = entry_count.saturating_add(1);
        if entry_count > maximum_entries || depth > MAX_DEPTH {
            return Err(CleanerError::InvalidRequest(
                "candidate exceeded the bounded entry-count or depth limit".to_owned(),
            ));
        }
        let metadata = fs::symlink_metadata(&path).map_err(|source| CleanerError::Io {
            action: "inspecting a cleanup candidate",
            source,
        })?;
        if is_link_or_reparse(&metadata) {
            return Err(CleanerError::ItemChanged(path_text(&path)));
        }
        let relative = path.strip_prefix(root).unwrap_or(Path::new(""));
        if metadata.is_dir() {
            entries.push((path_text(relative), true, 0_u64, 0_u128));
            let children = fs::read_dir(&path).map_err(|source| CleanerError::Io {
                action: "enumerating a cleanup candidate",
                source,
            })?;
            for child in children {
                let child = child.map_err(|source| CleanerError::Io {
                    action: "reading a cleanup candidate entry",
                    source,
                })?;
                stack.push((child.path(), depth + 1));
            }
        } else if metadata.is_file() {
            file_count = file_count.saturating_add(1);
            size_bytes = size_bytes.saturating_add(metadata.len());
            entries.push((
                path_text(relative),
                false,
                metadata.len(),
                modified_ms(&metadata),
            ));
        } else {
            return Err(CleanerError::ItemChanged(path_text(&path)));
        }
    }
    entries.sort();
    let mut hasher = DefaultHasher::new();
    entries.hash(&mut hasher);
    Ok(TreeIdentity {
        size_bytes,
        file_count,
        entry_count,
        fingerprint: format!("{:016x}", hasher.finish()),
        is_directory,
    })
}

fn move_verified(
    source: &Path,
    destination: &Path,
    expected: &TreeIdentity,
) -> Result<VerificationState, CleanerError> {
    if destination.exists() {
        return Err(CleanerError::InvalidRequest(
            "the managed destination is already occupied".to_owned(),
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| CleanerError::Io {
            action: "creating managed quarantine storage",
            source,
        })?;
    }
    let rename_error = match fs::rename(source, destination) {
        Ok(()) => {
            let actual = tree_identity(
                destination,
                &Arc::new(AtomicBool::new(false)),
                MAX_VISITED_ENTRIES,
            )?;
            if actual != *expected {
                let _ = fs::rename(destination, source);
                return Err(CleanerError::ItemChanged(path_text(destination)));
            }
            return Ok(VerificationState::AtomicMove);
        }
        Err(error) => error,
    };

    ensure_cross_volume_copy_capacity(source, destination, expected.size_bytes, &rename_error)?;

    copy_node(source, destination)?;
    let actual = tree_identity(
        destination,
        &Arc::new(AtomicBool::new(false)),
        MAX_VISITED_ENTRIES,
    )?;
    if actual != *expected {
        let _ = remove_node(destination);
        return Err(CleanerError::ItemChanged(path_text(destination)));
    }
    if let Err(error) = remove_node(source) {
        return Err(CleanerError::PartialFailure(error.to_string()));
    }
    Ok(VerificationState::CopyVerified)
}

fn ensure_cross_volume_copy_capacity(
    source: &Path,
    destination: &Path,
    required_bytes: u64,
    rename_error: &std::io::Error,
) -> Result<(), CleanerError> {
    let disks = Disks::new_with_refreshed_list();
    let source_disk = disk_for_path(&disks, source).ok_or_else(|| {
        CleanerError::CopyFallbackRefused(format!(
            "the source filesystem could not be identified after the atomic move failed: {rename_error}"
        ))
    })?;
    let destination_disk = disk_for_path(&disks, destination).ok_or_else(|| {
        CleanerError::CopyFallbackRefused(format!(
            "the quarantine filesystem could not be identified after the atomic move failed: {rename_error}"
        ))
    })?;
    if source_disk.0 == destination_disk.0 {
        return Err(CleanerError::CopyFallbackRefused(format!(
            "the atomic move failed on the same filesystem ({rename_error}); the original remains in place"
        )));
    }
    let safety_margin = required_bytes / 20;
    if destination_disk.1 < required_bytes.saturating_add(safety_margin) {
        return Err(CleanerError::CopyFallbackRefused(
            "the quarantine volume does not have enough free space for a verified copy".to_owned(),
        ));
    }
    Ok(())
}

fn disk_for_path(disks: &Disks, path: &Path) -> Option<(String, u64)> {
    let normalized_path = normalized_windows_path(path);
    disks
        .list()
        .iter()
        .filter_map(|disk| {
            let mount = normalized_windows_path(disk.mount_point());
            normalized_path
                .starts_with(&mount)
                .then_some((mount, disk.available_space()))
        })
        .max_by_key(|(mount, _)| mount.len())
}

fn normalized_windows_path(path: &Path) -> String {
    path_text(path)
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn copy_node(source: &Path, destination: &Path) -> Result<(), CleanerError> {
    let metadata = fs::symlink_metadata(source).map_err(|source| CleanerError::Io {
        action: "reading an item before verified copy",
        source,
    })?;
    if is_link_or_reparse(&metadata) {
        return Err(CleanerError::ItemChanged(path_text(source)));
    }
    if metadata.is_file() {
        fs::copy(source, destination).map_err(|source| CleanerError::Io {
            action: "copying an item into managed quarantine",
            source,
        })?;
        return Ok(());
    }
    fs::create_dir(destination).map_err(|source| CleanerError::Io {
        action: "creating a quarantine directory",
        source,
    })?;
    for child in fs::read_dir(source).map_err(|source| CleanerError::Io {
        action: "enumerating an item during verified copy",
        source,
    })? {
        let child = child.map_err(|source| CleanerError::Io {
            action: "reading an item during verified copy",
            source,
        })?;
        copy_node(&child.path(), &destination.join(child.file_name()))?;
    }
    Ok(())
}

fn remove_node(path: &Path) -> Result<(), CleanerError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| CleanerError::Io {
        action: "revalidating an item before removal",
        source,
    })?;
    if is_link_or_reparse(&metadata) {
        return Err(CleanerError::ItemChanged(path_text(path)));
    }
    if metadata.is_file() {
        return fs::remove_file(path).map_err(|source| CleanerError::Io {
            action: "removing a verified copied file",
            source,
        });
    }
    for child in fs::read_dir(path).map_err(|source| CleanerError::Io {
        action: "enumerating a verified tree before removal",
        source,
    })? {
        let child = child.map_err(|source| CleanerError::Io {
            action: "reading a verified tree before removal",
            source,
        })?;
        remove_node(&child.path())?;
    }
    fs::remove_dir(path).map_err(|source| CleanerError::Io {
        action: "removing a verified empty directory",
        source,
    })
}

fn ensure_new_destination_below(path: &Path, root: &Path) -> Result<(), CleanerError> {
    fs::create_dir_all(root).map_err(|source| CleanerError::Io {
        action: "creating the quarantine root",
        source,
    })?;
    let canonical_root = root.canonicalize().map_err(|source| CleanerError::Io {
        action: "canonicalizing the quarantine root",
        source,
    })?;
    let parent = path.parent().ok_or_else(|| {
        CleanerError::InvalidRequest("the quarantine destination has no parent".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(|source| CleanerError::Io {
        action: "creating a quarantine manifest directory",
        source,
    })?;
    let canonical_parent = parent.canonicalize().map_err(|source| CleanerError::Io {
        action: "canonicalizing the quarantine destination",
        source,
    })?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(CleanerError::UnsafeRoot(path_text(path)));
    }
    Ok(())
}

fn validate_existing_quarantine_path(path: &Path, root: &Path) -> Result<PathBuf, CleanerError> {
    let canonical_root = root.canonicalize().map_err(|source| CleanerError::Io {
        action: "canonicalizing the quarantine root",
        source,
    })?;
    let canonical = path.canonicalize().map_err(|source| CleanerError::Io {
        action: "locating a quarantined item",
        source,
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(CleanerError::UnsafeRoot(path_text(&canonical)));
    }
    ensure_no_reparse_ancestors(&canonical, &canonical_root)?;
    Ok(canonical)
}

fn safe_restore_alternative(original: &Path, item_id: &str) -> PathBuf {
    let suffix = item_id.chars().take(8).collect::<String>();
    let file_name = original
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "restored-item".to_owned());
    original.with_file_name(format!("{file_name}.mr-manager-restored-{suffix}"))
}

fn practical_lock_state(path: &Path, is_directory: bool) -> CleanupLockState {
    if is_directory {
        return CleanupLockState::Unknown;
    }
    match OpenOptions::new().read(true).write(true).open(path) {
        Ok(_) => CleanupLockState::Available,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            CleanupLockState::InUse
        }
        Err(_) => CleanupLockState::Unknown,
    }
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn canonical_or_absolute(path: &Path) -> Result<PathBuf, CleanerError> {
    if path.exists() {
        return path.canonicalize().map_err(|source| CleanerError::Io {
            action: "canonicalizing managed cleanup storage",
            source,
        });
    }
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .map_err(|source| CleanerError::Io {
                action: "resolving managed cleanup storage",
                source,
            })
    }
}

fn validate_identifier(value: &str, label: &str) -> Result<(), CleanerError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(CleanerError::InvalidRequest(format!(
            "{label} must be a bounded UUID-like identifier"
        )));
    }
    Ok(())
}

fn issue(path: &Path, code: &str, message: &str) -> CleanupScanIssue {
    CleanupScanIssue {
        path: path_text(path),
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn modified_ms(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_millis())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn request_for(root: &Path) -> CleanupScanRequest {
        CleanupScanRequest {
            operation_id: Uuid::new_v4().to_string(),
            root_paths: vec![path_text(root)],
        }
    }

    #[test]
    fn node_modules_round_trips_without_touching_source() {
        let temporary = tempfile::tempdir().expect("fixture root should be created");
        let project = temporary.path().join("fixture-project");
        let node_modules = project.join("node_modules").join("fixture-package");
        fs::create_dir_all(&node_modules).expect("fixture directories should be created");
        fs::write(project.join("source.ts"), b"export const source = true;\n")
            .expect("source fixture should be written");
        fs::write(node_modules.join("index.js"), b"module.exports = 42;\n")
            .expect("artifact fixture should be written");
        let original_bytes = fs::read(node_modules.join("index.js")).expect("fixture should read");
        let quarantine_root = temporary.path().join("app-data").join("quarantine");

        let scan = scan_cleanup_candidates(
            request_for(&project),
            Arc::new(AtomicBool::new(false)),
            &quarantine_root,
        )
        .expect("fixture scan should work");
        assert_eq!(scan.candidates.len(), 1);
        assert_eq!(scan.candidates[0].display_name, "node_modules");

        let mut manager = CleanerManager::new(quarantine_root.clone());
        manager.finish_scan(scan.clone());
        let mut plan = manager
            .create_plan(&CreateCleanupPlanRequest {
                scan_id: scan.scan_id,
                candidate_ids: vec![scan.candidates[0].id.clone()],
            })
            .expect("plan should be created from cached server evidence");
        let mut manifest = begin_manifest(&plan, &quarantine_root);
        quarantine_item(&plan.items[0], &mut manifest.items[0], &quarantine_root)
            .expect("fixture artifact should quarantine");
        refresh_manifest_state(&mut manifest);
        plan.state = CleanupPlanState::Executed;
        plan.manifest_id = Some(manifest.id.clone());

        assert!(!project.join("node_modules").exists());
        assert_eq!(
            fs::read(project.join("source.ts")).expect("source remains"),
            b"export const source = true;\n"
        );
        restore_item(
            &mut manifest.items[0],
            &quarantine_root,
            RestoreConflictStrategy::Fail,
        )
        .expect("fixture artifact should restore");
        assert_eq!(
            fs::read(node_modules.join("index.js")).expect("restored artifact should read"),
            original_bytes
        );
        assert_eq!(
            fs::read(project.join("source.ts")).expect("source remains"),
            b"export const source = true;\n"
        );
    }

    #[test]
    fn changed_candidate_is_rejected_before_quarantine() {
        let temporary = tempfile::tempdir().expect("fixture root should be created");
        let project = temporary.path().join("project");
        let candidate_path = project.join("node_modules");
        fs::create_dir_all(&candidate_path).expect("candidate should be created");
        fs::write(candidate_path.join("a.js"), b"before").expect("fixture should be written");
        let quarantine_root = temporary.path().join("quarantine");
        let scan = scan_cleanup_candidates(
            request_for(&project),
            Arc::new(AtomicBool::new(false)),
            &quarantine_root,
        )
        .expect("scan should work");
        fs::write(candidate_path.join("a.js"), b"after-change").expect("fixture should be changed");
        let plan = CleanupPlan {
            id: Uuid::new_v4().to_string(),
            scan_id: scan.scan_id,
            created_at_ms: now_ms(),
            roots: scan.roots,
            items: scan.candidates,
            total_size_bytes: 6,
            total_file_count: 1,
            state: CleanupPlanState::Reviewed,
            confirmation_phrase: "QUARANTINE 1 ITEMS".to_owned(),
            manifest_id: None,
        };
        let mut manifest = begin_manifest(&plan, &quarantine_root);
        assert!(matches!(
            quarantine_item(&plan.items[0], &mut manifest.items[0], &quarantine_root),
            Err(CleanerError::ItemChanged(_))
        ));
        assert!(candidate_path.exists());
    }

    #[test]
    fn restore_never_overwrites_an_occupied_destination() {
        let temporary = tempfile::tempdir().expect("fixture root should be created");
        let quarantine_root = temporary.path().join("quarantine");
        let source = quarantine_root.join("manifest").join("item");
        fs::create_dir_all(&source).expect("quarantine fixture should be created");
        fs::write(source.join("data.txt"), b"quarantined").expect("fixture should be written");
        let original = temporary.path().join("project").join("node_modules");
        fs::create_dir_all(&original).expect("occupied destination should be created");
        fs::write(original.join("new.txt"), b"newer").expect("new content should be written");
        let identity = tree_identity(&source, &Arc::new(AtomicBool::new(false)), 10)
            .expect("identity should calculate");
        let mut item = QuarantineItem {
            id: "12345678-0000-0000-0000-000000000000".to_owned(),
            original_canonical_path: path_text(&original),
            quarantine_path: path_text(&source),
            restored_path: None,
            size_bytes: identity.size_bytes,
            file_count: identity.file_count,
            category: CleanupCategory::DependencyCache,
            reason: "fixture".to_owned(),
            project_association: None,
            state: QuarantineItemState::Quarantined,
            verification: VerificationState::AtomicMove,
            purge_eligible: true,
            error: None,
            quarantined_at_ms: Some(now_ms()),
            restored_at_ms: None,
            purged_at_ms: None,
            purge_confirmation_phrase: "PURGE fixture".to_owned(),
        };
        assert!(matches!(
            restore_item(&mut item, &quarantine_root, RestoreConflictStrategy::Fail),
            Err(CleanerError::RestoreConflict(_))
        ));
        restore_item(
            &mut item,
            &quarantine_root,
            RestoreConflictStrategy::SafeAlternative,
        )
        .expect("safe alternative restore should work");
        assert_eq!(
            fs::read(original.join("new.txt")).expect("occupied content remains"),
            b"newer"
        );
        assert!(
            Path::new(
                item.restored_path
                    .as_deref()
                    .expect("restored path is recorded")
            )
            .exists()
        );
    }

    #[test]
    fn cancellation_stops_a_bounded_scan() {
        let temporary = tempfile::tempdir().expect("fixture root should be created");
        let project = temporary.path().join("project");
        fs::create_dir_all(&project).expect("fixture should be created");
        let cancellation = Arc::new(AtomicBool::new(true));
        let result = scan_cleanup_candidates(
            request_for(&project),
            cancellation,
            &temporary.path().join("quarantine"),
        )
        .expect("cancelled scan should return a safe partial result");
        assert!(result.cancelled);
        assert!(result.candidates.is_empty());
    }

    #[test]
    fn candidate_subtrees_share_the_depth_and_entry_safety_budget() {
        let temporary = tempfile::tempdir().expect("fixture root should be created");
        let project = temporary.path().join("project");
        let mut nested = project.join("node_modules");
        for index in 0..=MAX_DEPTH {
            nested = nested.join(format!("level-{index}"));
        }
        fs::create_dir_all(&nested).expect("deep synthetic candidate should be created");
        fs::write(nested.join("index.js"), b"fixture").expect("fixture should be written");

        let result = scan_cleanup_candidates(
            request_for(&project),
            Arc::new(AtomicBool::new(false)),
            &temporary.path().join("quarantine"),
        )
        .expect("bounded scan should return a safe result");

        assert!(result.limits_reached);
        assert!(result.candidates.is_empty());
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.code == "CANDIDATE_LIMIT_REACHED")
        );
    }

    #[test]
    fn root_and_links_are_rejected_or_skipped() {
        let root = Path::new(std::path::MAIN_SEPARATOR_STR);
        assert!(matches!(
            validate_selected_root(root, Path::new("managed-quarantine")),
            Err(CleanerError::UnsafeRoot(_))
        ));

        let temporary = tempfile::tempdir().expect("fixture root should be created");
        let project = temporary.path().join("project");
        fs::create_dir_all(&project).expect("fixture should be created");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(temporary.path(), project.join("node_modules"))
                .expect("fixture symlink should be created");
            let result = scan_cleanup_candidates(
                request_for(&project),
                Arc::new(AtomicBool::new(false)),
                &temporary.path().join("quarantine"),
            )
            .expect("scan should work");
            assert!(result.candidates.is_empty());
            assert!(
                result
                    .issues
                    .iter()
                    .any(|issue| issue.code == "REPARSE_POINT_SKIPPED")
            );
        }
    }

    #[test]
    fn purge_requires_the_exact_strong_confirmation() {
        let temporary = tempfile::tempdir().expect("fixture root should be created");
        let quarantine_root = temporary.path().join("quarantine");
        let path = quarantine_root.join("manifest").join("item");
        fs::create_dir_all(&path).expect("fixture should be created");
        let mut file = File::create(path.join("data.bin")).expect("fixture file should be created");
        file.write_all(b"fixture")
            .expect("fixture should be written");
        let mut item = QuarantineItem {
            id: "item".to_owned(),
            original_canonical_path: path_text(&temporary.path().join("original")),
            quarantine_path: path_text(&path),
            restored_path: None,
            size_bytes: 7,
            file_count: 1,
            category: CleanupCategory::BuildOutput,
            reason: "fixture".to_owned(),
            project_association: None,
            state: QuarantineItemState::Quarantined,
            verification: VerificationState::AtomicMove,
            purge_eligible: true,
            error: None,
            quarantined_at_ms: Some(now_ms()),
            restored_at_ms: None,
            purged_at_ms: None,
            purge_confirmation_phrase: "PURGE item".to_owned(),
        };
        assert!(matches!(
            purge_item(&mut item, &quarantine_root, "purge item"),
            Err(CleanerError::ConfirmationMismatch(_))
        ));
        assert!(path.exists());
        purge_item(&mut item, &quarantine_root, "PURGE item")
            .expect("exact confirmation should permit fixture purge");
        assert!(!path.exists());
    }
}
