use std::collections::{BTreeSet, HashMap, VecDeque};
use std::ffi::OsStr;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

use serde_json::Value as JsonValue;
use thiserror::Error;
use toml::Value as TomlValue;
use uuid::Uuid;

use crate::domain::{
    EnvironmentFileSummary, GitAvailability, GitSummary, PackageManagerSummary, Project,
    ProjectDiscoveryResult, ProjectIssue, ProjectIssueSeverity, ProjectManifest,
    ProjectManifestKind, ProjectScanHealth, ProjectScript, ProjectStack,
};

const MAX_DEPTH: u8 = 8;
const MAX_DISCOVERY_DIRECTORIES: u32 = 25_000;
const MAX_MANIFEST_BYTES: u64 = 1_048_576;
const MAX_ENV_BYTES: u64 = 262_144;
const GIT_TIMEOUT: Duration = Duration::from_millis(1_500);

#[derive(Debug, Error)]
pub enum ProjectScanError {
    #[error("the selected project root is invalid: {0}")]
    InvalidRoot(String),
    #[error("a project scan operation with this id is already running")]
    OperationAlreadyRunning,
    #[error("the project scan registry is unavailable")]
    RegistryUnavailable,
    #[error("the scan could not read {action}")]
    Io {
        action: &'static str,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone)]
pub struct ValidatedRoot {
    root_path: PathBuf,
    canonical_root_path: PathBuf,
}

impl ValidatedRoot {
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn canonical_root_path(&self) -> &Path {
        &self.canonical_root_path
    }

    pub fn root_path_text(&self) -> String {
        path_text(&self.root_path)
    }

    pub fn canonical_root_path_text(&self) -> String {
        path_text(&self.canonical_root_path)
    }
}

#[derive(Clone, Default)]
pub struct ScanRegistry {
    operations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl ScanRegistry {
    pub fn begin(&self, operation_id: String) -> Result<ScanPermit, ProjectScanError> {
        let mut operations = self
            .operations
            .lock()
            .map_err(|_| ProjectScanError::RegistryUnavailable)?;
        if operations.contains_key(&operation_id) {
            return Err(ProjectScanError::OperationAlreadyRunning);
        }

        let cancelled = Arc::new(AtomicBool::new(false));
        operations.insert(operation_id.clone(), Arc::clone(&cancelled));
        Ok(ScanPermit {
            operation_id,
            cancelled,
            registry: self.clone(),
        })
    }

    pub fn cancel(&self, operation_id: &str) -> Result<(), ProjectScanError> {
        let operations = self
            .operations
            .lock()
            .map_err(|_| ProjectScanError::RegistryUnavailable)?;
        if let Some(flag) = operations.get(operation_id) {
            flag.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    fn finish(&self, operation_id: &str) {
        if let Ok(mut operations) = self.operations.lock() {
            operations.remove(operation_id);
        }
    }
}

pub struct ScanPermit {
    operation_id: String,
    cancelled: Arc<AtomicBool>,
    registry: ScanRegistry,
}

impl ScanPermit {
    pub fn cancelled(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }
}

impl Drop for ScanPermit {
    fn drop(&mut self) {
        self.registry.finish(&self.operation_id);
    }
}

pub fn normalize_depth(maximum_depth: u8) -> u8 {
    maximum_depth.min(MAX_DEPTH)
}

pub fn validate_project_root(path: &Path) -> Result<ValidatedRoot, ProjectScanError> {
    if path.as_os_str().is_empty() {
        return Err(ProjectScanError::InvalidRoot(
            "The selected path is empty.".to_owned(),
        ));
    }

    let input_metadata = fs::symlink_metadata(path).map_err(|source| ProjectScanError::Io {
        action: "the selected root metadata",
        source,
    })?;
    if !input_metadata.is_dir() {
        return Err(ProjectScanError::InvalidRoot(
            "The selected path is not a directory.".to_owned(),
        ));
    }
    if is_reparse_or_symlink(&input_metadata) {
        return Err(ProjectScanError::InvalidRoot(
            "The selected root is a symlink, junction, or other reparse point.".to_owned(),
        ));
    }

    let canonical_root_path = fs::canonicalize(path).map_err(|source| ProjectScanError::Io {
        action: "the selected root path",
        source,
    })?;
    let canonical_metadata =
        fs::symlink_metadata(&canonical_root_path).map_err(|source| ProjectScanError::Io {
            action: "the canonical root metadata",
            source,
        })?;
    if is_reparse_or_symlink(&canonical_metadata) {
        return Err(ProjectScanError::InvalidRoot(
            "The canonical root is a symlink, junction, or other reparse point.".to_owned(),
        ));
    }
    if is_dangerous_root(&canonical_root_path) {
        return Err(ProjectScanError::InvalidRoot(format!(
            "{} is too broad or system-sensitive to scan as a project root.",
            path_text(&canonical_root_path)
        )));
    }

    Ok(ValidatedRoot {
        root_path: path.to_path_buf(),
        canonical_root_path,
    })
}

pub fn scan_project_path(path: &Path) -> Result<Project, ProjectScanError> {
    let root = validate_project_root(path)?;
    Ok(scan_validated_project(&root))
}

pub fn scan_validated_project(root: &ValidatedRoot) -> Project {
    let root_path = root.root_path();
    let canonical_root = root.canonical_root_path();
    let mut issues = Vec::new();
    let mut manifests = Vec::new();
    let mut stacks = BTreeSet::new();

    let package_json = read_json_manifest(root_path, "package.json", &mut issues);
    let pyproject = read_toml_manifest(root_path, "pyproject.toml", &mut issues);
    let cargo_toml = read_toml_manifest(root_path, "Cargo.toml", &mut issues);

    push_manifest(
        root_path,
        ".git",
        ProjectManifestKind::GitRepository,
        &mut manifests,
    );
    if push_manifest(
        root_path,
        "package.json",
        ProjectManifestKind::NodePackage,
        &mut manifests,
    ) {
        stacks.insert(ProjectStack::Node);
    }
    if push_manifest(
        root_path,
        "pyproject.toml",
        ProjectManifestKind::PythonProject,
        &mut manifests,
    ) {
        stacks.insert(ProjectStack::Python);
    }
    if push_manifest(
        root_path,
        "requirements.txt",
        ProjectManifestKind::PythonRequirements,
        &mut manifests,
    ) {
        stacks.insert(ProjectStack::Python);
    }
    if push_manifest(
        root_path,
        "uv.lock",
        ProjectManifestKind::UvLock,
        &mut manifests,
    ) {
        stacks.insert(ProjectStack::Python);
    }
    push_manifest(
        root_path,
        "poetry.lock",
        ProjectManifestKind::PoetryLock,
        &mut manifests,
    );
    push_manifest(
        root_path,
        "Pipfile",
        ProjectManifestKind::Pipfile,
        &mut manifests,
    );
    push_manifest(
        root_path,
        "Pipfile.lock",
        ProjectManifestKind::PipfileLock,
        &mut manifests,
    );
    if push_manifest(
        root_path,
        "Cargo.toml",
        ProjectManifestKind::RustCargo,
        &mut manifests,
    ) {
        stacks.insert(ProjectStack::Rust);
    }
    push_manifest(
        root_path,
        "Cargo.lock",
        ProjectManifestKind::CargoLock,
        &mut manifests,
    );

    let compose_files = detect_compose_files(root_path, &mut manifests);
    let package_manager = detect_package_manager(root_path, package_json.as_ref(), &mut issues);
    let mut scripts =
        detect_node_scripts(root_path, package_json.as_ref(), package_manager.as_ref());
    scripts.extend(detect_python_scripts(
        root_path,
        pyproject.as_ref(),
        root_path.join("uv.lock").is_file(),
    ));
    scripts.extend(detect_rust_scripts(root_path, cargo_toml.as_ref()));

    let environment_files = detect_environment_files(root_path, &mut issues);
    let local_database_hints = detect_local_database_hints(root_path);
    let git_summary = Some(detect_git_summary(canonical_root, &mut issues));

    if manifests
        .iter()
        .all(|manifest| manifest.kind == ProjectManifestKind::GitRepository)
    {
        issues.push(ProjectIssue::new(
            "NO_PROJECT_MANIFEST",
            ProjectIssueSeverity::Warning,
            "No Node, Python, Rust, or Compose manifest was found at this project root.",
        ));
    }
    if environment_files
        .iter()
        .any(|file| !file.example && file.relative_path.starts_with(".env"))
    {
        issues.push(ProjectIssue::new(
            "ENV_FILE_PRESENT",
            ProjectIssueSeverity::Information,
            "Environment file keys were detected; values were not read or stored.",
        ));
    }

    let name = detect_project_name(
        root_path,
        package_json.as_ref(),
        pyproject.as_ref(),
        cargo_toml.as_ref(),
    );
    let scan_health = ProjectScanHealth::from_issues(issues);

    Project {
        id: Uuid::new_v4().to_string(),
        name,
        root_path: path_text(root_path),
        canonical_root_path: path_text(canonical_root),
        tags: Vec::new(),
        notes: String::new(),
        checklist: Vec::new(),
        pinned: false,
        archived: false,
        detected_stacks: stacks.into_iter().collect(),
        manifests,
        package_manager,
        scripts,
        git_summary,
        compose_files,
        environment_files,
        local_database_hints,
        last_scanned_at: Some(now_ms()),
        scan_health,
    }
}

pub fn discover_projects(
    root: &ValidatedRoot,
    operation_id: String,
    maximum_depth: u8,
    cancelled: Arc<AtomicBool>,
) -> ProjectDiscoveryResult {
    let maximum_depth = normalize_depth(maximum_depth);
    let mut queue = VecDeque::from([(root.root_path().to_path_buf(), 0_u8)]);
    let mut projects = Vec::new();
    let mut scanned_directories = 0_u32;
    let mut skipped_directories = 0_u32;
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    let mut was_cancelled = false;

    while let Some((directory, depth)) = queue.pop_front() {
        if cancelled.load(Ordering::SeqCst) {
            was_cancelled = true;
            break;
        }
        if scanned_directories >= MAX_DISCOVERY_DIRECTORIES {
            issues.push(ProjectIssue::new(
                "SCAN_DIRECTORY_LIMIT_REACHED",
                ProjectIssueSeverity::Warning,
                "Project discovery stopped after reaching the directory limit.",
            ));
            break;
        }

        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(_) => {
                skipped_directories = skipped_directories.saturating_add(1);
                continue;
            }
        };
        if !metadata.is_dir() || is_reparse_or_symlink(&metadata) {
            skipped_directories = skipped_directories.saturating_add(1);
            continue;
        }

        let canonical = match fs::canonicalize(&directory) {
            Ok(canonical) => canonical,
            Err(_) => {
                skipped_directories = skipped_directories.saturating_add(1);
                continue;
            }
        };
        if !canonical.starts_with(root.canonical_root_path()) || !seen.insert(path_text(&canonical))
        {
            skipped_directories = skipped_directories.saturating_add(1);
            continue;
        }

        scanned_directories = scanned_directories.saturating_add(1);
        if looks_like_project(&directory) {
            let project_root = ValidatedRoot {
                root_path: directory.clone(),
                canonical_root_path: canonical,
            };
            projects.push(scan_validated_project(&project_root));
        }

        if depth >= maximum_depth {
            continue;
        }

        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                skipped_directories = skipped_directories.saturating_add(1);
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
            if should_skip_directory(name) {
                skipped_directories = skipped_directories.saturating_add(1);
                continue;
            }
            match entry.file_type() {
                Ok(file_type) if file_type.is_dir() => {
                    queue.push_back((path, depth.saturating_add(1)));
                }
                Ok(_) => {}
                Err(_) => {
                    skipped_directories = skipped_directories.saturating_add(1);
                }
            }
        }
    }

    ProjectDiscoveryResult {
        operation_id,
        projects,
        scanned_directories,
        skipped_directories,
        cancelled: was_cancelled,
        issues,
    }
}

pub fn merge_existing_metadata(scanned: &mut Project, existing: &Project) {
    scanned.id.clone_from(&existing.id);
    scanned.tags.clone_from(&existing.tags);
    scanned.notes.clone_from(&existing.notes);
    scanned.checklist.clone_from(&existing.checklist);
    scanned.pinned = existing.pinned;
    scanned.archived = existing.archived;
}

pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let clean = sanitize_text(tag).trim().to_owned();
        if clean.is_empty() || clean.len() > 40 {
            continue;
        }
        let key = clean.to_lowercase();
        if seen.insert(key) {
            normalized.push(clean);
        }
        if normalized.len() >= 24 {
            break;
        }
    }
    normalized
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn looks_like_project(path: &Path) -> bool {
    [
        "package.json",
        "pyproject.toml",
        "requirements.txt",
        "uv.lock",
        "Cargo.toml",
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ]
    .iter()
    .any(|name| path.join(name).exists())
}

fn push_manifest(
    root_path: &Path,
    relative_path: &'static str,
    kind: ProjectManifestKind,
    manifests: &mut Vec<ProjectManifest>,
) -> bool {
    if root_path.join(relative_path).exists() {
        manifests.push(ProjectManifest {
            kind,
            relative_path: relative_path.to_owned(),
        });
        true
    } else {
        false
    }
}

fn detect_compose_files(root_path: &Path, manifests: &mut Vec<ProjectManifest>) -> Vec<String> {
    let mut compose_files = Vec::new();
    for name in [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ] {
        if root_path.join(name).is_file() {
            compose_files.push(name.to_owned());
            manifests.push(ProjectManifest {
                kind: ProjectManifestKind::DockerCompose,
                relative_path: name.to_owned(),
            });
        }
    }
    compose_files
}

fn detect_package_manager(
    root_path: &Path,
    package_json: Option<&JsonValue>,
    issues: &mut Vec<ProjectIssue>,
) -> Option<PackageManagerSummary> {
    if package_json.is_none() && !root_path.join("package.json").is_file() {
        return None;
    }

    let mut evidence = Vec::new();
    let mut lockfiles = Vec::new();
    for (file, manager) in [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("bun.lockb", "bun"),
        ("bun.lock", "bun"),
        ("package-lock.json", "npm"),
    ] {
        if root_path.join(file).is_file() {
            evidence.push(file.to_owned());
            lockfiles.push((file, manager));
        }
    }

    let package_manager_field = package_json
        .and_then(|value| value.get("packageManager"))
        .and_then(JsonValue::as_str)
        .map(sanitize_text);
    if let Some(field) = &package_manager_field {
        evidence.push(format!("packageManager={field}"));
    }

    let field_manager = package_manager_field
        .as_deref()
        .and_then(|value| value.split('@').next())
        .filter(|value| matches!(*value, "npm" | "pnpm" | "yarn" | "bun"));
    let name = lockfiles
        .first()
        .map(|(_, manager)| (*manager).to_owned())
        .or_else(|| field_manager.map(str::to_owned))
        .unwrap_or_else(|| "npm".to_owned());
    let conflicting_lockfiles = if lockfiles.len() > 1 {
        lockfiles
            .iter()
            .map(|(file, _)| (*file).to_owned())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    if !conflicting_lockfiles.is_empty() {
        issues.push(
            ProjectIssue::new(
                "CONFLICTING_NODE_LOCKFILES",
                ProjectIssueSeverity::Warning,
                "Multiple Node lockfiles were found in the project root.",
            )
            .with_remediation(
                "Keep the lockfile for the package manager the project actually uses.",
            ),
        );
    }

    Some(PackageManagerSummary {
        name,
        evidence,
        conflicting_lockfiles,
    })
}

fn detect_node_scripts(
    root_path: &Path,
    package_json: Option<&JsonValue>,
    package_manager: Option<&PackageManagerSummary>,
) -> Vec<ProjectScript> {
    let manager_name = package_manager
        .map(|summary| summary.name.as_str())
        .unwrap_or("npm");
    let executable = package_manager_executable(manager_name);
    let Some(scripts) = package_json
        .and_then(|value| value.get("scripts"))
        .and_then(JsonValue::as_object)
    else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for (name, script) in scripts {
        if script.as_str().is_none() {
            continue;
        }
        let label = sanitize_text(name);
        if label.is_empty() {
            continue;
        }
        result.push(ProjectScript {
            id: format!("node:{label}"),
            label,
            source: "package.json".to_owned(),
            executable: executable.clone(),
            arguments: vec!["run".to_owned(), sanitize_text(name)],
            working_directory: path_text(root_path),
            verified: false,
        });
    }
    result.sort_by(|left, right| left.label.cmp(&right.label));
    result
}

fn detect_python_scripts(
    root_path: &Path,
    pyproject: Option<&TomlValue>,
    has_uv_lock: bool,
) -> Vec<ProjectScript> {
    let Some(scripts) = pyproject
        .and_then(|value| value.get("project"))
        .and_then(|value| value.get("scripts"))
        .and_then(TomlValue::as_table)
    else {
        return Vec::new();
    };

    let executable = if has_uv_lock {
        platform_executable("uv")
    } else {
        platform_executable("python")
    };
    let prefix = if has_uv_lock {
        vec!["run".to_owned()]
    } else {
        vec!["-m".to_owned()]
    };

    let mut result = Vec::new();
    for (name, value) in scripts {
        if !value.is_str() {
            continue;
        }
        let label = sanitize_text(name);
        if label.is_empty() {
            continue;
        }
        let mut arguments = prefix.clone();
        arguments.push(label.clone());
        result.push(ProjectScript {
            id: format!("python:{label}"),
            label,
            source: "pyproject.toml".to_owned(),
            executable: executable.clone(),
            arguments,
            working_directory: path_text(root_path),
            verified: false,
        });
    }
    result.sort_by(|left, right| left.label.cmp(&right.label));
    result
}

fn detect_rust_scripts(root_path: &Path, cargo_toml: Option<&TomlValue>) -> Vec<ProjectScript> {
    if cargo_toml.is_none() {
        return Vec::new();
    }
    ["check", "test", "run"]
        .into_iter()
        .map(|command| ProjectScript {
            id: format!("cargo:{command}"),
            label: format!("cargo {command}"),
            source: "Cargo.toml".to_owned(),
            executable: platform_executable("cargo"),
            arguments: vec![command.to_owned()],
            working_directory: path_text(root_path),
            verified: false,
        })
        .collect()
}

fn detect_environment_files(
    root_path: &Path,
    issues: &mut Vec<ProjectIssue>,
) -> Vec<EnvironmentFileSummary> {
    let mut files = Vec::new();
    for name in [
        ".env",
        ".env.local",
        ".env.development",
        ".env.production",
        ".env.example",
        ".env.sample",
    ] {
        let path = root_path.join(name);
        if !path.is_file() {
            continue;
        }
        let size_bytes = match path.metadata() {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        };
        if size_bytes > MAX_ENV_BYTES {
            issues.push(ProjectIssue::new(
                "ENV_FILE_TOO_LARGE",
                ProjectIssueSeverity::Warning,
                format!("{name} is too large for key-name inspection."),
            ));
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(contents) => files.push(EnvironmentFileSummary {
                relative_path: name.to_owned(),
                key_names: parse_env_key_names(&contents),
                example: name.ends_with(".example") || name.ends_with(".sample"),
                size_bytes,
            }),
            Err(_) => issues.push(ProjectIssue::new(
                "ENV_FILE_UNREADABLE",
                ProjectIssueSeverity::Warning,
                format!("{name} could not be read for key-name inspection."),
            )),
        }
    }
    files
}

fn detect_local_database_hints(root_path: &Path) -> Vec<String> {
    let mut hints = Vec::new();
    let Ok(entries) = fs::read_dir(root_path) else {
        return hints;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        let lower = name.to_lowercase();
        if lower.ends_with(".sqlite")
            || lower.ends_with(".sqlite3")
            || lower.ends_with(".db")
            || lower == "dev.db"
        {
            hints.push(name.to_owned());
        }
        if hints.len() >= 32 {
            break;
        }
    }
    hints.sort();
    hints
}

fn detect_project_name(
    root_path: &Path,
    package_json: Option<&JsonValue>,
    pyproject: Option<&TomlValue>,
    cargo_toml: Option<&TomlValue>,
) -> String {
    if let Some(name) = package_json
        .and_then(|value| value.get("name"))
        .and_then(JsonValue::as_str)
        .map(sanitize_text)
        .filter(|name| !name.is_empty())
    {
        return name;
    }
    if let Some(name) = pyproject
        .and_then(|value| value.get("project"))
        .and_then(|value| value.get("name"))
        .and_then(TomlValue::as_str)
        .map(sanitize_text)
        .filter(|name| !name.is_empty())
    {
        return name;
    }
    if let Some(name) = cargo_toml
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("name"))
        .and_then(TomlValue::as_str)
        .map(sanitize_text)
        .filter(|name| !name.is_empty())
    {
        return name;
    }
    root_path
        .file_name()
        .and_then(OsStr::to_str)
        .map(sanitize_text)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Untitled project".to_owned())
}

fn read_json_manifest(
    root_path: &Path,
    relative_path: &'static str,
    issues: &mut Vec<ProjectIssue>,
) -> Option<JsonValue> {
    let path = root_path.join(relative_path);
    if !path.is_file() {
        return None;
    }
    let text = read_manifest_text(&path, relative_path, issues)?;
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(_) => {
            issues.push(ProjectIssue::new(
                "MANIFEST_PARSE_ERROR",
                ProjectIssueSeverity::Warning,
                format!("{relative_path} could not be parsed."),
            ));
            None
        }
    }
}

fn read_toml_manifest(
    root_path: &Path,
    relative_path: &'static str,
    issues: &mut Vec<ProjectIssue>,
) -> Option<TomlValue> {
    let path = root_path.join(relative_path);
    if !path.is_file() {
        return None;
    }
    let text = read_manifest_text(&path, relative_path, issues)?;
    match toml::from_str::<TomlValue>(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(ProjectIssue::new(
                "MANIFEST_PARSE_ERROR",
                ProjectIssueSeverity::Warning,
                format!("{relative_path} could not be parsed: {error}"),
            ));
            None
        }
    }
}

