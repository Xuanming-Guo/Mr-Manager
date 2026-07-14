use std::collections::{HashMap, VecDeque};
use std::io;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use thiserror::Error;
use uuid::Uuid;

use crate::collector::CollectedSnapshot;
use crate::domain::{
    AppSettings, CollectorDiagnostics, CorrelationFinding, DockerActivitySnapshot,
    FeatureAvailability, GpuAdapterSnapshot, GpuSnapshot, LocalDevServerSnapshot,
    MetricRecordingDetail, MetricRecordingExport, NetworkDashboardSnapshot, ProcessKey,
    ProcessSnapshot, RankedProcess, RankedProcesses, RecordingAnnotation, RecordingSample,
    RecordingSessionSummary, RecordingStatus, RecordingSystemSample, StartMetricRecordingRequest,
    SystemDiagnosticsSnapshot,
};
use crate::security::redaction;

const MAX_ACTIVE_SAMPLES: usize = 900;
const MAX_PERSISTED_SAMPLES: usize = 600;
const MAX_SESSION_NAME: usize = 80;
const MAX_ANNOTATION_LABEL: usize = 300;
const NVIDIA_SMI_TIMEOUT: Duration = Duration::from_millis(1_500);

#[derive(Debug, Error)]
pub enum SystemDiagnosticsError {
    #[error("a metric recording is already active")]
    RecordingAlreadyActive,
    #[error("no metric recording is active")]
    NoActiveRecording,
    #[error("recording name is invalid")]
    InvalidRecordingName,
    #[error("annotation label is invalid")]
    InvalidAnnotation,
    #[error("GPU provider command could not run")]
    Io(#[from] io::Error),
    #[error("GPU provider command timed out")]
    Timeout,
}

#[derive(Debug, Clone)]
struct ActiveRecording {
    id: String,
    name: String,
    started_at_ms: u64,
    samples: VecDeque<RecordingSample>,
    annotations: Vec<RecordingAnnotation>,
    next_sequence: u64,
    downsampled: bool,
}

#[derive(Debug, Default)]
pub struct SystemDiagnosticsManager {
    active: Option<ActiveRecording>,
    dropped_recording_samples: u64,
}

impl SystemDiagnosticsManager {
    pub fn start(
        &mut self,
        request: StartMetricRecordingRequest,
    ) -> Result<RecordingSessionSummary, SystemDiagnosticsError> {
        if self.active.is_some() {
            return Err(SystemDiagnosticsError::RecordingAlreadyActive);
        }
        let name = validate_name(&request.name)?;
        let active = ActiveRecording {
            id: Uuid::new_v4().to_string(),
            name,
            started_at_ms: unix_time_ms(),
            samples: VecDeque::new(),
            annotations: Vec::new(),
            next_sequence: 0,
            downsampled: false,
        };
        let summary = summary_for_active(&active, Vec::new());
        self.active = Some(active);
        Ok(summary)
    }

    pub fn add_annotation(
        &mut self,
        label: &str,
    ) -> Result<RecordingAnnotation, SystemDiagnosticsError> {
        let active = self
            .active
            .as_mut()
            .ok_or(SystemDiagnosticsError::NoActiveRecording)?;
        let label = validate_annotation(label)?;
        let annotation = RecordingAnnotation {
            id: Uuid::new_v4().to_string(),
            at_ms: unix_time_ms(),
            label,
        };
        active.annotations.push(annotation.clone());
        Ok(annotation)
    }

    pub fn capture_sample(&mut self, mut sample: RecordingSample) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        sample.sequence = active.next_sequence;
        active.next_sequence = active.next_sequence.wrapping_add(1);
        if active.samples.len() >= MAX_ACTIVE_SAMPLES {
            active.samples.pop_front();
            active.downsampled = true;
            self.dropped_recording_samples = self.dropped_recording_samples.saturating_add(1);
        }
        active.samples.push_back(sample);
    }

