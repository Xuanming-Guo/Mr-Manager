use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::domain::{
    EvidenceConfidence, FeatureAvailability, IntegrationCategory, IntegrationEndpoint,
    IntegrationEvidence, IntegrationInstalledState, IntegrationProcessRef, IntegrationRunningState,
    IntegrationStatus, OllamaModel, OllamaStatus, PortEndpoint, ProcessSnapshot, WslDistribution,
    WslStatus,
};
use crate::security::redaction;

const VERSION_TIMEOUT: Duration = Duration::from_millis(2_500);
const WSL_TIMEOUT: Duration = Duration::from_millis(3_000);
const OLLAMA_TIMEOUT: Duration = Duration::from_millis(800);
const MAX_HTTP_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("unknown integration detector: {0}")]
    UnknownDetector(String),
    #[error("the command timed out")]
    Timeout,
    #[error("the command could not run")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone)]
struct Detector {
    id: &'static str,
    display_name: &'static str,
    category: IntegrationCategory,
    executables: &'static [&'static str],
    version_args: Option<&'static [&'static str]>,
    process_names: &'static [&'static str],
    ports: &'static [u16],
    capabilities: &'static [&'static str],
}

#[derive(Debug)]
struct CommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

pub fn list_integrations(
    processes: &[ProcessSnapshot],
    ports: &[PortEndpoint],
) -> Vec<IntegrationStatus> {
    detectors()
        .iter()
        .map(|detector| inspect_detector(detector, processes, ports))
        .collect()
}

pub fn probe_integration(
    detector_id: &str,
    processes: &[ProcessSnapshot],
    ports: &[PortEndpoint],
) -> Result<IntegrationStatus, IntegrationError> {
    let detector = detectors()
        .iter()
        .find(|detector| detector.id == detector_id)
        .ok_or_else(|| IntegrationError::UnknownDetector(detector_id.to_owned()))?;
    Ok(inspect_detector(detector, processes, ports))
}

pub fn get_ollama_status(processes: &[ProcessSnapshot]) -> OllamaStatus {
    let now = unix_time_ms();
    let process_refs = process_refs(processes, &["ollama"]);
    let executable_paths = find_executables(&["ollama", "ollama.exe"]);
    let mut evidence = Vec::new();
    let mut errors = Vec::new();

    if !executable_paths.is_empty() {
        evidence.push(IntegrationEvidence {
            source: "PATH".to_owned(),
            detail: format!(
                "Found Ollama executable at {}",
                executable_paths[0].display()
            ),
            confidence: EvidenceConfidence::Strong,
        });
    }
    if !process_refs.is_empty() {
        evidence.push(IntegrationEvidence {
            source: "process".to_owned(),
            detail: format!("{} Ollama process(es) observed locally", process_refs.len()),
            confidence: EvidenceConfidence::Strong,
        });
    }

    let endpoint = "http://127.0.0.1:11434";
    let version = match ollama_json("/api/version") {
        Ok(value) => {
            evidence.push(IntegrationEvidence {
                source: "loopback-api".to_owned(),
                detail: "Ollama answered /api/version on 127.0.0.1:11434".to_owned(),
                confidence: EvidenceConfidence::Certain,
            });
            value
                .get("version")
                .and_then(JsonValue::as_str)
                .map(clean_version)
        }
        Err(error) => {
            if !process_refs.is_empty() || !executable_paths.is_empty() {
                errors.push(format!("Loopback API unavailable: {error}"));
            }
            None
        }
    };

    let installed_models = match ollama_json("/api/tags") {
        Ok(value) => ollama_models_from_value(&value, false),
        Err(error) => {
            if version.is_some() {
                errors.push(format!("Installed models unavailable: {error}"));
            }
            Vec::new()
        }
    };
    let running_models = match ollama_json("/api/ps") {
        Ok(value) => ollama_models_from_value(&value, true),
        Err(error) => {
            if version.is_some() {
                errors.push(format!("Running models unavailable: {error}"));
            }
            Vec::new()
        }
    };

    let availability = if version.is_some() {
        FeatureAvailability::available("Ollama loopback API responded locally.")
    } else if !process_refs.is_empty() || !executable_paths.is_empty() {
        FeatureAvailability::unavailable(
            "Ollama was detected locally, but the loopback API is not responding.",
        )
    } else {
        FeatureAvailability::unavailable("Ollama was not detected on PATH or in running processes.")
    };

    OllamaStatus {
        availability,
        endpoint: version.as_ref().map(|_| endpoint.to_owned()),
        version,
        installed_models,
        running_models,
        processes: process_refs,
        evidence,
        last_checked_at_ms: now,
        errors,
    }
}

