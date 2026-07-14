use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::domain::{
    ComposeDoctorIssue, ComposeNetwork, ComposeParseSource, ComposePortMapping, ComposeProject,
    ComposeService, ComposeVolume, ComposeVolumeMount, DockerAvailability, DockerContainer,
    DockerContainerActionKind, DockerContainerActionRequest, DockerContainerActionResult,
    DockerDesktopProcessState, DockerDiagnostic, DockerInventory, DockerIssueSeverity, DockerLabel,
    DockerLogEntry, DockerMount, DockerNetwork, DockerPortMapping, DockerProjectAssociation,
    DockerResourceUsage, DockerStatus, DockerVolume, PortEndpoint, PortProtocol, ProcessSnapshot,
    Project,
};
use crate::security::redaction;

const DOCKER_TIMEOUT: Duration = Duration::from_millis(10_000);
const DOCKER_COMPOSE_TIMEOUT: Duration = Duration::from_millis(8_000);
const DOCKER_ACTION_TIMEOUT: Duration = Duration::from_millis(20_000);
const MAX_CONTAINERS: usize = 200;
const MAX_LABELS: usize = 80;
const MAX_COMPOSE_BYTES: u64 = 1_048_576;
const MAX_DOCKER_LOG_LINES: u16 = 400;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Docker CLI was not found on PATH")]
    CliMissing,
    #[error("Docker command timed out")]
    Timeout,
    #[error("Docker command failed: {stderr}")]
    CommandFailed {
        status_code: Option<i32>,
        stderr: String,
    },
    #[error("Docker command could not run")]
    Io(#[source] io::Error),
    #[error("Docker output could not be parsed: {0}")]
    Parse(String),
    #[error("the Docker container reference is invalid")]
    InvalidContainerReference,
    #[error("the Docker action confirmation did not match the required phrase")]
    InvalidConfirmation { expected: String },
    #[error("the Compose file path is invalid: {0}")]
    InvalidComposePath(String),
    #[error("the Compose file could not be read")]
    ComposeIo(#[source] io::Error),
}

#[derive(Debug)]
struct DockerCommandOutput {
    status_success: bool,
    status_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone)]
struct ComposeTextParse {
    services: Vec<ComposeService>,
    networks: Vec<ComposeNetwork>,
    volumes: Vec<ComposeVolume>,
    diagnostics: Vec<DockerDiagnostic>,
}

pub fn inspect_docker_status(processes: &[ProcessSnapshot]) -> DockerStatus {
    let desktop_process = docker_desktop_process_state(processes);
    let mut diagnostics = Vec::new();
    let collected_at_ms = unix_time_ms();

    let version = match run_docker(&["version", "--format", "{{json .}}"], DOCKER_TIMEOUT) {
        Ok(output) if output.status_success => output,
        Ok(output) => {
            let stderr = redaction::redact(&output.stderr);
            let availability = classify_failed_docker_version(&stderr);
            diagnostics.push(diagnostic_for_failed_version(availability, &stderr));
            return DockerStatus {
                availability,
                cli_detected: true,
                daemon_reachable: false,
                client_version: None,
                server_version: None,
                context: docker_context().ok(),
                docker_desktop_process: desktop_process,
                collected_at_ms,
                diagnostics,
            };
        }
        Err(DockerError::CliMissing) => {
            diagnostics.push(
                DockerDiagnostic::new(
                    "DOCKER_CLI_MISSING",
                    DockerIssueSeverity::Information,
                    "Docker CLI was not found on PATH.",
                )
                .with_remediation(
                    "Install Docker Desktop or ensure docker.exe is available on PATH.",
                ),
            );
            return DockerStatus {
                availability: DockerAvailability::CliMissing,
                cli_detected: false,
                daemon_reachable: false,
                client_version: None,
                server_version: None,
                context: None,
                docker_desktop_process: desktop_process,
                collected_at_ms,
                diagnostics,
            };
        }
        Err(DockerError::Timeout) => {
            diagnostics.push(
                DockerDiagnostic::new(
                    "DOCKER_VERSION_TIMEOUT",
                    DockerIssueSeverity::Warning,
                    "Docker did not answer the version probe before the timeout.",
                )
                .with_remediation("Open Docker Desktop and retry after it finishes starting."),
            );
            return DockerStatus {
                availability: if desktop_process == DockerDesktopProcessState::Running {
                    DockerAvailability::Starting
                } else {
                    DockerAvailability::Error
                },
                cli_detected: true,
                daemon_reachable: false,
                client_version: None,
                server_version: None,
                context: docker_context().ok(),
                docker_desktop_process: desktop_process,
                collected_at_ms,
                diagnostics,
            };
        }
        Err(error) => {
            diagnostics.push(
                DockerDiagnostic::new(
                    "DOCKER_VERSION_PROBE_FAILED",
                    DockerIssueSeverity::Warning,
                    "Docker version probing failed.",
                )
                .with_evidence(error.to_string())
                .with_remediation(
                    "Verify Docker Desktop is installed and accessible from this user account.",
                ),
            );
            return DockerStatus {
                availability: DockerAvailability::Error,
                cli_detected: true,
                daemon_reachable: false,
                client_version: None,
                server_version: None,
                context: docker_context().ok(),
                docker_desktop_process: desktop_process,
                collected_at_ms,
                diagnostics,
            };
        }
    };

    let parsed = serde_json::from_str::<JsonValue>(&version.stdout).ok();
    let client_version = parsed
        .as_ref()
        .and_then(|value| value.pointer("/Client/Version"))
        .and_then(JsonValue::as_str)
        .map(clean_text);
    let server_version = parsed
        .as_ref()
        .and_then(|value| value.pointer("/Server/Version"))
        .and_then(JsonValue::as_str)
        .map(clean_text);
    let context = docker_context().ok();

    DockerStatus {
        availability: DockerAvailability::Running,
        cli_detected: true,
        daemon_reachable: true,
        client_version,
        server_version,
        context,
        docker_desktop_process: desktop_process,
        collected_at_ms,
        diagnostics,
    }
}

pub fn list_docker_inventory(
    projects: &[Project],
    processes: &[ProcessSnapshot],
) -> Result<DockerInventory, DockerError> {
    let mut status = inspect_docker_status(processes);
    if status.availability != DockerAvailability::Running {
        return Ok(DockerInventory {
            status,
            containers: Vec::new(),
            networks: Vec::new(),
            volumes: Vec::new(),
        });
    }

    let containers = partial_docker_resource(&mut status, "containers", list_containers(projects));
    let networks = partial_docker_resource(&mut status, "networks", list_networks());
    let volumes = partial_docker_resource(&mut status, "volumes", list_volumes());
    Ok(DockerInventory {
        status,
        containers,
        networks,
        volumes,
    })
}

fn partial_docker_resource<T>(
    status: &mut DockerStatus,
    resource: &str,
    result: Result<Vec<T>, DockerError>,
) -> Vec<T> {
    match result {
        Ok(items) => items,
        Err(error) => {
            let lower = error.to_string().to_ascii_lowercase();
            let (code, message, remediation) = if matches!(error, DockerError::Timeout) {
                (
                    "DOCKER_PARTIAL_TIMEOUT",
                    format!("Docker {resource} did not respond before the bounded timeout."),
                    "Retry after Docker Desktop finishes starting; other Docker data remains usable.",
                )
            } else if lower.contains("permission denied") || lower.contains("access is denied") {
                (
                    "DOCKER_PARTIAL_PERMISSION_DENIED",
                    format!("Windows denied access while reading Docker {resource}."),
                    "Check Docker Desktop access for the current Windows user, then sign out and back in if group membership changed.",
                )
            } else {
                (
                    "DOCKER_PARTIAL_INVENTORY",
                    format!("Docker {resource} are temporarily unavailable."),
                    "Retry the Docker refresh; available inventory sections remain visible.",
                )
            };
            status.diagnostics.push(
                DockerDiagnostic::new(code, DockerIssueSeverity::Warning, message)
                    .with_remediation(remediation)
                    .with_evidence(redaction::redact(&error.to_string())),
            );
            Vec::new()
        }
    }
}