    pub fn active_summary(&self) -> Option<RecordingSessionSummary> {
        self.active
            .as_ref()
            .map(|active| summary_for_active(active, analyze_samples(&active.samples)))
    }

    pub fn active_detail(&self) -> Option<MetricRecordingDetail> {
        self.active.as_ref().map(|active| {
            let samples = active.samples.iter().cloned().collect::<Vec<_>>();
            MetricRecordingDetail {
                summary: summary_for_active(active, analyze_samples(&active.samples)),
                samples,
                annotations: active.annotations.clone(),
            }
        })
    }

    pub fn stop(&mut self) -> Result<MetricRecordingDetail, SystemDiagnosticsError> {
        let active = self
            .active
            .take()
            .ok_or(SystemDiagnosticsError::NoActiveRecording)?;
        Ok(completed_detail(active))
    }

    pub fn delete_active_if_matches(&mut self, session_id: &str) -> bool {
        let should_delete = self
            .active
            .as_ref()
            .is_some_and(|active| active.id == session_id);
        if should_delete {
            self.active = None;
        }
        should_delete
    }

    pub const fn dropped_recording_samples(&self) -> u64 {
        self.dropped_recording_samples
    }
}

pub fn build_system_diagnostics_snapshot(
    collected: CollectedSnapshot,
    network: NetworkDashboardSnapshot,
    settings: AppSettings,
    manager: &mut SystemDiagnosticsManager,
    collection_duration_ms: u64,
) -> SystemDiagnosticsSnapshot {
    let gpu = probe_gpu();
    let ranked_processes = ranked_processes(&collected.processes);
    let docker = docker_activity(&collected.processes);
    let local_dev_servers = local_dev_servers(&collected.ports);
    let diagnostics = CollectorDiagnostics {
        collection_duration_ms,
        process_count: u32::try_from(collected.processes.len()).unwrap_or(u32::MAX),
        port_count: u32::try_from(collected.ports.len()).unwrap_or(u32::MAX),
        adapter_count: u32::try_from(network.adapters.len()).unwrap_or(u32::MAX),
        dropped_recording_samples: manager.dropped_recording_samples(),
        warnings: collector_warnings(&collected, &gpu),
    };

    let sample = recording_sample(
        &collected,
        &network,
        &gpu,
        &ranked_processes,
        &docker,
        &local_dev_servers,
    );
    manager.capture_sample(sample);
    let active_recording = manager.active_summary();
    let recent_findings = active_recording
        .as_ref()
        .map(|summary| summary.findings.clone())
        .unwrap_or_default();

    SystemDiagnosticsSnapshot {
        collected_at_ms: collected.overview.system.collected_at_ms,
        refresh_mode: settings.refresh_mode,
        system: collected.overview.system,
        gpu,
        ranked_processes,
        network,
        docker,
        local_dev_servers,
        collector_diagnostics: diagnostics,
        active_recording,
        recent_findings,
    }
}

pub fn export_detail(detail: MetricRecordingDetail) -> MetricRecordingExport {
    MetricRecordingExport {
        exported_at_ms: unix_time_ms(),
        redacted_by_default: true,
        detail,
    }
}

fn completed_detail(active: ActiveRecording) -> MetricRecordingDetail {
    let mut samples = active.samples.iter().cloned().collect::<Vec<_>>();
    let mut downsampled = active.downsampled;
    if samples.len() > MAX_PERSISTED_SAMPLES {
        samples = downsample_samples(&samples, MAX_PERSISTED_SAMPLES);
        downsampled = true;
    }
    let findings = analyze_slice(&samples);
    let local_only = samples
        .iter()
        .all(|sample| sample.local_only && !sample.included_internet_diagnostics);
    MetricRecordingDetail {
        summary: RecordingSessionSummary {
            id: active.id,
            name: active.name,
            status: RecordingStatus::Completed,
            started_at_ms: active.started_at_ms,
            stopped_at_ms: Some(unix_time_ms()),
            sample_count: u32::try_from(samples.len()).unwrap_or(u32::MAX),
            annotation_count: u32::try_from(active.annotations.len()).unwrap_or(u32::MAX),
            downsampled,
            local_only,
            findings,
        },
        samples,
        annotations: active.annotations,
    }
}