pub fn get_wsl_status() -> WslStatus {
    let now = unix_time_ms();
    match run_command(Path::new("wsl.exe"), &["--list", "--verbose"], WSL_TIMEOUT) {
        Ok(output) if output.success => {
            let text = strip_nuls(&output.stdout);
            let distros = parse_wsl_list_verbose(&text);
            let availability = if distros.is_empty() {
                FeatureAvailability::unavailable(
                    "WSL is installed, but no distributions were listed.",
                )
            } else {
                FeatureAvailability::available(format!(
                    "WSL listed {} distribution(s) through a read-only adapter.",
                    distros.len()
                ))
            };
            WslStatus {
                availability,
                distros,
                evidence: vec![IntegrationEvidence {
                    source: "wsl.exe".to_owned(),
                    detail: "Ran exact read-only command: wsl.exe --list --verbose".to_owned(),
                    confidence: EvidenceConfidence::Certain,
                }],
                last_checked_at_ms: now,
                errors: Vec::new(),
            }
        }
        Ok(output) => WslStatus {
            availability: FeatureAvailability::unavailable(
                "WSL did not return distribution state for this Windows user.",
            ),
            distros: Vec::new(),
            evidence: vec![IntegrationEvidence {
                source: "wsl.exe".to_owned(),
                detail: "wsl.exe was reachable but returned a non-zero exit code.".to_owned(),
                confidence: EvidenceConfidence::Strong,
            }],
            last_checked_at_ms: now,
            errors: vec![redaction::redact(&strip_nuls(&output.stderr))],
        },
        Err(IntegrationError::Timeout) => WslStatus {
            availability: FeatureAvailability::error(
                "WSL state probing timed out.",
                "Retry after any WSL startup or shutdown operation completes.",
            ),
            distros: Vec::new(),
            evidence: Vec::new(),
            last_checked_at_ms: now,
            errors: vec!["wsl.exe --list --verbose timed out".to_owned()],
        },
        Err(error) => WslStatus {
            availability: FeatureAvailability::unavailable(
                "WSL is not installed or wsl.exe is not available for this Windows user.",
            ),
            distros: Vec::new(),
            evidence: Vec::new(),
            last_checked_at_ms: now,
            errors: vec![error.to_string()],
        },
    }
}

fn inspect_detector(
    detector: &Detector,
    processes: &[ProcessSnapshot],
    ports: &[PortEndpoint],
) -> IntegrationStatus {
    let now = unix_time_ms();
    let executable_paths = find_executables(detector.executables);
    let process_refs = process_refs(processes, detector.process_names);
    let endpoints = endpoints_for_detector(ports, detector);
    let mut evidence = Vec::new();
    let mut errors = Vec::new();

    if !executable_paths.is_empty() {
        evidence.push(IntegrationEvidence {
            source: "PATH".to_owned(),
            detail: format!(
                "Found {} candidate executable(s); first: {}",
                executable_paths.len(),
                executable_paths[0].display()
            ),
            confidence: EvidenceConfidence::Strong,
        });
    }
    if !process_refs.is_empty() {
        evidence.push(IntegrationEvidence {
            source: "process".to_owned(),
            detail: format!("{} matching process(es) are running", process_refs.len()),
            confidence: EvidenceConfidence::Strong,
        });
    }
    if !endpoints.is_empty() {
        evidence.push(IntegrationEvidence {
            source: "owned-port-table".to_owned(),
            detail: format!(
                "{} local endpoint(s) match detector evidence",
                endpoints.len()
            ),
            confidence: EvidenceConfidence::Inferred,
        });
    }

    let version = executable_paths
        .first()
        .and_then(|path| version_for(detector, path, &mut errors));

    let installed_state = if !executable_paths.is_empty() {
        IntegrationInstalledState::Installed
    } else if !process_refs.is_empty() || !endpoints.is_empty() {
        IntegrationInstalledState::Unknown
    } else {
        IntegrationInstalledState::NotFound
    };
    let running_state = if !process_refs.is_empty() || !endpoints.is_empty() {
        IntegrationRunningState::Running
    } else if installed_state == IntegrationInstalledState::Installed {
        IntegrationRunningState::Stopped
    } else {
        IntegrationRunningState::Unknown
    };

    IntegrationStatus {
        detector_id: detector.id.to_owned(),
        display_name: detector.display_name.to_owned(),
        category: detector.category.clone(),
        installed_state,
        running_state,
        version,
        executable_paths: executable_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        processes: process_refs,
        endpoints,
        capabilities: detector
            .capabilities
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        evidence,
        last_checked_at_ms: now,
        errors,
    }
}