fn read_manifest_text(
    path: &Path,
    relative_path: &'static str,
    issues: &mut Vec<ProjectIssue>,
) -> Option<String> {
    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(_) => {
            issues.push(ProjectIssue::new(
                "MANIFEST_UNREADABLE",
                ProjectIssueSeverity::Warning,
                format!("{relative_path} metadata could not be read."),
            ));
            return None;
        }
    };
    if metadata.len() > MAX_MANIFEST_BYTES {
        issues.push(ProjectIssue::new(
            "MANIFEST_TOO_LARGE",
            ProjectIssueSeverity::Warning,
            format!("{relative_path} is too large to parse safely."),
        ));
        return None;
    }
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(_) => {
            issues.push(ProjectIssue::new(
                "MANIFEST_UNREADABLE",
                ProjectIssueSeverity::Warning,
                format!("{relative_path} could not be read."),
            ));
            None
        }
    }
}

fn parse_env_key_names(contents: &str) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((key, _)) = assignment.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || key.len() > 120
            || !key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            continue;
        }
        keys.insert(key.to_owned());
        if keys.len() >= 200 {
            break;
        }
    }
    keys.into_iter().collect()
}

fn detect_git_summary(root: &Path, issues: &mut Vec<ProjectIssue>) -> GitSummary {
    match run_git(root, &["rev-parse", "--is-inside-work-tree"]) {
        Ok(output) if output.status_success && output.stdout.trim() == "true" => {}
        Ok(_) => return empty_git_summary(GitAvailability::NotRepository),
        Err(GitCommandError::NotFound) => {
            issues.push(ProjectIssue::new(
                "GIT_UNAVAILABLE",
                ProjectIssueSeverity::Information,
                "Git was not available on PATH during the scan.",
            ));
            return empty_git_summary(GitAvailability::Unavailable);
        }
        Err(GitCommandError::Timeout) => {
            issues.push(ProjectIssue::new(
                "GIT_TIMEOUT",
                ProjectIssueSeverity::Warning,
                "Git status collection timed out.",
            ));
            return empty_git_summary(GitAvailability::Error);
        }
        Err(GitCommandError::Io) => return empty_git_summary(GitAvailability::Error),
    }

    let branch_output = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
    let branch_text = branch_output
        .as_ref()
        .filter(|output| output.status_success)
        .map(|output| sanitize_text(output.stdout.trim()))
        .filter(|value| !value.is_empty());
    let detached_head = branch_text.as_deref() == Some("HEAD");
    let branch = branch_text.filter(|value| value != "HEAD");

    let mut summary = empty_git_summary(GitAvailability::Available);
    summary.branch = branch;
    summary.detached_head = detached_head;

    if let Ok(status) = run_git(
        root,
        &["status", "--porcelain=v1", "-b", "--untracked-files=normal"],
    ) && status.status_success
    {
        parse_git_status(&status.stdout, &mut summary);
    }
    if let Ok(counts) = run_git(
        root,
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    ) && counts.status_success
    {
        let parts = counts.stdout.split_whitespace().collect::<Vec<_>>();
        if parts.len() >= 2 {
            summary.behind = parts[0].parse::<u32>().ok();
            summary.ahead = parts[1].parse::<u32>().ok();
        }
    }
    if let Ok(commit) = run_git(root, &["log", "-1", "--format=%h %s"])
        && commit.status_success
    {
        summary.last_commit = Some(sanitize_text(commit.stdout.trim()))
            .filter(|value| !value.is_empty())
            .map(|value| value.chars().take(160).collect());
    }
    if let Ok(remotes) = run_git(root, &["remote"])
        && remotes.status_success
    {
        summary.remotes = remotes
            .stdout
            .lines()
            .map(sanitize_text)
            .filter(|line| !line.is_empty())
            .take(16)
            .collect();
    }

    summary
}