pub fn list_containers(projects: &[Project]) -> Result<Vec<DockerContainer>, DockerError> {
    let ids = docker_container_ids()?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut args = vec![
        OsString::from("container"),
        OsString::from("inspect"),
        OsString::from("--size=false"),
    ];
    args.extend(ids.iter().take(MAX_CONTAINERS).map(OsString::from));
    let output = run_docker_os(&args, DOCKER_TIMEOUT)?;
    ensure_success(&output)?;
    let mut containers = parse_containers_from_inspect(&output.stdout, projects)?;

    let stats = docker_stats().unwrap_or_default();
    for container in &mut containers {
        if let Some(usage) = stats
            .get(&container.id)
            .or_else(|| stats.get(&container.short_id))
        {
            container.resource_usage = Some(usage.clone());
        }
    }
    containers.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(containers)
}

pub fn list_networks() -> Result<Vec<DockerNetwork>, DockerError> {
    let output = run_docker(
        &["network", "ls", "--no-trunc", "--format", "{{json .}}"],
        DOCKER_TIMEOUT,
    )?;
    ensure_success(&output)?;
    let mut networks = Vec::new();
    for line in output.stdout.lines() {
        let Ok(value) = serde_json::from_str::<JsonValue>(line) else {
            continue;
        };
        let name = string_field(&value, &["Name"]).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        networks.push(DockerNetwork {
            name,
            id: string_field(&value, &["ID"]),
            driver: string_field(&value, &["Driver"]),
            scope: string_field(&value, &["Scope"]),
        });
    }
    networks.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(networks)
}

pub fn list_volumes() -> Result<Vec<DockerVolume>, DockerError> {
    let output = run_docker(&["volume", "ls", "--format", "{{json .}}"], DOCKER_TIMEOUT)?;
    ensure_success(&output)?;
    let mut volumes = Vec::new();
    for line in output.stdout.lines() {
        let Ok(value) = serde_json::from_str::<JsonValue>(line) else {
            continue;
        };
        let name = string_field(&value, &["Name"]).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        volumes.push(DockerVolume {
            name,
            driver: string_field(&value, &["Driver"]),
            scope: string_field(&value, &["Scope"]),
            mountpoint: string_field(&value, &["Mountpoint"]),
        });
    }
    volumes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(volumes)
}

pub fn container_logs(
    container_id: &str,
    max_lines: u16,
) -> Result<Vec<DockerLogEntry>, DockerError> {
    validate_container_reference(container_id)?;
    let max_lines = max_lines.clamp(1, MAX_DOCKER_LOG_LINES).to_string();
    let args = [
        OsString::from("logs"),
        OsString::from("--timestamps"),
        OsString::from("--tail"),
        OsString::from(max_lines),
        OsString::from(container_id),
    ];
    let output = run_docker_os(&args, DOCKER_TIMEOUT)?;
    ensure_success(&output)?;

    Ok(output
        .stdout
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let (timestamp, line) = split_docker_timestamp(line);
            DockerLogEntry {
                sequence: u64::try_from(index).unwrap_or(u64::MAX),
                timestamp,
                line: redaction::redact(line).chars().take(2_000).collect(),
            }
        })
        .collect())
}

pub fn container_action(
    request: DockerContainerActionRequest,
    projects: &[Project],
) -> Result<DockerContainerActionResult, DockerError> {
    validate_container_reference(&request.container_id)?;
    let before = inspect_single_container(&request.container_id, projects)?;
    let expected = format!(
        "{} {}",
        request.action.as_command(),
        confirmation_name(&before)
    );
    if request.confirmation.trim() != expected {
        return Err(DockerError::InvalidConfirmation { expected });
    }

    let mut args = vec![
        OsString::from("container"),
        OsString::from(request.action.as_command()),
    ];
    if matches!(
        request.action,
        DockerContainerActionKind::Stop | DockerContainerActionKind::Restart
    ) {
        args.push(OsString::from("--time"));
        args.push(OsString::from("10"));
    }
    args.push(OsString::from(&request.container_id));

    let output = run_docker_os(&args, DOCKER_ACTION_TIMEOUT)?;
    ensure_success(&output)?;
    let container = inspect_single_container(&request.container_id, projects)?;

    Ok(DockerContainerActionResult {
        action: request.action,
        container,
        stdout: redaction::redact(&output.stdout)
            .chars()
            .take(1_000)
            .collect(),
    })
}

pub fn list_compose_projects(
    projects: &[Project],
    ports: &[PortEndpoint],
    containers: Option<&[DockerContainer]>,
) -> Vec<ComposeProject> {
    let mut result = Vec::new();
    for project in projects {
        for compose_file in &project.compose_files {
            match parse_compose_project(project, compose_file, ports, containers) {
                Ok(parsed) => result.push(parsed),
                Err(error) => result.push(compose_project_error(project, compose_file, error)),
            }
        }
    }
    result.sort_by(|left, right| {
        left.project_name
            .to_lowercase()
            .cmp(&right.project_name.to_lowercase())
            .then_with(|| left.compose_file.cmp(&right.compose_file))
    });
    result
}

pub fn parse_compose_project(
    project: &Project,
    compose_file: &str,
    ports: &[PortEndpoint],
    containers: Option<&[DockerContainer]>,
) -> Result<ComposeProject, DockerError> {
    let (path, relative_path) = validated_compose_path(project, compose_file)?;
    let text = read_compose_text(&path)?;
    let mut parsed = try_parse_canonical_compose(project, &relative_path, &path, &text)
        .unwrap_or_else(|diagnostic| {
            let mut fallback = parse_compose_text(project, &relative_path, &text);
            fallback.parse_diagnostics.push(diagnostic);
            fallback
        });
    parsed.doctor = compose_doctor(&parsed, ports, containers);
    Ok(parsed)
}

pub fn run_compose_doctor(
    project: &Project,
    compose_file: &str,
    ports: &[PortEndpoint],
    containers: Option<&[DockerContainer]>,
) -> Result<Vec<ComposeDoctorIssue>, DockerError> {
    parse_compose_project(project, compose_file, ports, containers).map(|project| project.doctor)
}

fn docker_container_ids() -> Result<Vec<String>, DockerError> {
    let output = run_docker(
        &["container", "ls", "--all", "--quiet", "--no-trunc"],
        DOCKER_TIMEOUT,
    )?;
    ensure_success(&output)?;
    Ok(output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| validate_container_reference(line).is_ok())
        .take(MAX_CONTAINERS)
        .map(ToOwned::to_owned)
        .collect())
}

fn docker_stats() -> Result<HashMap<String, DockerResourceUsage>, DockerError> {
    let output = run_docker(
        &["stats", "--no-stream", "--format", "{{json .}}"],
        DOCKER_TIMEOUT,
    )?;
    ensure_success(&output)?;
    let mut stats = HashMap::new();
    for line in output.stdout.lines() {
        let Ok(value) = serde_json::from_str::<JsonValue>(line) else {
            continue;
        };
        let Some(container) = string_field(&value, &["Container"]) else {
            continue;
        };
        let usage = DockerResourceUsage {
            cpu_percent: string_field(&value, &["CPUPerc"]),
            memory_usage: string_field(&value, &["MemUsage"]),
            memory_percent: string_field(&value, &["MemPerc"]),
            network_io: string_field(&value, &["NetIO"]),
            block_io: string_field(&value, &["BlockIO"]),
        };
        stats.insert(container, usage.clone());
        if let Some(name) = string_field(&value, &["Name"]) {
            stats.insert(name, usage);
        }
    }
    Ok(stats)
}

fn inspect_single_container(
    container_id: &str,
    projects: &[Project],
) -> Result<DockerContainer, DockerError> {
    let args = [
        OsString::from("container"),
        OsString::from("inspect"),
        OsString::from("--size=false"),
        OsString::from(container_id),
    ];
    let output = run_docker_os(&args, DOCKER_TIMEOUT)?;
    ensure_success(&output)?;
    let mut containers = parse_containers_from_inspect(&output.stdout, projects)?;
    containers
        .pop()
        .ok_or_else(|| DockerError::Parse("Docker inspect returned no container.".to_owned()))
}

fn parse_containers_from_inspect(
    stdout: &str,
    projects: &[Project],
) -> Result<Vec<DockerContainer>, DockerError> {
    let value = serde_json::from_str::<JsonValue>(stdout)
        .map_err(|error| DockerError::Parse(error.to_string()))?;
    let Some(items) = value.as_array() else {
        return Err(DockerError::Parse(
            "Docker inspect output was not an array.".to_owned(),
        ));
    };

    Ok(items
        .iter()
        .filter_map(|value| container_from_inspect_value(value, projects))
        .collect())
}