fn version_for(
    detector: &Detector,
    executable_path: &Path,
    errors: &mut Vec<String>,
) -> Option<String> {
    let args = detector.version_args?;
    if executable_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "cmd" | "bat"))
    {
        errors.push(
            "Version probe skipped because the detected launcher is a .cmd/.bat shim; Mr Manager does not invoke shell scripts here."
                .to_owned(),
        );
        return None;
    }

    match run_command(executable_path, args, VERSION_TIMEOUT) {
        Ok(output) if output.success => first_version_line(&output.stdout),
        Ok(output) => {
            let stderr = redaction::redact(&output.stderr);
            if !stderr.is_empty() {
                errors.push(format!("Version command failed: {stderr}"));
            }
            None
        }
        Err(IntegrationError::Timeout) => {
            errors.push("Version command timed out.".to_owned());
            None
        }
        Err(error) => {
            errors.push(format!("Version command could not run: {error}"));
            None
        }
    }
}

fn endpoints_for_detector(ports: &[PortEndpoint], detector: &Detector) -> Vec<IntegrationEndpoint> {
    ports
        .iter()
        .filter(|endpoint| {
            let port_match = detector.ports.contains(&endpoint.local_port);
            let process_match = endpoint
                .owning_process_name
                .as_ref()
                .is_some_and(|name| name_matches(name, detector.process_names));
            port_match || process_match
        })
        .take(12)
        .map(IntegrationEndpoint::from)
        .collect()
}