fn summary_for_active(
    active: &ActiveRecording,
    findings: Vec<CorrelationFinding>,
) -> RecordingSessionSummary {
    RecordingSessionSummary {
        id: active.id.clone(),
        name: active.name.clone(),
        status: RecordingStatus::Active,
        started_at_ms: active.started_at_ms,
        stopped_at_ms: None,
        sample_count: u32::try_from(active.samples.len()).unwrap_or(u32::MAX),
        annotation_count: u32::try_from(active.annotations.len()).unwrap_or(u32::MAX),
        downsampled: active.downsampled,
        local_only: active
            .samples
            .iter()
            .all(|sample| sample.local_only && !sample.included_internet_diagnostics),
        findings,
    }
}

fn recording_sample(
    collected: &CollectedSnapshot,
    network: &NetworkDashboardSnapshot,
    gpu: &GpuSnapshot,
    ranked: &RankedProcesses,
    docker: &DockerActivitySnapshot,
    local_dev_servers: &[LocalDevServerSnapshot],
) -> RecordingSample {
    let processes = combined_top_processes(ranked);
    RecordingSample {
        sequence: 0,
        collected_at_ms: collected.overview.system.collected_at_ms,
        local_only: true,
        included_internet_diagnostics: false,
        system: RecordingSystemSample {
            cpu_total_percent: collected.overview.system.cpu.total_usage_percent,
            memory: collected.overview.system.memory.clone(),
            battery: collected.overview.system.battery.clone(),
            network: network.combined.clone(),
            disk_read_bytes: collected
                .processes
                .iter()
                .map(|process| process.disk_read_bytes)
                .sum(),
            disk_write_bytes: collected
                .processes
                .iter()
                .map(|process| process.disk_write_bytes)
                .sum(),
        },
        gpu: gpu.clone(),
        top_processes: processes,
        network: network.clone(),
        docker: docker.clone(),
        local_dev_servers: local_dev_servers.to_vec(),
    }
}

fn ranked_processes(processes: &[ProcessSnapshot]) -> RankedProcesses {
    let mut top_cpu = processes
        .iter()
        .map(RankedProcess::from)
        .collect::<Vec<_>>();
    top_cpu.sort_by(|left, right| right.cpu_percent.total_cmp(&left.cpu_percent));
    top_cpu.truncate(40);

    let mut top_memory = processes
        .iter()
        .map(RankedProcess::from)
        .collect::<Vec<_>>();
    top_memory.sort_by(|left, right| right.memory_bytes.cmp(&left.memory_bytes));
    top_memory.truncate(40);

    let mut top_disk_io = processes
        .iter()
        .map(RankedProcess::from)
        .collect::<Vec<_>>();
    top_disk_io.sort_by(|left, right| {
        let right_io = right.disk_read_bytes.saturating_add(right.disk_write_bytes);
        let left_io = left.disk_read_bytes.saturating_add(left.disk_write_bytes);
        right_io.cmp(&left_io)
    });
    top_disk_io.truncate(40);

    RankedProcesses {
        top_cpu,
        top_memory,
        top_disk_io,
        top_gpu: FeatureAvailability::unsupported(
            "Per-process GPU usage is not available through the current Phase 6 provider.",
        ),
    }
}

impl From<&ProcessSnapshot> for RankedProcess {
    fn from(process: &ProcessSnapshot) -> Self {
        Self {
            key: process.key,
            name: process.name.clone(),
            executable_path: process.executable_path.clone(),
            cpu_percent: process.cpu_percent,
            memory_bytes: process.memory_bytes,
            disk_read_bytes: process.disk_read_bytes,
            disk_write_bytes: process.disk_write_bytes,
            listening_port_count: process.listening_port_count,
        }
    }
}