fn container_from_inspect_value(
    value: &JsonValue,
    projects: &[Project],
) -> Option<DockerContainer> {
    let id = string_field(value, &["Id"])?;
    let labels = labels_from_value(value.pointer("/Config/Labels"));
    let labels_map = labels
        .iter()
        .map(|label| (label.key.as_str(), label.value.as_str()))
        .collect::<HashMap<_, _>>();
    let name = string_field(value, &["Name"])
        .unwrap_or_else(|| id.chars().take(12).collect())
        .trim_start_matches('/')
        .to_owned();
    let state = string_field(value, &["State", "Status"]).unwrap_or_else(|| "unknown".to_owned());
    let health = value
        .pointer("/State/Health/Status")
        .and_then(JsonValue::as_str)
        .map(clean_text);
    let compose_working_dir = labels_map
        .get("com.docker.compose.project.working_dir")
        .map(|value| clean_text(value));

    Some(DockerContainer {
        short_id: id.chars().take(12).collect(),
        image: string_field(value, &["Config", "Image"])
            .or_else(|| string_field(value, &["Image"]))
            .unwrap_or_else(|| "unknown".to_owned()),
        status: status_text(value),
        created_at: string_field(value, &["Created"]),
        started_at: string_field(value, &["State", "StartedAt"]),
        finished_at: string_field(value, &["State", "FinishedAt"]),
        ports: ports_from_inspect(value.pointer("/NetworkSettings/Ports")),
        networks: networks_from_inspect(value.pointer("/NetworkSettings/Networks")),
        mounts: mounts_from_inspect(value.pointer("/Mounts")),
        compose_project: labels_map
            .get("com.docker.compose.project")
            .map(|value| clean_text(value)),
        compose_service: labels_map
            .get("com.docker.compose.service")
            .map(|value| clean_text(value)),
        compose_working_dir,
        user: string_field(value, &["Config", "User"]).filter(|value| !value.is_empty()),
        restart_policy: string_field(value, &["HostConfig", "RestartPolicy", "Name"])
            .filter(|value| !value.is_empty() && value != "no"),
        associated_project: associate_container_to_project(&labels_map, projects),
        labels,
        health,
        state,
        name,
        id,
        resource_usage: None,
    })
}

fn status_text(value: &JsonValue) -> String {
    let running = value
        .pointer("/State/Running")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let status = string_field(value, &["State", "Status"]).unwrap_or_else(|| "unknown".to_owned());
    let health = value
        .pointer("/State/Health/Status")
        .and_then(JsonValue::as_str);
    match (running, health) {
        (_, Some(health)) => format!("{status} ({})", clean_text(health)),
        _ => status,
    }
}

fn labels_from_value(value: Option<&JsonValue>) -> Vec<DockerLabel> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Vec::new();
    };
    let mut labels = object
        .iter()
        .take(MAX_LABELS)
        .filter_map(|(key, value)| {
            value.as_str().map(|value| DockerLabel {
                key: clean_text(key),
                value: redaction::redact(value).chars().take(400).collect(),
            })
        })
        .collect::<Vec<_>>();
    labels.sort_by(|left, right| left.key.cmp(&right.key));
    labels
}

fn ports_from_inspect(value: Option<&JsonValue>) -> Vec<DockerPortMapping> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Vec::new();
    };
    let mut ports = Vec::new();
    for (container_port, host_bindings) in object {
        let Some((port, protocol)) = parse_container_port_key(container_port) else {
            continue;
        };
        match host_bindings {
            JsonValue::Array(bindings) if !bindings.is_empty() => {
                for binding in bindings {
                    ports.push(DockerPortMapping {
                        host_ip: string_field(binding, &["HostIp"])
                            .filter(|value| !value.is_empty()),
                        host_port: string_field(binding, &["HostPort"])
                            .and_then(|value| value.parse::<u16>().ok()),
                        container_port: port,
                        protocol,
                    });
                }
            }
            _ => ports.push(DockerPortMapping {
                host_ip: None,
                host_port: None,
                container_port: port,
                protocol,
            }),
        }
    }
    ports.sort_by(|left, right| {
        left.host_port
            .cmp(&right.host_port)
            .then_with(|| left.container_port.cmp(&right.container_port))
    });
    ports
}

fn parse_container_port_key(value: &str) -> Option<(u16, PortProtocol)> {
    let (port, protocol) = value.split_once('/')?;
    let protocol = match protocol {
        "tcp" => PortProtocol::Tcp,
        "udp" => PortProtocol::Udp,
        _ => return None,
    };
    Some((port.parse().ok()?, protocol))
}

fn networks_from_inspect(value: Option<&JsonValue>) -> Vec<String> {
    let mut networks = value
        .and_then(JsonValue::as_object)
        .map(|object| object.keys().map(|key| clean_text(key)).collect::<Vec<_>>())
        .unwrap_or_default();
    networks.sort();
    networks
}