fn parse_git_status(stdout: &str, summary: &mut GitSummary) {
    for line in stdout.lines() {
        if line.starts_with("## ") {
            continue;
        }
        let chars = line.chars().collect::<Vec<_>>();
        if chars.len() < 2 {
            continue;
        }
        let x = chars[0];
        let y = chars[1];
        if x == '?' && y == '?' {
            summary.untracked = summary.untracked.saturating_add(1);
            continue;
        }
        if matches!((x, y), ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')) {
            summary.conflicted = summary.conflicted.saturating_add(1);
            continue;
        }
        if x != ' ' && x != '?' && x != '!' {
            summary.staged = summary.staged.saturating_add(1);
        }
        if x == 'D' || y == 'D' {
            summary.deleted = summary.deleted.saturating_add(1);
        } else if x == 'R' || y == 'R' {
            summary.renamed = summary.renamed.saturating_add(1);
        } else if x == 'M' || y == 'M' {
            summary.modified = summary.modified.saturating_add(1);
        }
    }
}

fn empty_git_summary(availability: GitAvailability) -> GitSummary {
    GitSummary {
        availability,
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

struct GitCommandOutput {
    status_success: bool,
    stdout: String,
}

enum GitCommandError {
    NotFound,
    Timeout,
    Io,
}

fn run_git(root: &Path, args: &[&str]) -> Result<GitCommandOutput, GitCommandError> {
    let mut command = crate::platform::process::hidden_command("git");
    command
        .arg("-C")
        .arg(root)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = command.spawn().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            GitCommandError::NotFound
        } else {
            GitCommandError::Io
        }
    })?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output().map_err(|_| GitCommandError::Io)?;
                return Ok(GitCommandOutput {
                    status_success: output.status.success(),
                    stdout: sanitize_text(&String::from_utf8_lossy(&output.stdout)),
                });
            }
            Ok(None) if started.elapsed() >= GIT_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitCommandError::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => return Err(GitCommandError::Io),
        }
    }
}