fn combined_top_processes(ranked: &RankedProcesses) -> Vec<RankedProcess> {
    let mut by_key = HashMap::<ProcessKey, RankedProcess>::new();
    for process in ranked
        .top_cpu
        .iter()
        .chain(ranked.top_memory.iter())
        .chain(ranked.top_disk_io.iter())
    {
        by_key.entry(process.key).or_insert_with(|| process.clone());
    }
    let mut processes = by_key.into_values().collect::<Vec<_>>();
    processes.sort_by(|left, right| right.cpu_percent.total_cmp(&left.cpu_percent));
    processes.truncate(60);
    processes
}

fn local_dev_servers(ports: &[crate::domain::PortEndpoint]) -> Vec<LocalDevServerSnapshot> {
    ports
        .iter()
        .filter(|port| port.local_url.is_some())
        .take(80)
        .map(|port| LocalDevServerSnapshot {
            port: port.local_port,
            local_url: port.local_url.clone(),
            lan_urls: port.lan_urls.clone(),
            process_name: port.owning_process_name.clone(),
            binding_scope: format!("{:?}", port.binding_scope).to_lowercase(),
        })
        .collect()
}

fn docker_activity(processes: &[ProcessSnapshot]) -> DockerActivitySnapshot {
    let mut names = processes
        .iter()
        .filter(|process| is_docker_process(&process.name))
        .map(|process| process.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    let process_count = u32::try_from(names.len()).unwrap_or(u32::MAX);
    let availability = if process_count > 0 {
        FeatureAvailability::available(
            "Docker-related process evidence was observed without polling Docker CLI.",
        )
    } else {
        FeatureAvailability::unavailable(
            "No Docker-related process evidence was observed in this sample.",
        )
    };
    DockerActivitySnapshot {
        availability,
        docker_process_count: process_count,
        docker_process_names: names,
        evidence: vec![
            "Recorder captures Docker activity as process evidence to avoid repeated Docker CLI polling during high-frequency recording."
                .to_owned(),
        ],
    }
}

fn is_docker_process(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("docker") || lower.contains("com.docker")
}

fn collector_warnings(collected: &CollectedSnapshot, gpu: &GpuSnapshot) -> Vec<String> {
    let mut warnings = collected
        .overview
        .collector_issues
        .iter()
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    if gpu.adapters.is_empty() {
        warnings.push("GPU telemetry is not available on this hardware/API.".to_owned());
    }
    warnings
}

fn probe_gpu() -> GpuSnapshot {
    let collected_at_ms = unix_time_ms();
    match run_nvidia_smi() {
        Ok(output) if output.success => {
            let adapters = parse_nvidia_smi(&output.stdout);
            if adapters.is_empty() {
                return unsupported_gpu(collected_at_ms, "nvidia-smi returned no GPU rows.");
            }
            GpuSnapshot {
                availability: FeatureAvailability::available(
                    "NVIDIA GPU telemetry was read through nvidia-smi.",
                ),
                provider: "nvidia-smi".to_owned(),
                adapters,
                collected_at_ms,
            }
        }
        Ok(output) => GpuSnapshot {
            availability: FeatureAvailability::unsupported(
                "nvidia-smi is present but did not return supported GPU telemetry.",
            ),
            provider: "nvidia-smi".to_owned(),
            adapters: Vec::new(),
            collected_at_ms,
        }
        .with_evidence(output.stderr),
        Err(SystemDiagnosticsError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            unsupported_gpu(
                collected_at_ms,
                "No supported GPU provider is available. NVIDIA nvidia-smi was not found.",
            )
        }
        Err(SystemDiagnosticsError::Timeout) => GpuSnapshot {
            availability: FeatureAvailability::error(
                "The GPU provider timed out.",
                "Retry after GPU drivers finish responding.",
            ),
            provider: "nvidia-smi".to_owned(),
            adapters: Vec::new(),
            collected_at_ms,
        },
        Err(error) => GpuSnapshot {
            availability: FeatureAvailability::error(
                "The GPU provider could not run.",
                "Verify GPU driver tooling is installed for this Windows user.",
            ),
            provider: "nvidia-smi".to_owned(),
            adapters: vec![GpuAdapterSnapshot {
                name: "GPU provider error".to_owned(),
                provider: "nvidia-smi".to_owned(),
                utilization_percent: None,
                vram_used_bytes: None,
                vram_total_bytes: None,
                temperature_celsius: None,
                evidence: vec![redaction::redact(&error.to_string())],
            }],
            collected_at_ms,
        },
    }
}

trait GpuSnapshotExt {
    fn with_evidence(self, stderr: String) -> Self;
}

impl GpuSnapshotExt for GpuSnapshot {
    fn with_evidence(mut self, stderr: String) -> Self {
        if !stderr.trim().is_empty() {
            self.adapters.push(GpuAdapterSnapshot {
                name: "GPU provider diagnostic".to_owned(),
                provider: self.provider.clone(),
                utilization_percent: None,
                vram_used_bytes: None,
                vram_total_bytes: None,
                temperature_celsius: None,
                evidence: vec![redaction::redact(&stderr)],
            });
        }
        self
    }
}

#[derive(Debug)]
struct GpuCommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn run_nvidia_smi() -> Result<GpuCommandOutput, SystemDiagnosticsError> {
    let mut child = crate::platform::process::hidden_command("nvidia-smi.exe")
        .args([
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output()?;
                return Ok(GpuCommandOutput {
                    success: output.status.success(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: redaction::redact(&String::from_utf8_lossy(&output.stderr)),
                });
            }
            Ok(None) if started.elapsed() >= NVIDIA_SMI_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SystemDiagnosticsError::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(SystemDiagnosticsError::Io(error)),
        }
    }
}

fn parse_nvidia_smi(stdout: &str) -> Vec<GpuAdapterSnapshot> {
    stdout
        .lines()
        .filter_map(|line| {
            let parts = line.split(',').map(|part| part.trim()).collect::<Vec<_>>();
            if parts.len() < 5 || parts[0].is_empty() {
                return None;
            }
            Some(GpuAdapterSnapshot {
                name: redaction::redact(parts[0]),
                provider: "nvidia-smi".to_owned(),
                utilization_percent: parse_f64(parts[1]),
                vram_used_bytes: parse_mib(parts[2]),
                vram_total_bytes: parse_mib(parts[3]),
                temperature_celsius: parse_f64(parts[4]),
                evidence: vec![
                    "Parsed exact nvidia-smi query output with no shell interpolation.".to_owned(),
                ],
            })
        })
        .take(16)
        .collect()
}

fn unsupported_gpu(collected_at_ms: u64, reason: &str) -> GpuSnapshot {
    GpuSnapshot {
        availability: FeatureAvailability::unsupported(reason),
        provider: "none".to_owned(),
        adapters: Vec::new(),
        collected_at_ms,
    }
}

fn parse_f64(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}

fn parse_mib(value: &str) -> Option<u64> {
    value
        .parse::<u64>()
        .ok()
        .and_then(|mib| mib.checked_mul(1024 * 1024))
}

fn downsample_samples(samples: &[RecordingSample], limit: usize) -> Vec<RecordingSample> {
    if samples.len() <= limit {
        return samples.to_vec();
    }
    let stride = samples.len().div_ceil(limit);
    samples
        .iter()
        .enumerate()
        .filter_map(|(index, sample)| {
            (index == 0 || index + 1 == samples.len() || index % stride == 0)
                .then_some(sample.clone())
        })
        .take(limit)
        .collect()
}

fn analyze_samples(samples: &VecDeque<RecordingSample>) -> Vec<CorrelationFinding> {
    let vec = samples.iter().cloned().collect::<Vec<_>>();
    analyze_slice(&vec)
}

fn analyze_slice(samples: &[RecordingSample]) -> Vec<CorrelationFinding> {
    if samples.len() < 2 {
        return Vec::new();
    }

    let mut best: Option<(f64, CorrelationFinding)> = None;
    for pair in samples.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        let previous_by_key = previous
            .top_processes
            .iter()
            .map(|process| (process.key, process))
            .collect::<HashMap<_, _>>();
        for process in &current.top_processes {
            if let Some(old) = previous_by_key.get(&process.key) {
                let cpu_delta = process.cpu_percent - old.cpu_percent;
                let memory_delta =
                    process.memory_bytes.saturating_sub(old.memory_bytes) as f64 / 1_048_576.0;
                let disk_delta = process
                    .disk_read_bytes
                    .saturating_add(process.disk_write_bytes)
                    .saturating_sub(old.disk_read_bytes.saturating_add(old.disk_write_bytes))
                    as f64
                    / 1_048_576.0;
                let score =
                    cpu_delta.max(0.0) * 100.0 + memory_delta.max(0.0) + disk_delta.max(0.0);
                if score <= 0.0 {
                    continue;
                }
                let finding = CorrelationFinding {
                    title: format!("Largest observed process-resource delta: {}", process.name),
                    detail: format!(
                        "Between {} and {}, {} changed by CPU {:+.1} points, memory +{:.1} MiB, disk I/O +{:.1} MiB.",
                        previous.collected_at_ms,
                        current.collected_at_ms,
                        process.name,
                        cpu_delta,
                        memory_delta.max(0.0),
                        disk_delta.max(0.0),
                    ),
                    evidence: vec![
                        format!("PID {} start {}", process.key.pid, process.key.start_time),
                        format!(
                            "Samples compared: {} -> {}",
                            previous.sequence, current.sequence
                        ),
                    ],
                    non_causal_disclaimer:
                        "This is a time correlation from local samples, not proof of causation."
                            .to_owned(),
                };
                if best
                    .as_ref()
                    .is_none_or(|(best_score, _)| score > *best_score)
                {
                    best = Some((score, finding));
                }
            }
        }
    }

    best.map(|(_, finding)| vec![finding]).unwrap_or_else(|| {
        let first = samples.first().expect("length checked");
        let last = samples.last().expect("length checked");
        vec![CorrelationFinding {
            title: "No large per-process delta observed".to_owned(),
            detail: format!(
                "System CPU changed from {:.1}% to {:.1}% across the recording window.",
                first.system.cpu_total_percent, last.system.cpu_total_percent
            ),
            evidence: vec![format!("Samples compared: {} total", samples.len())],
            non_causal_disclaimer:
                "This is a time correlation from local samples, not proof of causation.".to_owned(),
        }]
    })
}