fn mounts_from_inspect(value: Option<&JsonValue>) -> Vec<DockerMount> {
    let mut mounts = value
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let destination = string_field(item, &["Destination"])?;
                    Some(DockerMount {
                        kind: string_field(item, &["Type"]).unwrap_or_else(|| "unknown".to_owned()),
                        source: string_field(item, &["Source"]).filter(|value| !value.is_empty()),
                        destination,
                        mode: string_field(item, &["Mode"]).filter(|value| !value.is_empty()),
                        read_write: item.get("RW").and_then(JsonValue::as_bool),
                        name: string_field(item, &["Name"]).filter(|value| !value.is_empty()),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    mounts.sort_by(|left, right| left.destination.cmp(&right.destination));
    mounts
}

fn associate_container_to_project(
    labels: &HashMap<&str, &str>,
    projects: &[Project],
) -> Option<DockerProjectAssociation> {
    let working_dir = labels
        .get("com.docker.compose.project.working_dir")
        .map(|value| normalize_path_for_compare(value));
    let config_files = labels
        .get("com.docker.compose.project.config_files")
        .map(|value| normalize_path_for_compare(value));

    for project in projects {
        let project_root = normalize_path_for_compare(&project.canonical_root_path);
        if working_dir
            .as_deref()
            .is_some_and(|path| path.starts_with(&project_root))
            || config_files
                .as_deref()
                .is_some_and(|path| path.contains(&project_root))
        {
            return Some(DockerProjectAssociation {
                project_id: project.id.clone(),
                project_name: project.name.clone(),
                project_root: project.root_path.clone(),
                confidence: "strong".to_owned(),
                evidence: vec![
                    "Docker Compose labels include a working directory or config file under the registered project root."
                        .to_owned(),
                ],
            });
        }
    }

    let compose_project = labels.get("com.docker.compose.project")?;
    projects
        .iter()
        .find(|project| project.name.eq_ignore_ascii_case(compose_project))
        .map(|project| DockerProjectAssociation {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            project_root: project.root_path.clone(),
            confidence: "inferred".to_owned(),
            evidence: vec![
                "Docker Compose project label matches a registered project name.".to_owned(),
            ],
        })
}

fn try_parse_canonical_compose(
    project: &Project,
    relative_path: &str,
    path: &Path,
    text: &str,
) -> Result<ComposeProject, DockerDiagnostic> {
    let args = [
        OsString::from("compose"),
        OsString::from("-f"),
        path.as_os_str().to_owned(),
        OsString::from("config"),
        OsString::from("--format"),
        OsString::from("json"),
    ];
    let output = run_docker_os(&args, DOCKER_COMPOSE_TIMEOUT).map_err(|error| {
        DockerDiagnostic::new(
            "COMPOSE_CANONICAL_CONFIG_UNAVAILABLE",
            DockerIssueSeverity::Information,
            "Docker Compose canonical config was unavailable; Mr Manager used the safe fallback parser.",
        )
        .with_evidence(redaction::redact(&error.to_string()))
    })?;
    if !output.status_success {
        return Err(DockerDiagnostic::new(
            "COMPOSE_CANONICAL_CONFIG_UNAVAILABLE",
            DockerIssueSeverity::Information,
            "Docker Compose canonical config failed; Mr Manager used the safe fallback parser.",
        )
        .with_evidence(redaction::redact(&output.stderr)));
    }
    let value = serde_json::from_str::<JsonValue>(&output.stdout).map_err(|error| {
        DockerDiagnostic::new(
            "COMPOSE_CANONICAL_CONFIG_PARSE_FAILED",
            DockerIssueSeverity::Information,
            "Docker Compose returned config output Mr Manager could not parse; fallback parser was used.",
        )
        .with_evidence(error.to_string())
    })?;
    let mut parsed = parse_compose_json(project, relative_path, &value);
    parsed.unresolved_interpolation = unresolved_interpolation(text, project);
    Ok(parsed)
}

fn parse_compose_json(project: &Project, relative_path: &str, value: &JsonValue) -> ComposeProject {
    let mut services = value
        .get("services")
        .and_then(JsonValue::as_object)
        .map(|object| {
            object
                .iter()
                .map(|(name, value)| compose_service_from_json(name, value))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    services.sort_by(|left, right| left.name.cmp(&right.name));

    ComposeProject {
        id: compose_project_id(project, relative_path),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        project_root: project.root_path.clone(),
        compose_file: relative_path.to_owned(),
        source: ComposeParseSource::DockerComposeConfig,
        services,
        networks: compose_named_section(value.get("networks")).0,
        volumes: compose_named_section(value.get("volumes")).1,
        unresolved_interpolation: Vec::new(),
        parse_diagnostics: Vec::new(),
        doctor: Vec::new(),
    }
}

fn compose_service_from_json(name: &str, value: &JsonValue) -> ComposeService {
    ComposeService {
        name: clean_text(name),
        image: string_field(value, &["image"]),
        build: compose_build_from_json(value.get("build")),
        container_name: string_field(value, &["container_name"]),
        command: string_field(value, &["command"]),
        user: string_field(value, &["user"]),
        restart: string_field(value, &["restart"]),
        ports: compose_ports_from_json(value.get("ports")),
        volumes: compose_volumes_from_json(value.get("volumes")),
        environment_keys: compose_environment_keys(value.get("environment")),
        depends_on: compose_string_or_object_keys(value.get("depends_on")),
        networks: compose_string_or_object_keys(value.get("networks")),
        profiles: compose_string_list(value.get("profiles")),
        healthcheck_present: value.get("healthcheck").is_some(),
    }
}

fn compose_build_from_json(value: Option<&JsonValue>) -> Option<String> {
    match value {
        Some(JsonValue::String(value)) => Some(clean_text(value)),
        Some(JsonValue::Object(object)) => object
            .get("context")
            .and_then(JsonValue::as_str)
            .map(clean_text),
        _ => None,
    }
}

fn compose_ports_from_json(value: Option<&JsonValue>) -> Vec<ComposePortMapping> {
    value
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    JsonValue::String(value) => parse_compose_port(value),
                    JsonValue::Object(object) => {
                        let target = object
                            .get("target")
                            .and_then(JsonValue::as_u64)
                            .and_then(|value| u16::try_from(value).ok())?;
                        let published = object
                            .get("published")
                            .and_then(|value| {
                                value
                                    .as_str()
                                    .map(ToOwned::to_owned)
                                    .or_else(|| value.as_u64().map(|number| number.to_string()))
                            })
                            .and_then(|value| value.parse::<u16>().ok());
                        let protocol = object
                            .get("protocol")
                            .and_then(JsonValue::as_str)
                            .map(|value| {
                                if value == "udp" {
                                    PortProtocol::Udp
                                } else {
                                    PortProtocol::Tcp
                                }
                            })
                            .unwrap_or(PortProtocol::Tcp);
                        Some(ComposePortMapping {
                            host_ip: object
                                .get("host_ip")
                                .and_then(JsonValue::as_str)
                                .map(clean_text),
                            host_port: published,
                            container_port: target,
                            protocol,
                            raw: item.to_string(),
                        })
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn compose_volumes_from_json(value: Option<&JsonValue>) -> Vec<ComposeVolumeMount> {
    value
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    JsonValue::String(value) => Some(parse_compose_volume(value)),
                    JsonValue::Object(object) => {
                        let raw = item.to_string();
                        Some(ComposeVolumeMount {
                            source: object
                                .get("source")
                                .and_then(JsonValue::as_str)
                                .map(clean_text),
                            target: object
                                .get("target")
                                .and_then(JsonValue::as_str)
                                .map(clean_text),
                            mode: object.get("read_only").and_then(JsonValue::as_bool).map(
                                |read_only| {
                                    if read_only {
                                        "ro".to_owned()
                                    } else {
                                        "rw".to_owned()
                                    }
                                },
                            ),
                            raw,
                        })
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn compose_environment_keys(value: Option<&JsonValue>) -> Vec<String> {
    let mut keys = match value {
        Some(JsonValue::Object(object)) => object.keys().map(|key| clean_text(key)).collect(),
        Some(JsonValue::Array(items)) => items
            .iter()
            .filter_map(JsonValue::as_str)
            .filter_map(environment_key_from_assignment)
            .collect(),
        _ => Vec::new(),
    };
    keys.sort();
    keys.dedup();
    keys
}

fn compose_string_or_object_keys(value: Option<&JsonValue>) -> Vec<String> {
    let mut result = match value {
        Some(JsonValue::Array(items)) => items
            .iter()
            .filter_map(JsonValue::as_str)
            .map(clean_text)
            .collect(),
        Some(JsonValue::Object(object)) => object.keys().map(|key| clean_text(key)).collect(),
        Some(JsonValue::String(value)) => vec![clean_text(value)],
        _ => Vec::new(),
    };
    result.sort();
    result.dedup();
    result
}

fn compose_string_list(value: Option<&JsonValue>) -> Vec<String> {
    value
        .and_then(JsonValue::as_array)
        .map(|items| {
            let mut result = items
                .iter()
                .filter_map(JsonValue::as_str)
                .map(clean_text)
                .collect::<Vec<_>>();
            result.sort();
            result.dedup();
            result
        })
        .unwrap_or_default()
}

fn compose_named_section(value: Option<&JsonValue>) -> (Vec<ComposeNetwork>, Vec<ComposeVolume>) {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return (Vec::new(), Vec::new());
    };
    let mut networks = Vec::new();
    let mut volumes = Vec::new();
    for (name, section) in object {
        let external = match section.get("external") {
            Some(JsonValue::Bool(value)) => *value,
            Some(JsonValue::Object(_)) => true,
            _ => false,
        };
        networks.push(ComposeNetwork {
            name: clean_text(name),
            external,
        });
        volumes.push(ComposeVolume {
            name: clean_text(name),
            external,
        });
    }
    networks.sort_by(|left, right| left.name.cmp(&right.name));
    volumes.sort_by(|left, right| left.name.cmp(&right.name));
    (networks, volumes)
}

fn parse_compose_text(project: &Project, relative_path: &str, text: &str) -> ComposeProject {
    let parsed = parse_compose_text_sections(text);
    ComposeProject {
        id: compose_project_id(project, relative_path),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        project_root: project.root_path.clone(),
        compose_file: relative_path.to_owned(),
        source: ComposeParseSource::FallbackParser,
        services: parsed.services,
        networks: parsed.networks,
        volumes: parsed.volumes,
        unresolved_interpolation: unresolved_interpolation(text, project),
        parse_diagnostics: parsed.diagnostics,
        doctor: Vec::new(),
    }
}

fn parse_compose_text_sections(text: &str) -> ComposeTextParse {
    let mut services = BTreeMap::<String, ComposeService>::new();
    let mut networks = BTreeMap::<String, ComposeNetwork>::new();
    let mut volumes = BTreeMap::<String, ComposeVolume>::new();
    let mut diagnostics = Vec::new();
    let mut section = String::new();
    let mut current_service: Option<String> = None;
    let mut current_field: Option<String> = None;
    let mut current_named: Option<String> = None;

    for (line_number, original_line) in text.lines().enumerate() {
        let Some(line) = strip_comment(original_line) else {
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('\t') {
            diagnostics.push(parse_warning(
                line_number,
                "Tabs are not valid Compose YAML indentation.",
            ));
            continue;
        }
        let indent = line
            .chars()
            .take_while(|character| *character == ' ')
            .count();
        let trimmed = line.trim();

        if indent == 0 {
            if let Some((key, value)) = split_key_value(trimmed) {
                section = key;
                current_service = None;
                current_field = None;
                current_named = None;
                if !matches!(
                    section.as_str(),
                    "services" | "networks" | "volumes" | "version" | "name"
                ) {
                    diagnostics.push(parse_warning(
                        line_number,
                        format!("Top-level Compose section `{section}` is not interpreted by the fallback parser."),
                    ));
                }
                if !value.is_empty()
                    && matches!(section.as_str(), "services" | "networks" | "volumes")
                {
                    diagnostics.push(parse_warning(
                        line_number,
                        format!("Inline top-level section `{section}` is not supported by the fallback parser."),
                    ));
                }
            }
            continue;
        }

        match section.as_str() {
            "services" => {
                if indent == 2 {
                    if let Some((name, _)) = split_key_value(trimmed) {
                        services
                            .entry(name.clone())
                            .or_insert_with(|| empty_service(&name));
                        current_service = Some(name);
                        current_field = None;
                    }
                } else if indent == 4 {
                    let Some(service_name) = current_service.clone() else {
                        diagnostics.push(parse_warning(
                            line_number,
                            "Service field appeared before a service name.",
                        ));
                        continue;
                    };
                    if let Some((field, value)) = split_key_value(trimmed) {
                        current_field = Some(field.clone());
                        if let Some(service) = services.get_mut(&service_name) {
                            record_service_field(service, &field, &value);
                        }
                    }
                } else if indent >= 6 {
                    let Some(service_name) = current_service.clone() else {
                        continue;
                    };
                    let Some(field) = current_field.clone() else {
                        continue;
                    };
                    if let Some(service) = services.get_mut(&service_name) {
                        record_service_nested(service, &field, trimmed);
                    }
                }
            }
            "networks" => {
                if indent == 2 {
                    if let Some((name, _)) = split_key_value(trimmed) {
                        networks.entry(name.clone()).or_insert(ComposeNetwork {
                            name: name.clone(),
                            external: false,
                        });
                        current_named = Some(name);
                    }
                } else if indent == 4
                    && let (Some(name), Some((key, value))) =
                        (current_named.clone(), split_key_value(trimmed))
                    && key == "external"
                    && let Some(network) = networks.get_mut(&name)
                {
                    network.external = scalar_bool(&value);
                }
            }
            "volumes" => {
                if indent == 2 {
                    if let Some((name, _)) = split_key_value(trimmed) {
                        volumes.entry(name.clone()).or_insert(ComposeVolume {
                            name: name.clone(),
                            external: false,
                        });
                        current_named = Some(name);
                    }
                } else if indent == 4
                    && let (Some(name), Some((key, value))) =
                        (current_named.clone(), split_key_value(trimmed))
                    && key == "external"
                    && let Some(volume) = volumes.get_mut(&name)
                {
                    volume.external = scalar_bool(&value);
                }
            }
            _ => {}
        }
    }

    ComposeTextParse {
        services: services.into_values().collect(),
        networks: networks.into_values().collect(),
        volumes: volumes.into_values().collect(),
        diagnostics,
    }
}

fn empty_service(name: &str) -> ComposeService {
    ComposeService {
        name: clean_text(name),
        image: None,
        build: None,
        container_name: None,
        command: None,
        user: None,
        restart: None,
        ports: Vec::new(),
        volumes: Vec::new(),
        environment_keys: Vec::new(),
        depends_on: Vec::new(),
        networks: Vec::new(),
        profiles: Vec::new(),
        healthcheck_present: false,
    }
}

fn record_service_field(service: &mut ComposeService, field: &str, value: &str) {
    match field {
        "image" => service.image = non_empty_scalar(value),
        "build" => {
            if !value.is_empty() {
                service.build = non_empty_scalar(value);
            }
        }
        "container_name" => service.container_name = non_empty_scalar(value),
        "command" => service.command = non_empty_scalar(value),
        "user" => service.user = non_empty_scalar(value),
        "restart" => service.restart = non_empty_scalar(value),
        "healthcheck" => service.healthcheck_present = true,
        "ports" => {
            for item in parse_inline_list(value) {
                if let Some(port) = parse_compose_port(&item) {
                    service.ports.push(port);
                }
            }
        }
        "volumes" => {
            for item in parse_inline_list(value) {
                service.volumes.push(parse_compose_volume(&item));
            }
        }
        "depends_on" => service.depends_on.extend(parse_inline_list(value)),
        "networks" => service.networks.extend(parse_inline_list(value)),
        "profiles" => service.profiles.extend(parse_inline_list(value)),
        "environment" => service.environment_keys.extend(
            parse_inline_list(value)
                .into_iter()
                .filter_map(|item| environment_key_from_assignment(&item)),
        ),
        _ => {}
    }
    dedupe_service(service);
}

fn record_service_nested(service: &mut ComposeService, field: &str, trimmed: &str) {
    if let Some(item) = trimmed.strip_prefix("- ") {
        let item = parse_scalar(item);
        match field {
            "ports" => {
                if let Some(port) = parse_compose_port(&item) {
                    service.ports.push(port);
                }
            }
            "volumes" => service.volumes.push(parse_compose_volume(&item)),
            "depends_on" => service.depends_on.push(item),
            "networks" => service.networks.push(item),
            "profiles" => service.profiles.push(item),
            "environment" => {
                if let Some(key) = environment_key_from_assignment(&item) {
                    service.environment_keys.push(key);
                }
            }
            "healthcheck" => service.healthcheck_present = true,
            _ => {}
        }
        dedupe_service(service);
        return;
    }

    if let Some((key, value)) = split_key_value(trimmed) {
        match field {
            "depends_on" => service.depends_on.push(key),
            "networks" => service.networks.push(key),
            "environment" => service.environment_keys.push(key),
            "build" if key == "context" => service.build = non_empty_scalar(&value),
            "healthcheck" => service.healthcheck_present = true,
            _ => {}
        }
        dedupe_service(service);
    }
}

fn dedupe_service(service: &mut ComposeService) {
    service.depends_on.sort();
    service.depends_on.dedup();
    service.networks.sort();
    service.networks.dedup();
    service.profiles.sort();
    service.profiles.dedup();
    service.environment_keys.sort();
    service.environment_keys.dedup();
}

fn parse_compose_port(value: &str) -> Option<ComposePortMapping> {
    let raw = value.to_owned();
    let mut protocol = PortProtocol::Tcp;
    let without_protocol = if let Some((left, right)) = value.rsplit_once('/') {
        protocol = if right.eq_ignore_ascii_case("udp") {
            PortProtocol::Udp
        } else {
            PortProtocol::Tcp
        };
        left
    } else {
        value
    };
    let parts = without_protocol.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [container] => Some(ComposePortMapping {
            host_ip: None,
            host_port: None,
            container_port: parse_port(container)?,
            protocol,
            raw,
        }),
        [host, container] => Some(ComposePortMapping {
            host_ip: None,
            host_port: Some(parse_port(host)?),
            container_port: parse_port(container)?,
            protocol,
            raw,
        }),
        [host_ip, host, container] => Some(ComposePortMapping {
            host_ip: Some(clean_text(host_ip)),
            host_port: Some(parse_port(host)?),
            container_port: parse_port(container)?,
            protocol,
            raw,
        }),
        _ => None,
    }
}

fn parse_compose_volume(value: &str) -> ComposeVolumeMount {
    let raw = value.to_owned();
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [target] => ComposeVolumeMount {
            source: None,
            target: Some(clean_text(target)),
            mode: None,
            raw,
        },
        [source, target] => ComposeVolumeMount {
            source: Some(clean_text(source)),
            target: Some(clean_text(target)),
            mode: None,
            raw,
        },
        [source, target, mode, ..] => ComposeVolumeMount {
            source: Some(clean_text(source)),
            target: Some(clean_text(target)),
            mode: Some(clean_text(mode)),
            raw,
        },
        [] => ComposeVolumeMount {
            source: None,
            target: None,
            mode: None,
            raw,
        },
    }
}

fn compose_doctor(
    project: &ComposeProject,
    ports: &[PortEndpoint],
    containers: Option<&[DockerContainer]>,
) -> Vec<ComposeDoctorIssue> {
    let mut issues = Vec::new();
    let services = project
        .services
        .iter()
        .map(|service| (service.name.as_str(), service))
        .collect::<BTreeMap<_, _>>();
    let networks = project
        .networks
        .iter()
        .map(|network| network.name.as_str())
        .collect::<BTreeSet<_>>();
    let volumes = project
        .volumes
        .iter()
        .map(|volume| volume.name.as_str())
        .collect::<BTreeSet<_>>();

    for diagnostic in project
        .parse_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code != "COMPOSE_CANONICAL_CONFIG_UNAVAILABLE")
    {
        issues.push(
            ComposeDoctorIssue::new(
                "INVALID_COMPOSE_SYNTAX",
                DockerIssueSeverity::Warning,
                diagnostic.message.clone(),
            )
            .with_evidence(diagnostic.evidence.join(" ")),
        );
    }

    for variable in &project.unresolved_interpolation {
        issues.push(
            ComposeDoctorIssue::new(
                "UNRESOLVED_ENVIRONMENT_VARIABLE",
                DockerIssueSeverity::Warning,
                format!("`${variable}` is referenced by the Compose file but was not found in registered environment key metadata."),
            )
            .with_remediation("Add the key to an example/active environment file or provide a documented Compose default.")
            .with_evidence(format!("{} references ${{{variable}}}. Values were not read.", project.compose_file)),
        );
    }

    let mut host_ports = BTreeMap::<(String, u16), Vec<String>>::new();
    for service in &project.services {
        for port in &service.ports {
            if let Some(host_port) = port.host_port {
                let host_ip = port.host_ip.clone().unwrap_or_else(|| "0.0.0.0".to_owned());
                host_ports
                    .entry((host_ip, host_port))
                    .or_default()
                    .push(service.name.clone());
            }
        }
    }
    for ((host_ip, host_port), service_names) in
        host_ports.iter().filter(|(_, names)| names.len() > 1)
    {
        issues.push(
            ComposeDoctorIssue::new(
                "DUPLICATE_HOST_PORT",
                DockerIssueSeverity::Error,
                format!("Multiple services publish host port {host_port}."),
            )
            .with_remediation("Choose a unique host port for each service.")
            .with_evidence(format!(
                "{host_ip}:{host_port} is used by {}",
                service_names.join(", ")
            )),
        );
    }

    for service in &project.services {
        for port in &service.ports {
            if let Some(host_port) = port.host_port {
                for endpoint in ports
                    .iter()
                    .filter(|endpoint| endpoint.local_port == host_port)
                {
                    let owner = endpoint.owning_process_name.as_deref().unwrap_or_default();
                    if owner.to_ascii_lowercase().contains("docker") {
                        continue;
                    }
                    issues.push(
                        ComposeDoctorIssue::new(
                            "HOST_PORT_CONFLICT_NON_DOCKER_PROCESS",
                            DockerIssueSeverity::Warning,
                            format!("Compose service `{}` wants host port {host_port}, which is already owned by a non-Docker process.", service.name),
                        )
                        .for_service(service.name.clone())
                        .with_remediation("Stop the conflicting process or change the Compose host port before starting the stack.")
                        .with_evidence(format!(
                            "{}:{} is owned by {}.",
                            endpoint.local_address,
                            endpoint.local_port,
                            owner.if_empty("an unknown process")
                        )),
                    );
                }
            }
        }

        for mount in &service.volumes {
            if let Some(source) = &mount.source
                && is_bind_mount_source(source)
                && !bind_mount_exists(&project.project_root, source)
            {
                issues.push(
                    ComposeDoctorIssue::new(
                        "MISSING_BIND_MOUNT_PATH",
                        DockerIssueSeverity::Error,
                        format!(
                            "Service `{}` references a missing bind-mount path.",
                            service.name
                        ),
                    )
                    .for_service(service.name.clone())
                    .with_remediation("Create the path or update the Compose volume source.")
                    .with_evidence(format!(
                        "{} -> {}",
                        source,
                        mount.target.as_deref().unwrap_or("?")
                    )),
                );
            }
            if let Some(source) = &mount.source
                && is_named_volume(source)
                && !volumes.contains(source.as_str())
            {
                issues.push(
                    ComposeDoctorIssue::new(
                        "VOLUME_REFERENCED_BUT_UNDEFINED",
                        DockerIssueSeverity::Warning,
                        format!("Service `{}` references volume `{source}` but it is not defined in the Compose file.", service.name),
                    )
                    .for_service(service.name.clone())
                    .with_remediation("Define the named volume or mark it as external if that is intended."),
                );
            }
        }

        for dependency in &service.depends_on {
            match services.get(dependency.as_str()) {
                Some(target) if !target.healthcheck_present => {
                    issues.push(
                        ComposeDoctorIssue::new(
                            "DEPENDS_ON_TARGET_WITHOUT_HEALTHCHECK",
                            DockerIssueSeverity::Warning,
                            format!("Service `{}` depends on `{dependency}`, but `{dependency}` has no health check.", service.name),
                        )
                        .for_service(service.name.clone())
                        .with_remediation("Add a healthcheck to the dependency when startup readiness matters.")
                        .with_evidence(format!("{} -> {dependency}", service.name)),
                    );
                }
                Some(_) => {}
                None => {
                    issues.push(
                        ComposeDoctorIssue::new(
                            "SERVICE_DEPENDENCY_ABSENT",
                            DockerIssueSeverity::Error,
                            format!(
                                "Service `{}` depends on missing service `{dependency}`.",
                                service.name
                            ),
                        )
                        .for_service(service.name.clone())
                        .with_remediation("Add the missing service or remove the dependency."),
                    );
                }
            }
        }

        for network in &service.networks {
            if network != "default" && !networks.contains(network.as_str()) {
                issues.push(
                    ComposeDoctorIssue::new(
                        "NETWORK_REFERENCED_BUT_UNDEFINED",
                        DockerIssueSeverity::Warning,
                        format!("Service `{}` references network `{network}` but it is not defined in the Compose file.", service.name),
                    )
                    .for_service(service.name.clone())
                    .with_remediation("Define the network or mark it as external if that is intended."),
                );
            }
        }

        if let Some(image) = &service.image
            && image_uses_latest(image)
        {
            issues.push(
                ComposeDoctorIssue::new(
                    "FLOATING_LATEST_IMAGE_TAG",
                    DockerIssueSeverity::Warning,
                    format!("Service `{}` uses a floating image tag.", service.name),
                )
                .for_service(service.name.clone())
                .with_remediation("Pin images to an explicit version or digest for repeatable local environments.")
                .with_evidence(image.clone()),
            );
        }

        if service
            .user
            .as_deref()
            .is_some_and(|user| matches!(user.trim(), "root" | "0" | "0:0"))
        {
            issues.push(
                ComposeDoctorIssue::new(
                    "CONTAINER_CONFIGURED_AS_ROOT",
                    DockerIssueSeverity::Warning,
                    format!("Service `{}` is configured to run as root.", service.name),
                )
                .for_service(service.name.clone())
                .with_remediation(
                    "Use a non-root user when the image and local workflow support it.",
                ),
            );
        }

        if service_is_database(service) {
            for port in &service.ports {
                if port_is_database(port.container_port)
                    && port
                        .host_ip
                        .as_deref()
                        .map(is_all_interfaces)
                        .unwrap_or(true)
                {
                    issues.push(
                        ComposeDoctorIssue::new(
                            "DATABASE_PORT_EXPOSED_ALL_INTERFACES",
                            DockerIssueSeverity::Warning,
                            format!("Database service `{}` publishes port {} on all interfaces.", service.name, port.container_port),
                        )
                        .for_service(service.name.clone())
                        .with_remediation("Bind development databases to 127.0.0.1 unless LAN access is intentional.")
                        .with_evidence(port.raw.clone()),
                    );
                }
            }
        }

        if common_long_running_service(service) && !service.healthcheck_present {
            issues.push(
                ComposeDoctorIssue::new(
                    "MISSING_HEALTHCHECK",
                    DockerIssueSeverity::Information,
                    format!(
                        "Service `{}` looks long-running and has no health check.",
                        service.name
                    ),
                )
                .for_service(service.name.clone())
                .with_remediation("Consider adding a healthcheck so readiness is visible."),
            );
        }

        if service.restart.as_deref().unwrap_or("no") == "no" {
            issues.push(
                ComposeDoctorIssue::new(
                    "MISSING_RESTART_POLICY",
                    DockerIssueSeverity::Information,
                    format!("Service `{}` has no restart policy.", service.name),
                )
                .for_service(service.name.clone())
                .with_remediation("For long-running local infrastructure, consider `unless-stopped` or document why no restart is preferred."),
            );
        }
    }

    if let Some(containers) = containers {
        for service in &project.services {
            let matching = containers
                .iter()
                .filter(|container| {
                    container.compose_service.as_deref() == Some(service.name.as_str())
                        && container
                            .associated_project
                            .as_ref()
                            .is_some_and(|association| association.project_id == project.project_id)
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                issues.push(
                    ComposeDoctorIssue::new(
                        "PROJECT_SERVICE_EXPECTED_NOT_RUNNING",
                        DockerIssueSeverity::Information,
                        format!("No running or stopped Docker container was associated with service `{}`.", service.name),
                    )
                    .for_service(service.name.clone())
                    .with_evidence("Runtime association uses Docker Compose labels and registered project roots."),
                );
            }
            for container in matching {
                if container.health.as_deref() == Some("unhealthy") {
                    issues.push(
                        ComposeDoctorIssue::new(
                            "CONTAINER_CURRENTLY_UNHEALTHY",
                            DockerIssueSeverity::Error,
                            format!(
                                "Container `{}` for service `{}` is unhealthy.",
                                container.name, service.name
                            ),
                        )
                        .for_service(service.name.clone())
                        .with_remediation(
                            "Open logs and inspect the container healthcheck output.",
                        ),
                    );
                }
            }
        }
    }

    issues
}

fn unresolved_interpolation(text: &str, project: &Project) -> Vec<String> {
    let known_keys = project
        .environment_files
        .iter()
        .flat_map(|file| file.key_names.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut unresolved = BTreeSet::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("${") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find('}') else {
            break;
        };
        let expression = &after_start[..end];
        let variable = expression
            .split([':', '-', '?'])
            .next()
            .unwrap_or_default()
            .trim();
        let has_default = expression.contains(":-") || expression.contains('-');
        if !variable.is_empty()
            && !has_default
            && !known_keys.contains(variable)
            && std::env::var_os(variable).is_none()
        {
            unresolved.insert(variable.to_owned());
        }
        remaining = &after_start[end + 1..];
    }
    unresolved.into_iter().collect()
}

fn validated_compose_path(
    project: &Project,
    compose_file: &str,
) -> Result<(PathBuf, String), DockerError> {
    let relative = PathBuf::from(compose_file);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(DockerError::InvalidComposePath(
            "Compose paths must be relative to the registered project root.".to_owned(),
        ));
    }

    let root = PathBuf::from(&project.root_path);
    let path = root.join(&relative);
    let canonical_root = fs::canonicalize(&root).map_err(DockerError::ComposeIo)?;
    let canonical_path = fs::canonicalize(&path).map_err(DockerError::ComposeIo)?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(DockerError::InvalidComposePath(
            "The Compose file resolves outside the registered project root.".to_owned(),
        ));
    }
    Ok((canonical_path, relative.to_string_lossy().into_owned()))
}

fn read_compose_text(path: &Path) -> Result<String, DockerError> {
    let metadata = path.metadata().map_err(DockerError::ComposeIo)?;
    if !metadata.is_file() {
        return Err(DockerError::InvalidComposePath(
            "The Compose path is not a file.".to_owned(),
        ));
    }
    if metadata.len() > MAX_COMPOSE_BYTES {
        return Err(DockerError::InvalidComposePath(
            "The Compose file is too large to parse safely.".to_owned(),
        ));
    }
    fs::read_to_string(path).map_err(DockerError::ComposeIo)
}

fn compose_project_error(
    project: &Project,
    compose_file: &str,
    error: DockerError,
) -> ComposeProject {
    let issue = DockerDiagnostic::new(
        "COMPOSE_PARSE_FAILED",
        DockerIssueSeverity::Error,
        "Mr Manager could not parse this Compose file.",
    )
    .with_evidence(redaction::redact(&error.to_string()))
    .with_remediation("Verify the file still exists under the registered project root.");
    let mut compose = ComposeProject {
        id: compose_project_id(project, compose_file),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        project_root: project.root_path.clone(),
        compose_file: compose_file.to_owned(),
        source: ComposeParseSource::FallbackParser,
        services: Vec::new(),
        networks: Vec::new(),
        volumes: Vec::new(),
        unresolved_interpolation: Vec::new(),
        parse_diagnostics: vec![issue],
        doctor: Vec::new(),
    };
    compose.doctor = compose_doctor(&compose, &[], None);
    compose
}

fn run_docker(args: &[&str], timeout: Duration) -> Result<DockerCommandOutput, DockerError> {
    let args = args.iter().map(OsString::from).collect::<Vec<_>>();
    run_docker_os(&args, timeout)
}

fn run_docker_os(args: &[OsString], timeout: Duration) -> Result<DockerCommandOutput, DockerError> {
    let mut child = crate::platform::process::hidden_command("docker")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                DockerError::CliMissing
            } else {
                DockerError::Io(error)
            }
        })?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output().map_err(DockerError::Io)?;
                return Ok(DockerCommandOutput {
                    status_success: output.status.success(),
                    status_code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: redaction::redact(&String::from_utf8_lossy(&output.stderr)),
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DockerError::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(DockerError::Io(error)),
        }
    }
}

fn ensure_success(output: &DockerCommandOutput) -> Result<(), DockerError> {
    if output.status_success {
        Ok(())
    } else {
        Err(DockerError::CommandFailed {
            status_code: output.status_code,
            stderr: output.stderr.clone(),
        })
    }
}

fn docker_context() -> Result<String, DockerError> {
    let output = run_docker(&["context", "show"], Duration::from_millis(3_000))?;
    ensure_success(&output)?;
    Ok(clean_text(output.stdout.trim()))
}

fn classify_failed_docker_version(stderr: &str) -> DockerAvailability {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("permission denied") || lower.contains("access is denied") {
        DockerAvailability::Inaccessible
    } else if lower.contains("cannot connect")
        || lower.contains("error during connect")
        || lower.contains("is the docker daemon running")
        || lower.contains("docker daemon")
    {
        DockerAvailability::InstalledStopped
    } else {
        DockerAvailability::Error
    }
}

fn diagnostic_for_failed_version(
    availability: DockerAvailability,
    stderr: &str,
) -> DockerDiagnostic {
    match availability {
        DockerAvailability::InstalledStopped => DockerDiagnostic::new(
            "DOCKER_DAEMON_NOT_REACHABLE",
            DockerIssueSeverity::Information,
            "Docker CLI is installed, but the daemon is not reachable.",
        )
        .with_remediation("Start Docker Desktop and retry.")
        .with_evidence(stderr.chars().take(500).collect::<String>()),
        DockerAvailability::Inaccessible => DockerDiagnostic::new(
            "DOCKER_DAEMON_INACCESSIBLE",
            DockerIssueSeverity::Warning,
            "Docker appears to be installed, but this user cannot access the daemon.",
        )
        .with_remediation("Verify Docker Desktop permissions for this Windows user.")
        .with_evidence(stderr.chars().take(500).collect::<String>()),
        _ => DockerDiagnostic::new(
            "DOCKER_VERSION_FAILED",
            DockerIssueSeverity::Warning,
            "Docker version probing returned an unexpected error.",
        )
        .with_remediation("Open Docker Desktop, verify the current context, and retry.")
        .with_evidence(stderr.chars().take(500).collect::<String>()),
    }
}

fn docker_desktop_process_state(processes: &[ProcessSnapshot]) -> DockerDesktopProcessState {
    if !cfg!(target_os = "windows") {
        return DockerDesktopProcessState::Unknown;
    }
    if processes.iter().any(|process| {
        let lower = process.name.to_ascii_lowercase();
        lower.contains("docker desktop")
            || lower.contains("com.docker.backend")
            || lower == "dockerd.exe"
            || lower == "docker.exe"
    }) {
        DockerDesktopProcessState::Running
    } else {
        DockerDesktopProcessState::NotDetected
    }
}

fn validate_container_reference(value: &str) -> Result<(), DockerError> {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        });
    if valid {
        Ok(())
    } else {
        Err(DockerError::InvalidContainerReference)
    }
}