fn should_skip_directory(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git"
            | ".hg"
            | ".svn"
            | "node_modules"
            | "target"
            | ".venv"
            | "venv"
            | "__pycache__"
            | "dist"
            | "build"
            | ".next"
            | ".nuxt"
            | ".turbo"
            | ".cache"
            | "coverage"
            | ".idea"
    )
}

fn is_dangerous_root(canonical_path: &Path) -> bool {
    if canonical_path.parent().is_none() {
        return true;
    }

    let canonical = normalize_path_for_compare(canonical_path);
    if env_path_matches("WINDIR", &canonical)
        || env_path_matches("SystemRoot", &canonical)
        || env_path_matches("ProgramFiles", &canonical)
        || env_path_matches("ProgramFiles(x86)", &canonical)
        || env_path_matches("USERPROFILE", &canonical)
    {
        return true;
    }

    let sensitive_suffixes = [
        "\\windows",
        "\\program files",
        "\\program files (x86)",
        "\\users",
        "\\programdata",
    ];
    sensitive_suffixes
        .iter()
        .any(|suffix| canonical.ends_with(suffix))
}

fn env_path_matches(variable: &str, canonical: &str) -> bool {
    std::env::var_os(variable)
        .and_then(|path| fs::canonicalize(path).ok())
        .map(|path| normalize_path_for_compare(&path) == canonical)
        .unwrap_or(false)
}