fn process_refs(
    processes: &[ProcessSnapshot],
    process_names: &[&'static str],
) -> Vec<IntegrationProcessRef> {
    processes
        .iter()
        .filter(|process| name_matches(&process.name, process_names))
        .take(24)
        .map(|process| IntegrationProcessRef {
            key: process.key,
            name: process.name.clone(),
            executable_path: process.executable_path.clone(),
        })
        .collect()
}

fn name_matches(name: &str, process_names: &[&'static str]) -> bool {
    let normalized = normalize_name(name);
    process_names
        .iter()
        .any(|candidate| normalized == normalize_name(candidate))
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_ascii_lowercase()
}

fn find_executables(names: &[&'static str]) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();
    let path_env = env::var_os("PATH").unwrap_or_default();
    let pathext = env::var_os("PATHEXT")
        .map(|value| {
            value
                .to_string_lossy()
                .split(';')
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![".exe".to_owned(), ".cmd".to_owned(), ".bat".to_owned()]);

    for directory in env::split_paths(&path_env) {
        for name in names {
            let candidate = directory.join(name);
            if candidate.is_file() {
                found.insert(candidate);
            }
            if Path::new(name).extension().is_none() {
                for extension in &pathext {
                    let candidate = directory.join(format!("{name}{extension}"));
                    if candidate.is_file() {
                        found.insert(candidate);
                    }
                }
            }
        }
    }

    found.into_iter().take(8).collect()
}

fn run_command(
    executable: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandOutput, IntegrationError> {
    let mut child = crate::platform::process::hidden_command(executable)
        .args(args.iter().map(OsString::from))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output()?;
                return Ok(CommandOutput {
                    success: output.status.success(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: redaction::redact(&String::from_utf8_lossy(&output.stderr)),
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(IntegrationError::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(IntegrationError::Io(error)),
        }
    }
}

fn ollama_json(path: &str) -> Result<JsonValue, String> {
    let body = loopback_http_get("127.0.0.1:11434", path, OLLAMA_TIMEOUT)?;
    serde_json::from_str::<JsonValue>(&body).map_err(|error| format!("invalid JSON: {error}"))
}

fn loopback_http_get(address: &str, path: &str, timeout: Duration) -> Result<String, String> {
    let socket: SocketAddr = address
        .parse()
        .map_err(|error| format!("invalid loopback address: {error}"))?;
    if !socket.ip().is_loopback() {
        return Err("refusing to contact a non-loopback Ollama endpoint".to_owned());
    }
    let mut stream =
        TcpStream::connect_timeout(&socket, timeout).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    let mut limited = stream.take(MAX_HTTP_BYTES as u64);
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let response = String::from_utf8_lossy(&bytes);
    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .ok_or_else(|| "HTTP response body missing".to_owned())?;
    Ok(body.to_owned())
}

fn ollama_models_from_value(value: &JsonValue, loaded: bool) -> Vec<OllamaModel> {
    value
        .get("models")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .take(300)
        .map(|model| {
            let details = model.get("details").unwrap_or(&JsonValue::Null);
            OllamaModel {
                name: string_field(model, "name").unwrap_or_else(|| {
                    string_field(model, "model").unwrap_or_else(|| "unknown".to_owned())
                }),
                model: string_field(model, "model"),
                size_bytes: u64_field(model, "size"),
                digest: string_field(model, "digest"),
                modified_at: string_field(model, "modified_at"),
                format: string_field(details, "format"),
                family: string_field(details, "family"),
                parameter_size: string_field(details, "parameter_size"),
                quantization_level: string_field(details, "quantization_level"),
                loaded,
                expires_at: string_field(model, "expires_at"),
                size_vram_bytes: u64_field(model, "size_vram"),
            }
        })
        .collect()
}

fn string_field(value: &JsonValue, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .map(clean_version)
}

fn u64_field(value: &JsonValue, key: &str) -> Option<u64> {
    value.get(key).and_then(JsonValue::as_u64)
}

fn parse_wsl_list_verbose(text: &str) -> Vec<WslDistribution> {
    text.lines()
        .filter_map(|line| {
            let normalized = line.trim().trim_start_matches('*').trim();
            if normalized.is_empty() || normalized.to_ascii_lowercase().starts_with("name") {
                return None;
            }
            let parts = normalized.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 3 {
                return None;
            }
            let version = parts.last().and_then(|value| value.parse::<u8>().ok());
            let state = parts.get(parts.len().saturating_sub(2))?.to_string();
            let name = parts[..parts.len().saturating_sub(2)].join(" ");
            Some(WslDistribution {
                name,
                state,
                version,
            })
        })
        .collect()
}

fn strip_nuls(value: &str) -> String {
    value.replace('\0', "")
}

fn first_version_line(value: &str) -> Option<String> {
    value
        .lines()
        .map(clean_version)
        .find(|line| !line.is_empty())
}

fn clean_version(value: &str) -> String {
    let mut text = value.trim().replace('\0', "");
    text.truncate(180);
    redaction::redact(&text)
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn detectors() -> &'static [Detector] {
    &[
        Detector {
            id: "git",
            display_name: "Git",
            category: IntegrationCategory::Runtime,
            executables: &["git", "git.exe"],
            version_args: Some(&["--version"]),
            process_names: &["git"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "vscode",
            display_name: "Visual Studio Code",
            category: IntegrationCategory::Editor,
            executables: &["code", "code.exe"],
            version_args: Some(&["--version"]),
            process_names: &["code", "code-insiders"],
            ports: &[],
            capabilities: &["path", "process"],
        },
        Detector {
            id: "docker",
            display_name: "Docker CLI / Engine",
            category: IntegrationCategory::Container,
            executables: &["docker", "docker.exe"],
            version_args: Some(&["--version"]),
            process_names: &["docker", "dockerd", "docker desktop", "com.docker.backend"],
            ports: &[],
            capabilities: &["version", "path", "process"],
        },
        Detector {
            id: "docker-compose",
            display_name: "Docker Compose",
            category: IntegrationCategory::Container,
            executables: &["docker-compose", "docker-compose.exe"],
            version_args: Some(&["--version"]),
            process_names: &["docker-compose"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "ollama",
            display_name: "Ollama",
            category: IntegrationCategory::LocalAi,
            executables: &["ollama", "ollama.exe"],
            version_args: Some(&["--version"]),
            process_names: &["ollama"],
            ports: &[11434],
            capabilities: &["version", "loopback-api", "models"],
        },
        Detector {
            id: "wsl",
            display_name: "Windows Subsystem for Linux",
            category: IntegrationCategory::Shell,
            executables: &["wsl.exe"],
            version_args: Some(&["--version"]),
            process_names: &["wsl", "wslhost", "wslservice"],
            ports: &[],
            capabilities: &["distros", "state"],
        },
        Detector {
            id: "node",
            display_name: "Node.js",
            category: IntegrationCategory::Runtime,
            executables: &["node", "node.exe"],
            version_args: Some(&["--version"]),
            process_names: &["node"],
            ports: &[3000, 5173, 1420, 8080],
            capabilities: &["version", "path", "local-dev-server"],
        },
        Detector {
            id: "npm",
            display_name: "npm",
            category: IntegrationCategory::PackageManager,
            executables: &["npm", "npm.cmd"],
            version_args: Some(&["--version"]),
            process_names: &["npm", "node"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "pnpm",
            display_name: "pnpm",
            category: IntegrationCategory::PackageManager,
            executables: &["pnpm", "pnpm.cmd"],
            version_args: Some(&["--version"]),
            process_names: &["pnpm", "node"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "yarn",
            display_name: "Yarn",
            category: IntegrationCategory::PackageManager,
            executables: &["yarn", "yarn.cmd"],
            version_args: Some(&["--version"]),
            process_names: &["yarn", "node"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "bun",
            display_name: "Bun",
            category: IntegrationCategory::Runtime,
            executables: &["bun", "bun.exe"],
            version_args: Some(&["--version"]),
            process_names: &["bun"],
            ports: &[3000, 5173],
            capabilities: &["version", "path", "local-dev-server"],
        },
        Detector {
            id: "python",
            display_name: "Python",
            category: IntegrationCategory::Runtime,
            executables: &["python", "python.exe", "py.exe"],
            version_args: Some(&["--version"]),
            process_names: &["python", "py"],
            ports: &[5000, 8000, 8080],
            capabilities: &["version", "path", "local-dev-server"],
        },
        Detector {
            id: "pip",
            display_name: "pip",
            category: IntegrationCategory::PackageManager,
            executables: &["pip", "pip.exe"],
            version_args: Some(&["--version"]),
            process_names: &["pip", "python"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "uv",
            display_name: "uv",
            category: IntegrationCategory::PackageManager,
            executables: &["uv", "uv.exe"],
            version_args: Some(&["--version"]),
            process_names: &["uv"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "poetry",
            display_name: "Poetry",
            category: IntegrationCategory::PackageManager,
            executables: &["poetry", "poetry.exe"],
            version_args: Some(&["--version"]),
            process_names: &["poetry", "python"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "rust",
            display_name: "Rust / Cargo",
            category: IntegrationCategory::Runtime,
            executables: &["cargo", "cargo.exe", "rustc", "rustc.exe"],
            version_args: Some(&["--version"]),
            process_names: &["cargo", "rustc"],
            ports: &[],
            capabilities: &["version", "path"],
        },
        Detector {
            id: "go",
            display_name: "Go",
            category: IntegrationCategory::Runtime,
            executables: &["go", "go.exe"],
            version_args: Some(&["version"]),
            process_names: &["go"],
            ports: &[8080],
            capabilities: &["version", "path", "local-dev-server"],
        },
        Detector {
            id: "java",
            display_name: "Java",
            category: IntegrationCategory::Runtime,
            executables: &["java", "java.exe"],
            version_args: Some(&["--version"]),
            process_names: &["java", "javaw"],
            ports: &[8080, 8081],
            capabilities: &["version", "path", "local-dev-server"],
        },
        Detector {
            id: "postgresql",
            display_name: "PostgreSQL",
            category: IntegrationCategory::Database,
            executables: &["psql", "psql.exe", "postgres", "postgres.exe"],
            version_args: Some(&["--version"]),
            process_names: &["postgres", "pg_ctl"],
            ports: &[5432],
            capabilities: &["version", "path", "local-service"],
        },
        Detector {
            id: "mysql",
            display_name: "MySQL / MariaDB",
            category: IntegrationCategory::Database,
            executables: &["mysql", "mysql.exe", "mariadb", "mariadb.exe"],
            version_args: Some(&["--version"]),
            process_names: &["mysql", "mysqld", "mariadbd"],
            ports: &[3306],
            capabilities: &["version", "path", "local-service"],
        },
        Detector {
            id: "redis",
            display_name: "Redis",
            category: IntegrationCategory::Database,
            executables: &[
                "redis-server",
                "redis-server.exe",
                "redis-cli",
                "redis-cli.exe",
            ],
            version_args: Some(&["--version"]),
            process_names: &["redis-server"],
            ports: &[6379],
            capabilities: &["version", "path", "local-service"],
        },
        Detector {
            id: "mongodb",
            display_name: "MongoDB",
            category: IntegrationCategory::Database,
            executables: &["mongod", "mongod.exe", "mongosh", "mongosh.exe"],
            version_args: Some(&["--version"]),
            process_names: &["mongod", "mongosh"],
            ports: &[27017],
            capabilities: &["version", "path", "local-service"],
        },
        Detector {
            id: "vpn-clients",
            display_name: "VPN clients",
            category: IntegrationCategory::Vpn,
            executables: &["tailscale.exe", "wireguard.exe", "openvpn.exe"],
            version_args: None,
            process_names: &[
                "tailscale",
                "tailscaled",
                "wireguard",
                "openvpn",
                "nordvpn",
                "protonvpn",
                "expressvpn",
                "surfshark",
                "zerotier",
                "cloudflared",
            ],
            ports: &[],
            capabilities: &["process-evidence", "probable-vpn-signal"],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_verbose_parser_handles_current_marker_and_names() {
        let distros = parse_wsl_list_verbose(
            "  NAME                   STATE           VERSION\n* Ubuntu                 Running         2\n  Debian Test            Stopped         1\n",
        );
        assert_eq!(distros.len(), 2);
        assert_eq!(distros[0].name, "Ubuntu");
        assert_eq!(distros[0].state, "Running");
        assert_eq!(distros[0].version, Some(2));
        assert_eq!(distros[1].name, "Debian Test");
    }

    #[test]
    fn ollama_model_parser_reads_installed_and_running_metadata() {
        let value = serde_json::json!({
            "models": [{
                "name": "llama3:latest",
                "model": "llama3:latest",
                "size": 123,
                "digest": "abc",
                "modified_at": "2026-01-01T00:00:00Z",
                "details": {
                    "format": "gguf",
                    "family": "llama",
                    "parameter_size": "8B",
                    "quantization_level": "Q4_0"
                }
            }]
        });
        let models = ollama_models_from_value(&value, false);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "llama3:latest");
        assert_eq!(models[0].size_bytes, Some(123));
        assert!(!models[0].loaded);
    }

    #[test]
    fn version_line_is_bounded_and_non_empty() {
        assert_eq!(
            first_version_line("\nnode v22.0.0\nextra"),
            Some("node v22.0.0".to_owned())
        );
    }
}