fn confirmation_name(container: &DockerContainer) -> String {
    if container.name.is_empty() {
        container.short_id.clone()
    } else {
        container.name.clone()
    }
}

fn split_docker_timestamp(line: &str) -> (Option<String>, &str) {
    if let Some((timestamp, rest)) = line.split_once(' ')
        && timestamp.contains('T')
    {
        return (Some(clean_text(timestamp)), rest);
    }
    (None, line)
}

fn parse_warning(line_number: usize, message: impl Into<String>) -> DockerDiagnostic {
    DockerDiagnostic::new(
        "COMPOSE_PARSE_WARNING",
        DockerIssueSeverity::Warning,
        message,
    )
    .with_evidence(format!("Line {}", line_number.saturating_add(1)))
}

fn split_key_value(trimmed: &str) -> Option<(String, String)> {
    let (key, value) = trimmed.split_once(':')?;
    let key = clean_text(key);
    if key.is_empty() {
        return None;
    }
    Some((key, parse_scalar(value)))
}

fn parse_scalar(value: &str) -> String {
    let trimmed = value.trim();
    let without_quotes = if trimmed.len() >= 2 {
        let first = trimmed.chars().next();
        let last = trimmed.chars().last();
        if matches!(
            (first, last),
            (Some('"'), Some('"')) | (Some('\''), Some('\''))
        ) {
            &trimmed[1..trimmed.len().saturating_sub(1)]
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    clean_text(without_quotes)
}

fn non_empty_scalar(value: &str) -> Option<String> {
    let value = parse_scalar(value);
    if value.is_empty() { None } else { Some(value) }
}

fn scalar_bool(value: &str) -> bool {
    matches!(
        parse_scalar(value).to_ascii_lowercase().as_str(),
        "true" | "yes"
    )
}

fn parse_inline_list(value: &str) -> Vec<String> {
    let value = parse_scalar(value);
    if value.is_empty() {
        return Vec::new();
    }
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        trimmed[1..trimmed.len().saturating_sub(1)]
            .split(',')
            .map(parse_scalar)
            .filter(|item| !item.is_empty())
            .collect()
    } else {
        vec![value]
    }
}

fn strip_comment(line: &str) -> Option<String> {
    let mut in_single = false;
    let mut in_double = false;
    for (index, character) in line.char_indices() {
        match character {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return Some(line[..index].trim_end().to_owned()),
            _ => {}
        }
    }
    Some(line.trim_end().to_owned())
}

fn environment_key_from_assignment(value: &str) -> Option<String> {
    let assignment = value.strip_prefix("export ").unwrap_or(value);
    let key = assignment
        .split_once('=')
        .map(|(key, _)| key)
        .unwrap_or(assignment)
        .trim();
    let valid = !key.is_empty()
        && key.len() <= 120
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    valid.then(|| key.to_owned())
}

fn parse_port(value: &str) -> Option<u16> {
    parse_scalar(value).parse::<u16>().ok()
}

fn is_bind_mount_source(source: &str) -> bool {
    source.starts_with('.')
        || source.starts_with('/')
        || source.starts_with('\\')
        || source.contains(":\\")
        || source.contains(":/")
}

fn is_named_volume(source: &str) -> bool {
    !is_bind_mount_source(source)
        && !source.is_empty()
        && source.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
}

fn bind_mount_exists(root: &str, source: &str) -> bool {
    let path = PathBuf::from(source);
    if path.is_absolute() {
        path.exists()
    } else {
        PathBuf::from(root).join(path).exists()
    }
}

fn service_is_database(service: &ComposeService) -> bool {
    let haystack = format!(
        "{} {}",
        service.name,
        service.image.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();
    ["postgres", "mysql", "mariadb", "redis", "mongo"]
        .iter()
        .any(|needle| haystack.contains(needle))
}

fn common_long_running_service(service: &ComposeService) -> bool {
    service_is_database(service)
        || service
            .image
            .as_deref()
            .map(|image| {
                let lower = image.to_ascii_lowercase();
                [
                    "nginx", "node", "redis", "postgres", "mysql", "mariadb", "mongo",
                ]
                .iter()
                .any(|needle| lower.contains(needle))
            })
            .unwrap_or(false)
}

fn port_is_database(port: u16) -> bool {
    matches!(port, 5432 | 3306 | 33060 | 6379 | 27017)
}

fn is_all_interfaces(value: &str) -> bool {
    matches!(value, "0.0.0.0" | "::" | "[::]" | "")
}

fn image_uses_latest(image: &str) -> bool {
    if image.contains('@') {
        return false;
    }
    let last_segment = image.rsplit('/').next().unwrap_or(image);
    !last_segment.contains(':') || last_segment.ends_with(":latest")
}

fn compose_project_id(project: &Project, relative_path: &str) -> String {
    format!("{}:{}", project.id, relative_path.replace('\\', "/"))
}

fn string_field(value: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(clean_text)
        .filter(|value| !value.is_empty())
}

fn clean_text(value: &str) -> String {
    redaction::redact(value)
        .chars()
        .filter(|character| !character.is_control() || matches!(*character, '\n' | '\r' | '\t'))
        .collect::<String>()
        .replace('\r', "")
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn normalize_path_for_compare(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

trait EmptyFallback {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl EmptyFallback for str {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.is_empty() { fallback } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        EnvironmentFileSummary, GitSummary, ProjectIssue, ProjectIssueSeverity, ProjectManifest,
        ProjectManifestKind, ProjectScanHealth, ProjectScanState,
    };

    #[test]
    fn optional_inventory_timeout_is_reported_as_partial_data() {
        let mut status = DockerStatus {
            availability: DockerAvailability::Running,
            cli_detected: true,
            daemon_reachable: true,
            client_version: None,
            server_version: None,
            context: Some("fixture".to_owned()),
            docker_desktop_process: DockerDesktopProcessState::Running,
            collected_at_ms: 1,
            diagnostics: Vec::new(),
        };
        let items = partial_docker_resource::<DockerNetwork>(
            &mut status,
            "networks",
            Err(DockerError::Timeout),
        );
        assert!(items.is_empty());
        assert_eq!(status.availability, DockerAvailability::Running);
        assert_eq!(status.diagnostics[0].code, "DOCKER_PARTIAL_TIMEOUT");
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("fixtures")
            .join("projects")
            .join(name)
    }

    fn compose_project_fixture() -> Project {
        let root = fixture_path("compose-stack");
        Project {
            id: "compose-project".to_owned(),
            name: "compose-stack".to_owned(),
            root_path: root.to_string_lossy().into_owned(),
            canonical_root_path: root.to_string_lossy().into_owned(),
            tags: Vec::new(),
            notes: String::new(),
            checklist: Vec::new(),
            pinned: false,
            archived: false,
            detected_stacks: Vec::new(),
            manifests: vec![ProjectManifest {
                kind: ProjectManifestKind::DockerCompose,
                relative_path: "compose.yaml".to_owned(),
            }],
            package_manager: None,
            scripts: Vec::new(),
            git_summary: Some(GitSummary::not_repository()),
            compose_files: vec!["compose.yaml".to_owned()],
            environment_files: vec![EnvironmentFileSummary {
                relative_path: ".env.example".to_owned(),
                key_names: vec!["DATABASE_URL".to_owned(), "POSTGRES_PASSWORD".to_owned()],
                example: true,
                size_bytes: 48,
            }],
            local_database_hints: Vec::new(),
            last_scanned_at: Some(1),
            scan_health: ProjectScanHealth {
                state: ProjectScanState::Healthy,
                issues: vec![ProjectIssue::new(
                    "FIXTURE",
                    ProjectIssueSeverity::Information,
                    "Fixture only.",
                )],
            },
        }
    }

    #[test]
    fn fallback_parser_reads_fixture_services_ports_and_sections() {
        let project = compose_project_fixture();
        let text = fs::read_to_string(fixture_path("compose-stack").join("compose.yaml"))
            .expect("fixture should read");
        let parsed = parse_compose_text(&project, "compose.yaml", &text);

        assert_eq!(parsed.services.len(), 3);
        assert!(parsed.services.iter().any(|service| service.name == "api"));
        assert!(
            parsed
                .networks
                .iter()
                .any(|network| network.name == "appnet")
        );
        assert!(parsed.volumes.iter().any(|volume| volume.name == "pgdata"));
        assert!(
            parsed
                .unresolved_interpolation
                .iter()
                .any(|key| key == "API_KEY")
        );
        assert!(
            parsed
                .services
                .iter()
                .flat_map(|service| service.ports.iter())
                .any(|port| port.host_port == Some(8080))
        );
    }

    #[test]
    fn compose_doctor_reports_deterministic_fixture_issues() {
        let project = compose_project_fixture();
        let parsed = parse_compose_project(&project, "compose.yaml", &[], None)
            .expect("fixture compose should parse");
        let codes = parsed
            .doctor
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<BTreeSet<_>>();

        assert!(codes.contains("DUPLICATE_HOST_PORT"));
        assert!(codes.contains("UNRESOLVED_ENVIRONMENT_VARIABLE"));
        assert!(codes.contains("MISSING_BIND_MOUNT_PATH"));
        assert!(codes.contains("FLOATING_LATEST_IMAGE_TAG"));
        assert!(codes.contains("DEPENDS_ON_TARGET_WITHOUT_HEALTHCHECK"));
        assert!(codes.contains("DATABASE_PORT_EXPOSED_ALL_INTERFACES"));
    }

    #[test]
    fn compose_port_parser_handles_host_ip_and_protocol() {
        let parsed = parse_compose_port("127.0.0.1:5353:53/udp").expect("port should parse");

        assert_eq!(parsed.host_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(parsed.host_port, Some(5353));
        assert_eq!(parsed.container_port, 53);
        assert_eq!(parsed.protocol, PortProtocol::Udp);
    }
}