fn normalize_path_for_compare(path: &Path) -> String {
    path_text(path).trim_end_matches('\\').to_ascii_lowercase()
}

fn is_reparse_or_symlink(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink() || has_reparse_point(metadata)
}

#[cfg(windows)]
fn has_reparse_point(metadata: &Metadata) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn has_reparse_point(_metadata: &Metadata) -> bool {
    false
}

fn package_manager_executable(name: &str) -> String {
    match name {
        "pnpm" => platform_executable("pnpm"),
        "yarn" => platform_executable("yarn"),
        "bun" => platform_executable("bun"),
        _ => platform_executable("npm"),
    }
}

fn platform_executable(name: &str) -> String {
    if cfg!(windows) {
        match name {
            "cargo" => "cargo.exe".to_owned(),
            "python" => "python.exe".to_owned(),
            _ => format!("{name}.cmd"),
        }
    } else {
        name.to_owned()
    }
}

fn sanitize_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            !character.is_control()
                || *character == '\n'
                || *character == '\r'
                || *character == '\t'
        })
        .collect::<String>()
        .replace('\r', "")
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("fixtures")
            .join("projects")
            .join(name)
    }

    #[test]
    fn node_fixture_scan_detects_scripts_without_shell_strings() {
        let project = scan_project_path(&fixture_path("vite-app")).expect("fixture should scan");

        assert!(project.detected_stacks.contains(&ProjectStack::Node));
        assert_eq!(
            project.package_manager.as_ref().map(|pm| pm.name.as_str()),
            Some("npm")
        );
        assert!(project.scripts.iter().any(|script| {
            script.label == "dev"
                && script.executable.ends_with("npm.cmd")
                && script.arguments == ["run", "dev"]
        }));
    }

    #[test]
    fn python_fixture_scan_keeps_only_env_key_names() {
        let project = scan_project_path(&fixture_path("python-uv")).expect("fixture should scan");

        assert!(project.detected_stacks.contains(&ProjectStack::Python));
        let env_file = project
            .environment_files
            .iter()
            .find(|file| file.relative_path == ".env.example")
            .expect("fixture env example should be detected");
        assert!(env_file.key_names.iter().any(|key| key == "DATABASE_URL"));
        assert!(!format!("{env_file:?}").contains("sqlite"));
    }

    #[test]
    fn rust_fixture_scan_adds_structured_cargo_actions() {
        let project = scan_project_path(&fixture_path("rust-cli")).expect("fixture should scan");

        assert!(project.detected_stacks.contains(&ProjectStack::Rust));
        assert!(project.scripts.iter().any(|script| {
            script.label == "cargo test"
                && script.executable.ends_with("cargo.exe")
                && script.arguments == ["test"]
        }));
    }

    #[test]
    fn conflicting_lockfiles_fixture_reports_warning() {
        let project =
            scan_project_path(&fixture_path("conflicting-lockfiles")).expect("fixture should scan");

        let package_manager = project
            .package_manager
            .expect("Node fixture should detect a package manager");
        assert!(package_manager.conflicting_lockfiles.len() >= 2);
        assert!(project.scan_health.issues.iter().any(|issue| {
            issue.code == "CONFLICTING_NODE_LOCKFILES"
                && issue.severity == ProjectIssueSeverity::Warning
        }));
    }

    #[test]
    fn compose_fixture_scan_detects_compose_file_and_env_keys() {
        let project =
            scan_project_path(&fixture_path("compose-stack")).expect("fixture should scan");

        assert_eq!(project.compose_files, vec!["compose.yaml"]);
        assert!(
            project
                .manifests
                .iter()
                .any(|manifest| manifest.kind == ProjectManifestKind::DockerCompose)
        );
        assert!(project.environment_files.iter().any(|file| {
            file.relative_path == ".env.example"
                && file.key_names.iter().any(|key| key == "DATABASE_URL")
        }));
    }

    #[test]
    fn bounded_discovery_finds_mixed_fixture_projects() {
        let root = validate_project_root(&fixture_path("mixed-monorepo")).expect("root is valid");
        let result = discover_projects(
            &root,
            "fixture-discovery".to_owned(),
            4,
            Arc::new(AtomicBool::new(false)),
        );

        assert!(!result.cancelled);
        assert!(result.scanned_directories > 1);
        assert!(
            result
                .projects
                .iter()
                .any(|project| project.detected_stacks.contains(&ProjectStack::Node))
        );
        assert!(
            result
                .projects
                .iter()
                .any(|project| project.detected_stacks.contains(&ProjectStack::Python))
        );
        assert!(
            result
                .projects
                .iter()
                .any(|project| project.detected_stacks.contains(&ProjectStack::Rust))
        );
    }
}