fn validate_name(value: &str) -> Result<String, SystemDiagnosticsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_SESSION_NAME {
        return Err(SystemDiagnosticsError::InvalidRecordingName);
    }
    Ok(redaction::redact(trimmed))
}

fn validate_annotation(value: &str) -> Result<String, SystemDiagnosticsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_ANNOTATION_LABEL {
        return Err(SystemDiagnosticsError::InvalidAnnotation);
    }
    Ok(redaction::redact(trimmed))
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AdapterThroughput, AvailabilityState, BatterySnapshot, NetworkDashboardSnapshot, ProcessKey,
    };

    #[test]
    fn nvidia_smi_parser_handles_defensive_csv() {
        let adapters = parse_nvidia_smi("NVIDIA RTX, 42, 1000, 8000, 61\nbad\n");
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].utilization_percent, Some(42.0));
        assert_eq!(adapters[0].vram_used_bytes, Some(1000 * 1024 * 1024));
    }

    #[test]
    fn recording_names_and_annotations_are_validated() {
        let mut manager = SystemDiagnosticsManager::default();
        assert!(
            manager
                .start(StartMetricRecordingRequest {
                    name: "Fixture build".to_owned(),
                })
                .is_ok()
        );
        assert!(manager.add_annotation("Started npm build").is_ok());
        assert!(manager.add_annotation("").is_err());
    }

    #[test]
    fn correlation_uses_non_causal_language() {
        let key = ProcessKey {
            pid: 123,
            start_time: 456,
        };
        let samples = vec![
            test_sample(0, 1, key, 1.0, 10, 0),
            test_sample(1, 2, key, 55.0, 50 * 1024 * 1024, 5 * 1024 * 1024),
        ];
        let findings = analyze_slice(&samples);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].non_causal_disclaimer.contains("not proof"));
    }

    fn test_sample(
        sequence: u64,
        at: u64,
        key: ProcessKey,
        cpu: f64,
        memory: u64,
        disk: u64,
    ) -> RecordingSample {
        RecordingSample {
            sequence,
            collected_at_ms: at,
            local_only: true,
            included_internet_diagnostics: false,
            system: RecordingSystemSample {
                cpu_total_percent: cpu,
                memory: crate::domain::MemorySnapshot {
                    total_bytes: 100,
                    used_bytes: 50,
                    available_bytes: 50,
                    swap_total_bytes: 0,
                    swap_used_bytes: 0,
                },
                battery: BatterySnapshot {
                    availability: FeatureAvailability::unsupported("test"),
                    percentage: None,
                    ac_online: None,
                    remaining_seconds: None,
                },
                network: AdapterThroughput {
                    received_bytes_per_second: 0,
                    transmitted_bytes_per_second: 0,
                    session_received_bytes: 0,
                    session_transmitted_bytes: 0,
                    total_received_bytes: 0,
                    total_transmitted_bytes: 0,
                    peak_received_bytes_per_second: 0,
                    peak_transmitted_bytes_per_second: 0,
                    timeline: Vec::new(),
                },
                disk_read_bytes: disk,
                disk_write_bytes: 0,
            },
            gpu: GpuSnapshot {
                availability: FeatureAvailability {
                    state: AvailabilityState::Unsupported,
                    reason: Some("test".to_owned()),
                    remediation: None,
                },
                provider: "test".to_owned(),
                adapters: Vec::new(),
                collected_at_ms: at,
            },
            top_processes: vec![RankedProcess {
                key,
                name: "node.exe".to_owned(),
                executable_path: None,
                cpu_percent: cpu,
                memory_bytes: memory,
                disk_read_bytes: disk,
                disk_write_bytes: 0,
                listening_port_count: 1,
            }],
            network: NetworkDashboardSnapshot {
                collected_at_ms: at,
                external_diagnostics_enabled: false,
                combined: AdapterThroughput {
                    received_bytes_per_second: 0,
                    transmitted_bytes_per_second: 0,
                    session_received_bytes: 0,
                    session_transmitted_bytes: 0,
                    total_received_bytes: 0,
                    total_transmitted_bytes: 0,
                    peak_received_bytes_per_second: 0,
                    peak_transmitted_bytes_per_second: 0,
                    timeline: Vec::new(),
                },
                adapters: Vec::new(),
                gateway_reachability: crate::domain::GatewayStatus {
                    state: crate::domain::NetworkDiagnosticState::Unavailable,
                    gateway: None,
                    latency_ms: None,
                    local_only: true,
                    evidence: Vec::new(),
                },
                dns_status: crate::domain::DnsStatus {
                    state: crate::domain::NetworkDiagnosticState::Unavailable,
                    local_only: true,
                    configured_server_count: 0,
                    evidence: Vec::new(),
                },
                vpn_state: crate::domain::VpnState {
                    likely_active: false,
                    confidence: "none".to_owned(),
                    label: "none".to_owned(),
                    evidence: Vec::new(),
                },
                lan_ip_candidates: Vec::new(),
                local_dev_server_warnings: Vec::new(),
                per_process_usage: crate::domain::PerProcessNetworkUsage {
                    availability: FeatureAvailability::unsupported("test"),
                    entries: Vec::new(),
                },
                privacy_note: "redacted".to_owned(),
            },
            docker: DockerActivitySnapshot {
                availability: FeatureAvailability::unsupported("test"),
                docker_process_count: 0,
                docker_process_names: Vec::new(),
                evidence: Vec::new(),
            },
            local_dev_servers: Vec::new(),
        }
    }
}
