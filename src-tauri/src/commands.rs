use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::State;

use crate::cleaner::{self, CleanerError, CleanerManager, CleanerRegistry};
use crate::collector::{CollectedSnapshot, SystemCollector};
use crate::db::{Database, DatabaseError};
use crate::docker::{self, DockerError};
use crate::domain::{
    AddMetricAnnotationRequest, AppError, AppResult, AppSettings, BackgroundTask,
    BackgroundTaskDetail, BackgroundTaskKind, BackgroundTaskOutput, BackgroundTaskState,
    CapabilityEntry, CapabilityReport, CleanupPlan, CleanupScanRequest, CleanupScanResult,
    ComposeDoctorIssue, ComposeProject, CreateCleanupPlanRequest, DockerContainer,
    DockerContainerActionRequest, DockerContainerActionResult, DockerInventory, DockerLogEntry,
    DockerNetwork, DockerStatus, DockerVolume, ExecuteCleanupPlanRequest, FeatureAvailability,
    IntegrationStatus, ManagedCommand, ManagedCommandLogEntry, MetricRecordingDetail,
    MetricRecordingExport, NetworkDashboardSnapshot, NetworkDiagnosticKind,
    NetworkDiagnosticReport, NetworkDiagnosticRequest, OllamaStatus, OverviewSnapshot,
    PortEndpoint, ProcessSnapshot, Project, ProjectDiscoveryResult, ProjectMetadataUpdate,
    PurgeQuarantineItemRequest, PurgeQuarantineManifestRequest, QuarantineItemState,
    QuarantineManifest, RecordingAnnotation, RecordingSessionSummary, RestoreQuarantineItemRequest,
    StartMetricRecordingRequest, SystemDiagnosticsSnapshot, SystemSnapshot, TopologyGraph,
    VerificationState, WslStatus,
};
use crate::integrations::{self, IntegrationError};
use crate::logging::LoggingGuard;
use crate::networking::{NetworkError, NetworkMonitor};
use crate::platform;
use crate::projects::{self, ProjectScanError, ScanRegistry};
use crate::supervisor::{ProcessSupervisor, SupervisorError};
use crate::system_diagnostics::{self, SystemDiagnosticsError, SystemDiagnosticsManager};
use crate::tasks::{TaskManager, TaskRegistry};
use crate::topology;
use uuid::Uuid;

pub struct AppState {
    collector: Arc<Mutex<SystemCollector>>,
    database: Arc<Database>,
    project_scans: ScanRegistry,
    supervisor: ProcessSupervisor,
    network_monitor: Arc<Mutex<NetworkMonitor>>,
    system_diagnostics: Arc<Mutex<SystemDiagnosticsManager>>,
    cleaner: CleanerRegistry,
    cleanup_mutations: Arc<Mutex<()>>,
    docker_operations: Arc<Mutex<()>>,
    tasks: TaskRegistry,
    _logging_guard: Mutex<LoggingGuard>,
}

impl AppState {
    pub fn new(database: Database, logging_guard: LoggingGuard, quarantine_root: PathBuf) -> Self {
        Self {
            collector: Arc::new(Mutex::new(SystemCollector::new())),
            database: Arc::new(database),
            project_scans: ScanRegistry::default(),
            supervisor: ProcessSupervisor::default(),
            network_monitor: Arc::new(Mutex::new(NetworkMonitor::new())),
            system_diagnostics: Arc::new(Mutex::new(SystemDiagnosticsManager::default())),
            cleaner: Arc::new(Mutex::new(CleanerManager::new(quarantine_root))),
            cleanup_mutations: Arc::new(Mutex::new(())),
            docker_operations: Arc::new(Mutex::new(())),
            tasks: Arc::new(Mutex::new(TaskManager::default())),
            _logging_guard: Mutex::new(logging_guard),
        }
    }
}

#[tauri::command]
pub fn list_background_tasks(state: State<'_, AppState>) -> AppResult<Vec<BackgroundTask>> {
    Ok(state
        .tasks
        .lock()
        .map_err(|_| task_registry_error())?
        .list())
}

#[tauri::command]
pub fn get_background_task(
    task_id: String,
    state: State<'_, AppState>,
) -> AppResult<BackgroundTaskDetail> {
    state
        .tasks
        .lock()
        .map_err(|_| task_registry_error())?
        .get(&task_id)
        .ok_or_else(|| {
            not_found(
                "BACKGROUND_TASK_NOT_FOUND",
                "That background task is unavailable.",
            )
        })
}

#[tauri::command]
pub fn cancel_background_task(task_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let task = state
        .tasks
        .lock()
        .map_err(|_| task_registry_error())?
        .get(&task_id)
        .ok_or_else(|| {
            not_found(
                "BACKGROUND_TASK_NOT_FOUND",
                "That background task is unavailable.",
            )
        })?;
    match task.task.kind {
        BackgroundTaskKind::CleanupScan => {
            let manager = state.cleaner.lock().map_err(|_| cleaner_registry_error())?;
            if !manager.cancel_scan(&task_id) {
                return Err(not_found(
                    "CLEANUP_SCAN_NOT_ACTIVE",
                    "That cleanup scan is no longer active.",
                ));
            }
        }
        BackgroundTaskKind::ProjectDiscovery => state
            .project_scans
            .cancel(&task_id)
            .map_err(project_scan_error)?,
        BackgroundTaskKind::Quarantine | BackgroundTaskKind::InternetDiagnostics => {
            return Err(AppError::new(
                "BACKGROUND_TASK_NOT_CANCELLABLE",
                "This task cannot be cancelled safely after it starts.",
            ));
        }
    }
    if !state
        .tasks
        .lock()
        .map_err(|_| task_registry_error())?
        .request_cancel(&task_id)
    {
        return Err(AppError::new(
            "BACKGROUND_TASK_NOT_CANCELLABLE",
            "This task is no longer cancellable.",
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_overview_snapshot(state: State<'_, AppState>) -> AppResult<OverviewSnapshot> {
    Ok(collect(&state).await?.overview)
}

#[tauri::command]
pub async fn get_system_snapshot(state: State<'_, AppState>) -> AppResult<SystemSnapshot> {
    Ok(collect(&state).await?.overview.system)
}

#[tauri::command]
pub async fn get_system_diagnostics_snapshot(
    state: State<'_, AppState>,
) -> AppResult<SystemDiagnosticsSnapshot> {
    let started = Instant::now();
    let settings = state.database.get_settings().map_err(database_error)?;
    let external_enabled = settings.external_network_checks;
    let collected = collect(&state).await?;
    let network_monitor = Arc::clone(&state.network_monitor);
    let system_diagnostics = Arc::clone(&state.system_diagnostics);
    blocking("GET_WAR_ROOM_SNAPSHOT_TASK_FAILED", move || {
        let network = {
            let mut monitor = network_monitor.lock().map_err(|_| {
                AppError::new(
                    "NETWORK_MONITOR_UNAVAILABLE",
                    "The network monitor is temporarily unavailable.",
                )
                .with_remediation("Retry after the current network collection completes.")
                .retryable()
            })?;
            monitor.snapshot(&collected.processes, &collected.ports, external_enabled)
        };
        let mut manager = system_diagnostics.lock().map_err(|_| {
            AppError::new(
                "WAR_ROOM_UNAVAILABLE",
                "The System Diagnostics recorder is temporarily unavailable.",
            )
            .with_remediation("Retry after the current recorder operation completes.")
            .retryable()
        })?;
        Ok(system_diagnostics::build_system_diagnostics_snapshot(
            collected,
            network,
            settings,
            &mut manager,
            started.elapsed().as_millis() as u64,
        ))
    })
    .await
}

#[tauri::command]
pub async fn list_processes(state: State<'_, AppState>) -> AppResult<Vec<ProcessSnapshot>> {
    Ok(collect(&state).await?.processes)
}

#[tauri::command]
pub async fn list_ports(state: State<'_, AppState>) -> AppResult<Vec<PortEndpoint>> {
    Ok(collect(&state).await?.ports)
}

#[tauri::command]
pub async fn list_integrations(state: State<'_, AppState>) -> AppResult<Vec<IntegrationStatus>> {
    let collected = collect(&state).await?;
    blocking("LIST_INTEGRATIONS_TASK_FAILED", move || {
        Ok(integrations::list_integrations(
            &collected.processes,
            &collected.ports,
        ))
    })
    .await
}

#[tauri::command]
pub async fn probe_integration(
    detector_id: String,
    state: State<'_, AppState>,
) -> AppResult<IntegrationStatus> {
    let collected = collect(&state).await?;
    blocking("PROBE_INTEGRATION_TASK_FAILED", move || {
        integrations::probe_integration(&detector_id, &collected.processes, &collected.ports)
            .map_err(integration_error)
    })
    .await
}

#[tauri::command]
pub async fn get_ollama_status(state: State<'_, AppState>) -> AppResult<OllamaStatus> {
    let processes = collect(&state).await?.processes;
    blocking("GET_OLLAMA_STATUS_TASK_FAILED", move || {
        Ok(integrations::get_ollama_status(&processes))
    })
    .await
}

#[tauri::command]
pub async fn get_wsl_status() -> AppResult<WslStatus> {
    blocking("GET_WSL_STATUS_TASK_FAILED", move || {
        Ok(integrations::get_wsl_status())
    })
    .await
}

#[tauri::command]
pub async fn get_network_snapshot(
    state: State<'_, AppState>,
) -> AppResult<NetworkDashboardSnapshot> {
    let external_enabled = state
        .database
        .get_settings()
        .map_err(database_error)?
        .external_network_checks;
    let collected = collect(&state).await?;
    let monitor = Arc::clone(&state.network_monitor);
    blocking("GET_NETWORK_SNAPSHOT_TASK_FAILED", move || {
        let mut monitor = monitor.lock().map_err(|_| {
            AppError::new(
                "NETWORK_MONITOR_UNAVAILABLE",
                "The network monitor is temporarily unavailable.",
            )
            .with_remediation("Retry after the current network collection completes.")
            .retryable()
        })?;
        Ok(monitor.snapshot(&collected.processes, &collected.ports, external_enabled))
    })
    .await
}

#[tauri::command]
pub async fn run_network_diagnostic(
    request: NetworkDiagnosticRequest,
    state: State<'_, AppState>,
) -> AppResult<NetworkDiagnosticReport> {
    let external_enabled = state
        .database
        .get_settings()
        .map_err(database_error)?
        .external_network_checks;
    let collected = collect(&state).await?;
    let monitor = Arc::clone(&state.network_monitor);
    blocking("RUN_NETWORK_DIAGNOSTIC_TASK_FAILED", move || {
        let mut monitor = monitor.lock().map_err(|_| {
            AppError::new(
                "NETWORK_MONITOR_UNAVAILABLE",
                "The network monitor is temporarily unavailable.",
            )
            .with_remediation("Retry after the current network diagnostic completes.")
            .retryable()
        })?;
        monitor
            .run_diagnostic(
                request,
                external_enabled,
                &collected.processes,
                &collected.ports,
            )
            .map_err(network_error)
    })
    .await
}

#[tauri::command]
pub async fn start_internet_test_task(state: State<'_, AppState>) -> AppResult<BackgroundTask> {
    let external_enabled = state
        .database
        .get_settings()
        .map_err(database_error)?
        .external_network_checks;
    if !external_enabled {
        return Err(network_error(NetworkError::ExternalDiagnosticsDisabled));
    }
    let collected = collect(&state).await?;
    let task_id = Uuid::new_v4().to_string();
    let task = start_task(
        &state.tasks,
        task_id.clone(),
        BackgroundTaskKind::InternetDiagnostics,
        "Run Internet Test",
        "/network",
        false,
    )?;
    let monitor = Arc::clone(&state.network_monitor);
    let tasks = Arc::clone(&state.tasks);
    spawn_task(tasks, task_id, move || {
        let mut monitor = monitor.lock().map_err(|_| {
            AppError::new(
                "NETWORK_MONITOR_UNAVAILABLE",
                "The network monitor is temporarily unavailable.",
            )
            .with_remediation("Retry after the current network diagnostic completes.")
            .retryable()
        })?;
        let kinds = [
            NetworkDiagnosticKind::InternetDnsResolution,
            NetworkDiagnosticKind::InternetLatency,
            NetworkDiagnosticKind::PacketLoss,
            NetworkDiagnosticKind::DownloadSpeed,
            NetworkDiagnosticKind::UploadSpeed,
            NetworkDiagnosticKind::RouteVpnBehavior,
        ];
        let mut reports = Vec::with_capacity(kinds.len());
        for kind in kinds {
            reports.push(
                monitor
                    .run_diagnostic(
                        NetworkDiagnosticRequest {
                            kind,
                            consent_to_external: true,
                        },
                        true,
                        &collected.processes,
                        &collected.ports,
                    )
                    .map_err(network_error)?,
            );
        }
        Ok((
            BackgroundTaskState::Succeeded,
            "Internet diagnostics completed; review which tests contacted the internet.".to_owned(),
            BackgroundTaskOutput::InternetDiagnostics(reports),
        ))
    });
    Ok(task)
}

#[tauri::command]
pub fn start_metric_recording(
    request: StartMetricRecordingRequest,
    state: State<'_, AppState>,
) -> AppResult<RecordingSessionSummary> {
    state
        .system_diagnostics
        .lock()
        .map_err(|_| recorder_lock_error())?
        .start(request)
        .map_err(system_diagnostics_error)
}

#[tauri::command]
pub fn add_metric_annotation(
    request: AddMetricAnnotationRequest,
    state: State<'_, AppState>,
) -> AppResult<RecordingAnnotation> {
    state
        .system_diagnostics
        .lock()
        .map_err(|_| recorder_lock_error())?
        .add_annotation(&request.label)
        .map_err(system_diagnostics_error)
}

#[tauri::command]
pub fn stop_metric_recording(state: State<'_, AppState>) -> AppResult<RecordingSessionSummary> {
    let detail = state
        .system_diagnostics
        .lock()
        .map_err(|_| recorder_lock_error())?
        .stop()
        .map_err(system_diagnostics_error)?;
    state
        .database
        .save_metric_recording(&detail)
        .map_err(database_error)?;
    Ok(detail.summary)
}

#[tauri::command]
pub fn list_metric_sessions(state: State<'_, AppState>) -> AppResult<Vec<RecordingSessionSummary>> {
    let mut sessions = state
        .database
        .list_metric_sessions()
        .map_err(database_error)?;
    if let Some(active) = state
        .system_diagnostics
        .lock()
        .map_err(|_| recorder_lock_error())?
        .active_summary()
    {
        sessions.retain(|session| session.id != active.id);
        sessions.insert(0, active);
    }
    Ok(sessions)
}

#[tauri::command]
pub fn get_metric_recording(
    session_id: String,
    state: State<'_, AppState>,
) -> AppResult<MetricRecordingDetail> {
    if let Some(active) = state
        .system_diagnostics
        .lock()
        .map_err(|_| recorder_lock_error())?
        .active_detail()
        .filter(|detail| detail.summary.id == session_id)
    {
        return Ok(active);
    }
    state
        .database
        .get_metric_recording(&session_id)
        .map_err(database_error)?
        .ok_or_else(|| recording_not_found(&session_id))
}

#[tauri::command]
pub fn export_metric_recording(
    session_id: String,
    state: State<'_, AppState>,
) -> AppResult<MetricRecordingExport> {
    let detail = get_metric_recording(session_id, state)?;
    Ok(system_diagnostics::export_detail(detail))
}

#[tauri::command]
pub fn delete_metric_recording(session_id: String, state: State<'_, AppState>) -> AppResult<()> {
    if state
        .system_diagnostics
        .lock()
        .map_err(|_| recorder_lock_error())?
        .delete_active_if_matches(&session_id)
    {
        return Ok(());
    }
    if state
        .database
        .delete_metric_recording(&session_id)
        .map_err(database_error)?
    {
        Ok(())
    } else {
        Err(recording_not_found(&session_id))
    }
}

#[tauri::command]
pub async fn scan_cleanup_candidates(
    request: CleanupScanRequest,
    state: State<'_, AppState>,
) -> AppResult<CleanupScanResult> {
    let operation_id = request.operation_id.clone();
    let cleaner_registry = Arc::clone(&state.cleaner);
    let (cancellation, quarantine_root) = {
        let mut manager = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?;
        (
            manager.begin_scan(&operation_id).map_err(cleaner_error)?,
            manager.quarantine_root(),
        )
    };
    blocking("CLEANUP_SCAN_TASK_FAILED", move || {
        let result = cleaner::scan_cleanup_candidates(request, cancellation, &quarantine_root);
        let mut manager = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?;
        match result {
            Ok(result) => {
                manager.finish_scan(result.clone());
                Ok(result)
            }
            Err(error) => {
                manager.abandon_scan(&operation_id);
                Err(cleaner_error(error))
            }
        }
    })
    .await
}

#[tauri::command]
pub fn start_cleanup_scan_task(
    request: CleanupScanRequest,
    state: State<'_, AppState>,
) -> AppResult<BackgroundTask> {
    let task_id = request.operation_id.clone();
    let cleaner_registry = Arc::clone(&state.cleaner);
    let (cancellation, quarantine_root) = {
        let mut manager = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?;
        (
            manager.begin_scan(&task_id).map_err(cleaner_error)?,
            manager.quarantine_root(),
        )
    };
    let task = start_task(
        &state.tasks,
        task_id.clone(),
        BackgroundTaskKind::CleanupScan,
        "Scan selected folders",
        "/cleaner",
        true,
    )?;
    let tasks = Arc::clone(&state.tasks);
    spawn_task(tasks, task_id.clone(), move || {
        let result = cleaner::scan_cleanup_candidates(request, cancellation, &quarantine_root);
        let mut manager = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?;
        match result {
            Ok(result) => {
                let final_state = if result.cancelled {
                    BackgroundTaskState::Cancelled
                } else {
                    BackgroundTaskState::Succeeded
                };
                let summary = if result.cancelled {
                    "Cleanup scan cancelled at a safe checkpoint.".to_owned()
                } else {
                    format!("Found {} reviewable candidate(s).", result.candidates.len())
                };
                manager.finish_scan(result.clone());
                Ok((
                    final_state,
                    summary,
                    BackgroundTaskOutput::CleanupScan(result),
                ))
            }
            Err(error) => {
                manager.abandon_scan(&task_id);
                Err(cleaner_error(error))
            }
        }
    });
    Ok(task)
}

#[tauri::command]
pub fn cancel_cleanup_scan(operation_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let manager = state.cleaner.lock().map_err(|_| cleaner_registry_error())?;
    if manager.cancel_scan(&operation_id) {
        Ok(())
    } else {
        Err(AppError::new(
            "CLEANUP_SCAN_NOT_ACTIVE",
            "That cleanup scan is no longer active.",
        ))
    }
}

#[tauri::command]
pub fn create_cleanup_plan(
    request: CreateCleanupPlanRequest,
    state: State<'_, AppState>,
) -> AppResult<CleanupPlan> {
    let plan = state
        .cleaner
        .lock()
        .map_err(|_| cleaner_registry_error())?
        .create_plan(&request)
        .map_err(cleaner_error)?;
    state
        .database
        .save_cleanup_plan(&plan)
        .map_err(database_error)?;
    Ok(plan)
}

#[tauri::command]
pub async fn execute_quarantine_plan(
    request: ExecuteCleanupPlanRequest,
    state: State<'_, AppState>,
) -> AppResult<QuarantineManifest> {
    let database = Arc::clone(&state.database);
    let cleaner_registry = Arc::clone(&state.cleaner);
    let mutation_lock = Arc::clone(&state.cleanup_mutations);
    blocking("QUARANTINE_TASK_FAILED", move || {
        let _mutation = mutation_lock.lock().map_err(|_| cleanup_mutation_error())?;
        let mut plan = database
            .get_cleanup_plan(&request.plan_id)
            .map_err(database_error)?
            .ok_or_else(|| cleaner_error(CleanerError::PlanNotFound))?;
        if plan.state != crate::domain::CleanupPlanState::Reviewed || plan.manifest_id.is_some() {
            return Err(cleaner_error(CleanerError::PlanAlreadyExecuted));
        }
        if request.confirmation != plan.confirmation_phrase {
            return Err(cleaner_error(CleanerError::ConfirmationMismatch(
                plan.confirmation_phrase.clone(),
            )));
        }
        let quarantine_root = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?
            .quarantine_root();
        let mut manifest = cleaner::begin_manifest(&plan, &quarantine_root);
        if !database
            .start_quarantine_manifest(&mut plan, &manifest)
            .map_err(database_error)?
        {
            return Err(cleaner_error(CleanerError::PlanAlreadyExecuted));
        }

        for index in 0..plan.items.len() {
            {
                let candidate = &plan.items[index];
                let item = &mut manifest.items[index];
                if let Err(error) = cleaner::quarantine_item(candidate, item, &quarantine_root) {
                    if matches!(error, CleanerError::PartialFailure(_)) {
                        item.state = QuarantineItemState::Partial;
                        item.verification = VerificationState::CopyVerified;
                        item.purge_eligible = true;
                    } else {
                        item.state = QuarantineItemState::Failed;
                        item.verification = VerificationState::Failed;
                        item.purge_eligible = false;
                    }
                    item.error = Some(error.to_string());
                }
            }
            cleaner::refresh_manifest_state(&mut manifest);
            database
                .update_quarantine_manifest(&manifest)
                .map_err(database_error)?;
        }
        Ok(manifest)
    })
    .await
}

#[tauri::command]
pub fn start_quarantine_task(
    request: ExecuteCleanupPlanRequest,
    state: State<'_, AppState>,
) -> AppResult<BackgroundTask> {
    let task_id = format!("quarantine:{}", request.plan_id);
    let task = start_task(
        &state.tasks,
        task_id.clone(),
        BackgroundTaskKind::Quarantine,
        "Move reviewed items to quarantine",
        "/cleaner",
        false,
    )?;
    let database = Arc::clone(&state.database);
    let cleaner_registry = Arc::clone(&state.cleaner);
    let mutation_lock = Arc::clone(&state.cleanup_mutations);
    let tasks = Arc::clone(&state.tasks);
    spawn_task(tasks, task_id, move || {
        let _mutation = mutation_lock.lock().map_err(|_| cleanup_mutation_error())?;
        let mut plan = database
            .get_cleanup_plan(&request.plan_id)
            .map_err(database_error)?
            .ok_or_else(|| cleaner_error(CleanerError::PlanNotFound))?;
        if plan.state != crate::domain::CleanupPlanState::Reviewed || plan.manifest_id.is_some() {
            return Err(cleaner_error(CleanerError::PlanAlreadyExecuted));
        }
        if request.confirmation != plan.confirmation_phrase {
            return Err(cleaner_error(CleanerError::ConfirmationMismatch(
                plan.confirmation_phrase.clone(),
            )));
        }
        let quarantine_root = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?
            .quarantine_root();
        let mut manifest = cleaner::begin_manifest(&plan, &quarantine_root);
        if !database
            .start_quarantine_manifest(&mut plan, &manifest)
            .map_err(database_error)?
        {
            return Err(cleaner_error(CleanerError::PlanAlreadyExecuted));
        }
        for index in 0..plan.items.len() {
            {
                let candidate = &plan.items[index];
                let item = &mut manifest.items[index];
                if let Err(error) = cleaner::quarantine_item(candidate, item, &quarantine_root) {
                    if matches!(error, CleanerError::PartialFailure(_)) {
                        item.state = QuarantineItemState::Partial;
                        item.verification = VerificationState::CopyVerified;
                        item.purge_eligible = true;
                    } else {
                        item.state = QuarantineItemState::Failed;
                        item.verification = VerificationState::Failed;
                        item.purge_eligible = false;
                    }
                    item.error = Some(error.to_string());
                }
            }
            cleaner::refresh_manifest_state(&mut manifest);
            database
                .update_quarantine_manifest(&manifest)
                .map_err(database_error)?;
        }
        let moved = manifest
            .items
            .iter()
            .filter(|item| item.state == QuarantineItemState::Quarantined)
            .count();
        Ok((
            BackgroundTaskState::Succeeded,
            format!("Quarantined {moved} item(s); permanent purge remains separate."),
            BackgroundTaskOutput::Quarantine(manifest),
        ))
    });
    Ok(task)
}

#[tauri::command]
pub fn list_quarantine_manifests(state: State<'_, AppState>) -> AppResult<Vec<QuarantineManifest>> {
    state
        .database
        .list_quarantine_manifests()
        .map_err(database_error)
}

#[tauri::command]
pub async fn restore_quarantine_item(
    request: RestoreQuarantineItemRequest,
    state: State<'_, AppState>,
) -> AppResult<QuarantineManifest> {
    let database = Arc::clone(&state.database);
    let cleaner_registry = Arc::clone(&state.cleaner);
    let mutation_lock = Arc::clone(&state.cleanup_mutations);
    blocking("QUARANTINE_RESTORE_TASK_FAILED", move || {
        let _mutation = mutation_lock.lock().map_err(|_| cleanup_mutation_error())?;
        let mut manifest = database
            .get_quarantine_manifest(&request.manifest_id)
            .map_err(database_error)?
            .ok_or_else(|| cleaner_error(CleanerError::ManifestNotFound))?;
        let item = manifest
            .items
            .iter_mut()
            .find(|item| item.id == request.item_id)
            .ok_or_else(|| cleaner_error(CleanerError::ManifestNotFound))?;
        let quarantine_root = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?
            .quarantine_root();
        cleaner::restore_item(item, &quarantine_root, request.conflict_strategy)
            .map_err(cleaner_error)?;
        cleaner::refresh_manifest_state(&mut manifest);
        database
            .update_quarantine_manifest(&manifest)
            .map_err(database_error)?;
        Ok(manifest)
    })
    .await
}

#[tauri::command]
pub async fn purge_quarantine_item(
    request: PurgeQuarantineItemRequest,
    state: State<'_, AppState>,
) -> AppResult<QuarantineManifest> {
    let database = Arc::clone(&state.database);
    let cleaner_registry = Arc::clone(&state.cleaner);
    let mutation_lock = Arc::clone(&state.cleanup_mutations);
    blocking("QUARANTINE_PURGE_TASK_FAILED", move || {
        let _mutation = mutation_lock.lock().map_err(|_| cleanup_mutation_error())?;
        let mut manifest = database
            .get_quarantine_manifest(&request.manifest_id)
            .map_err(database_error)?
            .ok_or_else(|| cleaner_error(CleanerError::ManifestNotFound))?;
        let item = manifest
            .items
            .iter_mut()
            .find(|item| item.id == request.item_id)
            .ok_or_else(|| cleaner_error(CleanerError::ManifestNotFound))?;
        let quarantine_root = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?
            .quarantine_root();
        cleaner::purge_item(item, &quarantine_root, &request.confirmation)
            .map_err(cleaner_error)?;
        cleaner::refresh_manifest_state(&mut manifest);
        database
            .update_quarantine_manifest(&manifest)
            .map_err(database_error)?;
        Ok(manifest)
    })
    .await
}

#[tauri::command]
pub async fn purge_quarantine_manifest(
    request: PurgeQuarantineManifestRequest,
    state: State<'_, AppState>,
) -> AppResult<QuarantineManifest> {
    let database = Arc::clone(&state.database);
    let cleaner_registry = Arc::clone(&state.cleaner);
    let mutation_lock = Arc::clone(&state.cleanup_mutations);
    blocking("QUARANTINE_MANIFEST_PURGE_TASK_FAILED", move || {
        let _mutation = mutation_lock.lock().map_err(|_| cleanup_mutation_error())?;
        let mut manifest = database
            .get_quarantine_manifest(&request.manifest_id)
            .map_err(database_error)?
            .ok_or_else(|| cleaner_error(CleanerError::ManifestNotFound))?;
        let expected = format!("PURGE MANIFEST {}", manifest.id);
        if request.confirmation != expected {
            return Err(cleaner_error(CleanerError::ConfirmationMismatch(expected)));
        }
        let quarantine_root = cleaner_registry
            .lock()
            .map_err(|_| cleaner_registry_error())?
            .quarantine_root();
        for index in 0..manifest.items.len() {
            if !manifest.items[index].purge_eligible
                || !matches!(
                    manifest.items[index].state,
                    QuarantineItemState::Quarantined | QuarantineItemState::Partial
                )
            {
                continue;
            }
            let phrase = manifest.items[index].purge_confirmation_phrase.clone();
            if let Err(error) =
                cleaner::purge_item(&mut manifest.items[index], &quarantine_root, &phrase)
            {
                manifest.items[index].error = Some(error.to_string());
            }
            cleaner::refresh_manifest_state(&mut manifest);
            database
                .update_quarantine_manifest(&manifest)
                .map_err(database_error)?;
        }
        Ok(manifest)
    })
    .await
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.database.get_settings().map_err(database_error)
}

#[tauri::command]
pub fn update_app_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    state
        .database
        .update_settings(&settings)
        .map_err(database_error)
}

#[tauri::command]
pub fn get_capability_report() -> CapabilityReport {
    let windows = cfg!(target_os = "windows");
    let available_on_windows = |reason: &'static str| {
        if windows {
            FeatureAvailability::available(reason)
        } else {
            FeatureAvailability::unsupported("This production milestone targets Windows 11.")
        }
    };
    let battery = match platform::query_power_status() {
        Ok(status) if status.battery_present => {
            FeatureAvailability::available("Windows reports a system battery.")
        }
        Ok(_) => FeatureAvailability::unsupported("Windows reports no system battery."),
        Err(error) => FeatureAvailability::error(
            error.to_string(),
            "Retry the capability probe without elevating the whole application.",
        ),
    };

    CapabilityReport {
        platform: std::env::consts::OS.to_owned(),
        standard_user_mode: true,
        features: vec![
            CapabilityEntry {
                id: "system-metrics".to_owned(),
                label: "System metrics".to_owned(),
                availability: available_on_windows(
                    "CPU, memory, disk, and network collectors are active.",
                ),
                read_only: true,
            },
            CapabilityEntry {
                id: "process-inspection".to_owned(),
                label: "Process inspection".to_owned(),
                availability: available_on_windows(
                    "Process metadata is read through standard-user APIs.",
                ),
                read_only: true,
            },
            CapabilityEntry {
                id: "owned-ports".to_owned(),
                label: "TCP/UDP ownership".to_owned(),
                availability: available_on_windows(
                    "Windows ownership tables map endpoints to PIDs.",
                ),
                read_only: true,
            },
            CapabilityEntry {
                id: "battery".to_owned(),
                label: "Battery and AC state".to_owned(),
                availability: battery,
                read_only: true,
            },
            CapabilityEntry {
                id: "gpu".to_owned(),
                label: "GPU telemetry".to_owned(),
                availability: FeatureAvailability::unsupported(
                    "System Diagnostics probes NVIDIA telemetry through bounded nvidia-smi when available; other providers report unsupported.",
                ),
                read_only: true,
            },
            CapabilityEntry {
                id: "local-settings".to_owned(),
                label: "Local settings database".to_owned(),
                availability: FeatureAvailability::available(
                    "Settings use the application data directory with versioned SQLite migrations.",
                ),
                read_only: false,
            },
            CapabilityEntry {
                id: "docker-compose".to_owned(),
                label: "Docker and Compose".to_owned(),
                availability: available_on_windows(
                    "Docker is probed through exact Docker CLI arguments; absence, stopped daemon, and permission states are shown explicitly.",
                ),
                read_only: false,
            },
            CapabilityEntry {
                id: "integration-detectors".to_owned(),
                label: "Integration detectors".to_owned(),
                availability: available_on_windows(
                    "Tool, runtime, local service, Ollama, and WSL detectors use constrained evidence sources and bounded exact probes.",
                ),
                read_only: true,
            },
            CapabilityEntry {
                id: "network-dashboard".to_owned(),
                label: "Network dashboard".to_owned(),
                availability: available_on_windows(
                    "Local adapter throughput uses OS counters; internet diagnostics remain disabled until explicit opt-in.",
                ),
                read_only: true,
            },
            CapabilityEntry {
                id: "system-diagnostics".to_owned(),
                label: "System Diagnostics and recordings".to_owned(),
                availability: available_on_windows(
                    "Frequent local metrics, bounded recording sessions, annotations, export, delete, and honest GPU unsupported states are available.",
                ),
                read_only: false,
            },
            CapabilityEntry {
                id: "safe-cleaner".to_owned(),
                label: "Cleaner and quarantine".to_owned(),
                availability: available_on_windows(
                    "Explicit-root scanning, immutable plans, reversible quarantine, exact restore, and confirmed purge are available.",
                ),
                read_only: false,
            },
        ],
    }
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> AppResult<Vec<Project>> {
    let database = Arc::clone(&state.database);
    blocking("LIST_PROJECTS_TASK_FAILED", move || {
        database.list_projects().map_err(database_error)
    })
    .await
}

#[tauri::command]
pub async fn add_project(path: String, state: State<'_, AppState>) -> AppResult<Project> {
    let database = Arc::clone(&state.database);
    blocking("ADD_PROJECT_TASK_FAILED", move || {
        let mut scanned =
            projects::scan_project_path(&PathBuf::from(path)).map_err(project_scan_error)?;
        if let Some(existing) = database
            .get_project_by_canonical_path(&scanned.canonical_root_path)
            .map_err(database_error)?
        {
            projects::merge_existing_metadata(&mut scanned, &existing);
        }
        database.save_project(&scanned).map_err(database_error)
    })
    .await
}

#[tauri::command]
pub async fn add_project_root(
    root_path: String,
    operation_id: String,
    maximum_depth: u8,
    state: State<'_, AppState>,
) -> AppResult<ProjectDiscoveryResult> {
    let database = Arc::clone(&state.database);
    let registry = state.project_scans.clone();
    let permit = registry
        .begin(operation_id.clone())
        .map_err(project_scan_error)?;
    let cancelled = permit.cancelled();

    blocking("ADD_PROJECT_ROOT_TASK_FAILED", move || {
        let root = projects::validate_project_root(&PathBuf::from(root_path))
            .map_err(project_scan_error)?;
        let mut result =
            projects::discover_projects(&root, operation_id.clone(), maximum_depth, cancelled);
        for project in &mut result.projects {
            if let Some(existing) = database
                .get_project_by_canonical_path(&project.canonical_root_path)
                .map_err(database_error)?
            {
                projects::merge_existing_metadata(project, &existing);
            }
            *project = database.save_project(project).map_err(database_error)?;
        }
        database
            .save_project_root(
                &operation_id,
                &root.root_path_text(),
                &root.canonical_root_path_text(),
                projects::normalize_depth(maximum_depth),
                projects::now_ms(),
            )
            .map_err(database_error)?;
        drop(permit);
        Ok(result)
    })
    .await
}

#[tauri::command]
pub fn start_project_discovery_task(
    root_path: String,
    operation_id: String,
    maximum_depth: u8,
    state: State<'_, AppState>,
) -> AppResult<BackgroundTask> {
    let database = Arc::clone(&state.database);
    let registry = state.project_scans.clone();
    let permit = registry
        .begin(operation_id.clone())
        .map_err(project_scan_error)?;
    let cancelled = permit.cancelled();
    let task = start_task(
        &state.tasks,
        operation_id.clone(),
        BackgroundTaskKind::ProjectDiscovery,
        "Discover projects",
        "/projects",
        true,
    )?;
    let tasks = Arc::clone(&state.tasks);
    spawn_task(tasks, operation_id.clone(), move || {
        let root = projects::validate_project_root(&PathBuf::from(root_path))
            .map_err(project_scan_error)?;
        let mut result =
            projects::discover_projects(&root, operation_id.clone(), maximum_depth, cancelled);
        for project in &mut result.projects {
            if let Some(existing) = database
                .get_project_by_canonical_path(&project.canonical_root_path)
                .map_err(database_error)?
            {
                projects::merge_existing_metadata(project, &existing);
            }
            *project = database.save_project(project).map_err(database_error)?;
        }
        database
            .save_project_root(
                &operation_id,
                &root.root_path_text(),
                &root.canonical_root_path_text(),
                projects::normalize_depth(maximum_depth),
                projects::now_ms(),
            )
            .map_err(database_error)?;
        drop(permit);
        let final_state = if result.cancelled {
            BackgroundTaskState::Cancelled
        } else {
            BackgroundTaskState::Succeeded
        };
        let summary = if result.cancelled {
            "Project discovery cancelled at a safe checkpoint.".to_owned()
        } else {
            format!("Discovered {} project(s).", result.projects.len())
        };
        Ok((
            final_state,
            summary,
            BackgroundTaskOutput::ProjectDiscovery(result),
        ))
    });
    Ok(task)
}

#[tauri::command]
pub async fn scan_project(project_id: String, state: State<'_, AppState>) -> AppResult<Project> {
    let database = Arc::clone(&state.database);
    blocking("SCAN_PROJECT_TASK_FAILED", move || {
        let existing = database
            .get_project(&project_id)
            .map_err(database_error)?
            .ok_or_else(|| not_found("PROJECT_NOT_FOUND", "The project is not registered."))?;
        let mut scanned = projects::scan_project_path(&PathBuf::from(&existing.root_path))
            .map_err(project_scan_error)?;
        projects::merge_existing_metadata(&mut scanned, &existing);
        database.save_project(&scanned).map_err(database_error)
    })
    .await
}

#[tauri::command]
pub async fn remove_project(project_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let database = Arc::clone(&state.database);
    blocking("REMOVE_PROJECT_TASK_FAILED", move || {
        database.remove_project(&project_id).map_err(database_error)
    })
    .await
}

#[tauri::command]
pub fn cancel_project_scan(operation_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state
        .project_scans
        .cancel(&operation_id)
        .map_err(project_scan_error)
}

#[tauri::command]
pub async fn update_project_metadata(
    project_id: String,
    metadata: ProjectMetadataUpdate,
    state: State<'_, AppState>,
) -> AppResult<Project> {
    let database = Arc::clone(&state.database);
    blocking("UPDATE_PROJECT_METADATA_TASK_FAILED", move || {
        let normalized = ProjectMetadataUpdate {
            tags: projects::normalize_tags(&metadata.tags),
            notes: metadata.notes.chars().take(20_000).collect(),
            checklist: metadata.checklist.into_iter().take(200).collect(),
            pinned: metadata.pinned,
            archived: metadata.archived,
        };
        database
            .update_project_metadata(&project_id, &normalized)
            .map_err(database_error)?
            .ok_or_else(|| not_found("PROJECT_NOT_FOUND", "The project is not registered."))
    })
    .await
}

#[tauri::command]
pub async fn run_project_command(
    project_id: String,
    script_id: String,
    state: State<'_, AppState>,
) -> AppResult<ManagedCommand> {
    let database = Arc::clone(&state.database);
    let supervisor = state.supervisor.clone();
    blocking("RUN_PROJECT_COMMAND_TASK_FAILED", move || {
        let project = database
            .get_project(&project_id)
            .map_err(database_error)?
            .ok_or_else(|| not_found("PROJECT_NOT_FOUND", "The project is not registered."))?;
        supervisor
            .run_project_command(&project, &script_id)
            .map_err(supervisor_error)
    })
    .await
}

#[tauri::command]
pub async fn list_managed_processes(state: State<'_, AppState>) -> AppResult<Vec<ManagedCommand>> {
    let supervisor = state.supervisor.clone();
    blocking("LIST_MANAGED_PROCESSES_TASK_FAILED", move || {
        supervisor.list_runs().map_err(supervisor_error)
    })
    .await
}

#[tauri::command]
pub async fn get_managed_process_logs(
    run_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<ManagedCommandLogEntry>> {
    let supervisor = state.supervisor.clone();
    blocking("GET_MANAGED_PROCESS_LOGS_TASK_FAILED", move || {
        supervisor.logs(&run_id).map_err(supervisor_error)
    })
    .await
}

#[tauri::command]
pub async fn stop_managed_process(
    run_id: String,
    force: bool,
    state: State<'_, AppState>,
) -> AppResult<ManagedCommand> {
    let supervisor = state.supervisor.clone();
    blocking("STOP_MANAGED_PROCESS_TASK_FAILED", move || {
        supervisor
            .stop_run(&run_id, force)
            .map_err(supervisor_error)
    })
    .await
}

#[tauri::command]
pub async fn get_topology_graph(state: State<'_, AppState>) -> AppResult<TopologyGraph> {
    let snapshot = collect(&state).await?;
    let database = Arc::clone(&state.database);
    let supervisor = state.supervisor.clone();
    blocking("GET_TOPOLOGY_GRAPH_TASK_FAILED", move || {
        let projects = database.list_projects().map_err(database_error)?;
        let runs = supervisor.list_runs().map_err(supervisor_error)?;
        Ok(topology::build_topology(snapshot, projects, runs))
    })
    .await
}

#[tauri::command]
pub async fn get_docker_status(state: State<'_, AppState>) -> AppResult<DockerStatus> {
    let snapshot = collect(&state).await?;
    blocking("GET_DOCKER_STATUS_TASK_FAILED", move || {
        Ok(docker::inspect_docker_status(&snapshot.processes))
    })
    .await
}

#[tauri::command]
pub async fn get_docker_inventory(state: State<'_, AppState>) -> AppResult<DockerInventory> {
    let snapshot = collect(&state).await?;
    let database = Arc::clone(&state.database);
    let docker_operations = Arc::clone(&state.docker_operations);
    blocking("GET_DOCKER_INVENTORY_TASK_FAILED", move || {
        let _operation = docker_operations
            .lock()
            .map_err(|_| docker_operation_error())?;
        let projects = database.list_projects().map_err(database_error)?;
        docker::list_docker_inventory(&projects, &snapshot.processes).map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub async fn list_containers(state: State<'_, AppState>) -> AppResult<Vec<DockerContainer>> {
    let database = Arc::clone(&state.database);
    blocking("LIST_CONTAINERS_TASK_FAILED", move || {
        let projects = database.list_projects().map_err(database_error)?;
        docker::list_containers(&projects).map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub async fn list_docker_networks() -> AppResult<Vec<DockerNetwork>> {
    blocking("LIST_DOCKER_NETWORKS_TASK_FAILED", move || {
        docker::list_networks().map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub async fn list_docker_volumes() -> AppResult<Vec<DockerVolume>> {
    blocking("LIST_DOCKER_VOLUMES_TASK_FAILED", move || {
        docker::list_volumes().map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub async fn get_container_logs(
    container_id: String,
    max_lines: u16,
) -> AppResult<Vec<DockerLogEntry>> {
    blocking("GET_CONTAINER_LOGS_TASK_FAILED", move || {
        docker::container_logs(&container_id, max_lines).map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub async fn container_action(
    request: DockerContainerActionRequest,
    state: State<'_, AppState>,
) -> AppResult<DockerContainerActionResult> {
    let database = Arc::clone(&state.database);
    blocking("CONTAINER_ACTION_TASK_FAILED", move || {
        let projects = database.list_projects().map_err(database_error)?;
        docker::container_action(request, &projects).map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub async fn list_compose_projects(state: State<'_, AppState>) -> AppResult<Vec<ComposeProject>> {
    let snapshot = collect(&state).await?;
    let database = Arc::clone(&state.database);
    let docker_operations = Arc::clone(&state.docker_operations);
    blocking("LIST_COMPOSE_PROJECTS_TASK_FAILED", move || {
        let _operation = docker_operations
            .lock()
            .map_err(|_| docker_operation_error())?;
        let projects = database.list_projects().map_err(database_error)?;
        let status = docker::inspect_docker_status(&snapshot.processes);
        let containers = if matches!(
            status.availability,
            crate::domain::DockerAvailability::Running
        ) {
            docker::list_containers(&projects).ok()
        } else {
            None
        };
        Ok(docker::list_compose_projects(
            &projects,
            &snapshot.ports,
            containers.as_deref(),
        ))
    })
    .await
}

#[tauri::command]
pub async fn parse_compose_project(
    project_id: String,
    compose_file: String,
    state: State<'_, AppState>,
) -> AppResult<ComposeProject> {
    let snapshot = collect(&state).await?;
    let database = Arc::clone(&state.database);
    blocking("PARSE_COMPOSE_PROJECT_TASK_FAILED", move || {
        let projects = database.list_projects().map_err(database_error)?;
        let project = projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| not_found("PROJECT_NOT_FOUND", "The project is not registered."))?;
        let status = docker::inspect_docker_status(&snapshot.processes);
        let containers = if matches!(
            status.availability,
            crate::domain::DockerAvailability::Running
        ) {
            docker::list_containers(&projects).ok()
        } else {
            None
        };
        docker::parse_compose_project(
            project,
            &compose_file,
            &snapshot.ports,
            containers.as_deref(),
        )
        .map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub async fn run_compose_doctor(
    project_id: String,
    compose_file: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<ComposeDoctorIssue>> {
    let snapshot = collect(&state).await?;
    let database = Arc::clone(&state.database);
    blocking("RUN_COMPOSE_DOCTOR_TASK_FAILED", move || {
        let projects = database.list_projects().map_err(database_error)?;
        let project = projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| not_found("PROJECT_NOT_FOUND", "The project is not registered."))?;
        let status = docker::inspect_docker_status(&snapshot.processes);
        let containers = if matches!(
            status.availability,
            crate::domain::DockerAvailability::Running
        ) {
            docker::list_containers(&projects).ok()
        } else {
            None
        };
        docker::run_compose_doctor(
            project,
            &compose_file,
            &snapshot.ports,
            containers.as_deref(),
        )
        .map_err(docker_error)
    })
    .await
}

#[tauri::command]
pub fn open_local_preview(app: tauri::AppHandle, url: String) -> AppResult<()> {
    let parsed = parse_preview_url(&url)?;
    let allowed_scheme = parsed.scheme().to_owned();
    let allowed_host = parsed.host_str().unwrap_or_default().to_owned();
    let allowed_port = parsed.port_or_known_default();
    let label = format!("preview-{}", Uuid::new_v4());

    tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::External(parsed.clone()))
        .title(format!("Mr Manager Preview - {parsed}"))
        .inner_size(1100.0, 780.0)
        .on_navigation(move |target| {
            is_allowed_preview_url(target)
                && target.scheme() == allowed_scheme
                && target.host_str() == Some(allowed_host.as_str())
                && target.port_or_known_default() == allowed_port
        })
        .build()
        .map_err(|error| {
            AppError::new(
                "LOCAL_PREVIEW_OPEN_FAILED",
                "Mr Manager could not open the isolated preview window.",
            )
            .with_safe_details(error.to_string())
            .with_remediation("Verify the local service is still running and retry.")
            .retryable()
        })?;

    Ok(())
}

async fn collect(state: &State<'_, AppState>) -> AppResult<CollectedSnapshot> {
    let collector = Arc::clone(&state.collector);
    tauri::async_runtime::spawn_blocking(move || {
        collector
            .lock()
            .map_err(|_| {
                AppError::new(
                    "COLLECTOR_LOCK_UNAVAILABLE",
                    "The system collector is temporarily unavailable.",
                )
                .with_remediation("Retry after the current collection completes.")
                .retryable()
            })
            .map(|mut collector| collector.collect())
    })
    .await
    .map_err(|error| {
        AppError::new(
            "COLLECTOR_TASK_FAILED",
            "The background system collection task failed.",
        )
        .with_safe_details(error.to_string())
        .with_remediation("Retry the snapshot. Restart Mr Manager if it persists.")
        .retryable()
    })?
}

async fn blocking<T, F>(code: &'static str, task: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| {
            AppError::new(code, "The background task failed.")
                .with_safe_details(error.to_string())
                .with_remediation("Retry the action. Restart Mr Manager if it persists.")
                .retryable()
        })?
}

fn start_task(
    registry: &TaskRegistry,
    id: String,
    kind: BackgroundTaskKind,
    label: impl Into<String>,
    route: impl Into<String>,
    cancellable: bool,
) -> AppResult<BackgroundTask> {
    Ok(registry.lock().map_err(|_| task_registry_error())?.start(
        id,
        kind,
        label,
        route,
        cancellable,
    ))
}

fn spawn_task<F>(registry: TaskRegistry, id: String, operation: F)
where
    F: FnOnce() -> AppResult<(BackgroundTaskState, String, BackgroundTaskOutput)> + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(operation).await;
        let Ok(mut manager) = registry.lock() else {
            return;
        };
        match result {
            Ok(Ok((state, summary, output))) => manager.complete(&id, state, summary, output),
            Ok(Err(error)) => manager.fail(&id, error),
            Err(error) => manager.fail(
                &id,
                AppError::new(
                    "BACKGROUND_TASK_FAILED",
                    "The background task stopped unexpectedly.",
                )
                .with_safe_details(error.to_string())
                .with_remediation("Retry the task. Restart Mr Manager if it persists.")
                .retryable(),
            ),
        }
    });
}

fn database_error(error: DatabaseError) -> AppError {
    AppError::new(
        "LOCAL_DATABASE_ERROR",
        "Mr Manager could not access its local settings.",
    )
    .with_safe_details(error.to_string())
    .with_remediation("Close other Mr Manager instances and retry.")
    .retryable()
}

fn task_registry_error() -> AppError {
    AppError::new(
        "BACKGROUND_TASK_REGISTRY_UNAVAILABLE",
        "The background task registry is temporarily unavailable.",
    )
    .with_remediation("Retry the action. Restart Mr Manager if it persists.")
    .retryable()
}

fn docker_operation_error() -> AppError {
    AppError::new(
        "DOCKER_REFRESH_UNAVAILABLE",
        "The Docker refresh coordinator is temporarily unavailable.",
    )
    .with_remediation("Retry the Docker refresh.")
    .retryable()
}

fn cleaner_registry_error() -> AppError {
    AppError::new(
        "CLEANER_REGISTRY_UNAVAILABLE",
        "The Cleaner operation registry is temporarily unavailable.",
    )
    .with_remediation("Retry after the current Cleaner operation completes.")
    .retryable()
}

fn cleanup_mutation_error() -> AppError {
    AppError::new(
        "CLEANER_MUTATION_UNAVAILABLE",
        "Another quarantine, restore, or purge operation is already being finalized.",
    )
    .with_remediation("Wait for the current Cleaner action to finish, then retry.")
    .retryable()
}

fn cleaner_error(error: CleanerError) -> AppError {
    match error {
        CleanerError::InvalidRequest(details) => AppError::new(
            "INVALID_CLEANER_REQUEST",
            "The Cleaner request was not accepted.",
        )
        .with_safe_details(details)
        .with_remediation("Refresh Cleaner evidence and review the selection again."),
        CleanerError::UnsafeRoot(details) => AppError::new(
            "UNSAFE_CLEANER_ROOT",
            "Mr Manager will not scan or mutate that filesystem root.",
        )
        .with_safe_details(details)
        .with_remediation("Choose a specific project or workspace folder that is not a system or application-data root."),
        CleanerError::ScanNotFound => AppError::new(
            "CLEANUP_SCAN_EXPIRED",
            "The server-side cleanup scan is no longer available.",
        )
        .with_remediation("Run a fresh scan before creating a cleanup plan."),
        CleanerError::PlanNotFound => AppError::new(
            "CLEANUP_PLAN_NOT_FOUND",
            "The reviewed cleanup plan no longer exists.",
        )
        .with_remediation("Create a new plan from a fresh scan."),
        CleanerError::ManifestNotFound => AppError::new(
            "QUARANTINE_ITEM_NOT_FOUND",
            "The quarantine manifest or item no longer exists.",
        )
        .with_remediation("Refresh the quarantine history."),
        CleanerError::ConfirmationMismatch(expected) => AppError::new(
            "CLEANER_CONFIRMATION_MISMATCH",
            "The typed confirmation did not match the required phrase.",
        )
        .with_safe_details(format!("Expected confirmation: {expected}"))
        .with_remediation("Type the exact case-sensitive phrase shown in the confirmation panel."),
        CleanerError::PlanAlreadyExecuted => AppError::new(
            "CLEANUP_PLAN_ALREADY_EXECUTED",
            "An immutable cleanup plan can be executed only once.",
        )
        .with_remediation("Refresh the quarantine history or create a new plan."),
        CleanerError::ItemChanged(path) => AppError::new(
            "CLEANUP_ITEM_CHANGED",
            "A cleanup item changed after review, so Mr Manager left it in place.",
        )
        .with_safe_details(path)
        .with_remediation("Run a fresh scan and review a new immutable plan."),
        CleanerError::PartialFailure(details) => AppError::new(
            "QUARANTINE_PARTIAL_FAILURE",
            "A verified quarantine copy exists, but the original remained in place.",
        )
        .with_safe_details(details)
        .with_remediation("Review the manifest. You may purge only the managed copy after confirming the original still exists."),
        CleanerError::CopyFallbackRefused(details) => AppError::new(
            "QUARANTINE_COPY_FALLBACK_REFUSED",
            "Mr Manager could not use a safe verified-copy fallback, so the original stayed in place.",
        )
        .with_safe_details(details)
        .with_remediation("Check locks, permissions, and destination free space, then run a fresh scan before retrying."),
        CleanerError::RestoreConflict(alternative) => AppError::new(
            "RESTORE_DESTINATION_OCCUPIED",
            "The original path is occupied and was not overwritten.",
        )
        .with_safe_details(format!("Safe alternative: {alternative}"))
        .with_remediation("Choose Restore beside existing to use the server-generated conflict-safe path."),
        CleanerError::Io { action, source } => AppError::new(
            "CLEANER_FILESYSTEM_ERROR",
            "Mr Manager could not complete the filesystem operation safely.",
        )
        .with_safe_details(format!("{action}: {source}"))
        .with_remediation("Close tools using the item, verify permissions and free space, then run a fresh scan before retrying.")
        .retryable(),
    }
}

fn project_scan_error(error: ProjectScanError) -> AppError {
    match error {
        ProjectScanError::InvalidRoot(message) => AppError::new(
            "INVALID_PROJECT_ROOT",
            "Mr Manager cannot scan the selected project root.",
        )
        .with_safe_details(message)
        .with_remediation("Choose a specific project folder or a smaller parent folder."),
        ProjectScanError::OperationAlreadyRunning => AppError::new(
            "PROJECT_SCAN_ALREADY_RUNNING",
            "A project scan with this operation id is already running.",
        )
        .with_remediation("Wait for the scan to finish or cancel it before starting another."),
        ProjectScanError::RegistryUnavailable => AppError::new(
            "PROJECT_SCAN_REGISTRY_UNAVAILABLE",
            "The project scan registry is temporarily unavailable.",
        )
        .with_remediation("Retry the scan."),
        ProjectScanError::Io { action, source } => AppError::new(
            "PROJECT_SCAN_IO_ERROR",
            "Mr Manager could not read the selected project folder.",
        )
        .with_safe_details(format!("{action}: {source}"))
        .with_remediation("Check that the folder still exists and is readable.")
        .retryable(),
    }
}

fn supervisor_error(error: SupervisorError) -> AppError {
    match error {
        SupervisorError::LockUnavailable => AppError::new(
            "SUPERVISOR_UNAVAILABLE",
            "The managed process supervisor is temporarily unavailable.",
        )
        .with_remediation("Retry the action."),
        SupervisorError::ScriptNotFound => AppError::new(
            "PROJECT_SCRIPT_NOT_FOUND",
            "The selected project command is no longer available.",
        )
        .with_remediation("Rescan the project and choose one of the detected commands."),
        SupervisorError::UnsafeWorkingDirectory => AppError::new(
            "UNSAFE_COMMAND_WORKING_DIRECTORY",
            "The command working directory is outside the registered project root.",
        )
        .with_remediation("Rescan the project or remove and re-add it from the correct folder."),
        SupervisorError::EmptyExecutable => AppError::new(
            "EMPTY_COMMAND_EXECUTABLE",
            "The detected command executable is empty.",
        )
        .with_remediation("Rescan the project and verify the detected scripts."),
        SupervisorError::Spawn(source) => AppError::new(
            "PROJECT_COMMAND_SPAWN_FAILED",
            "Mr Manager could not start the project command.",
        )
        .with_safe_details(source.to_string())
        .with_remediation("Check that the executable is installed and available on PATH.")
        .retryable(),
        SupervisorError::JobObject(details) => AppError::new(
            "PROJECT_COMMAND_JOB_OBJECT_FAILED",
            "Mr Manager could not attach the process to a managed Windows Job Object.",
        )
        .with_safe_details(details)
        .with_remediation(
            "The command was not left running unmanaged. Retry from a normal user session.",
        ),
        SupervisorError::RunNotFound => AppError::new(
            "MANAGED_PROCESS_NOT_FOUND",
            "The managed process run does not exist.",
        )
        .with_remediation("Refresh the managed process list and retry."),
        SupervisorError::Stop(source) => AppError::new(
            "MANAGED_PROCESS_STOP_FAILED",
            "Mr Manager could not deliver the stop request.",
        )
        .with_safe_details(source.to_string())
        .with_remediation("Retry with force stop if the process is still running.")
        .retryable(),
    }
}

fn docker_error(error: DockerError) -> AppError {
    match error {
        DockerError::CliMissing => AppError::new(
            "DOCKER_CLI_MISSING",
            "Docker CLI is not installed or not available on PATH.",
        )
        .with_remediation("Install Docker Desktop or add docker.exe to PATH."),
        DockerError::Timeout => AppError::new(
            "DOCKER_COMMAND_TIMEOUT",
            "Docker did not respond before Mr Manager's safety timeout.",
        )
        .with_remediation("Start Docker Desktop fully, then retry.")
        .retryable(),
        DockerError::CommandFailed {
            status_code,
            stderr,
        } => {
            let lower = stderr.to_ascii_lowercase();
            let mut error = AppError::new(
                "DOCKER_COMMAND_FAILED",
                "Docker returned an error for the requested operation.",
            )
            .with_safe_details(format!(
                "exit={}; {}",
                status_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_owned()),
                stderr
            ))
            .with_remediation(
                "Verify Docker Desktop is running and the selected object still exists.",
            )
            .retryable();
            if lower.contains("permission denied") || lower.contains("access is denied") {
                error = error.permission_relevant();
            }
            error
        }
        DockerError::Io(source) => AppError::new(
            "DOCKER_COMMAND_IO_ERROR",
            "Mr Manager could not start the Docker CLI operation.",
        )
        .with_safe_details(source.to_string())
        .with_remediation("Verify Docker is installed for this Windows user.")
        .retryable(),
        DockerError::Parse(message) => AppError::new(
            "DOCKER_OUTPUT_PARSE_ERROR",
            "Mr Manager could not parse Docker's structured output.",
        )
        .with_safe_details(message)
        .with_remediation("Refresh the Docker page. If it persists, capture diagnostics."),
        DockerError::InvalidContainerReference => AppError::new(
            "INVALID_DOCKER_CONTAINER_REFERENCE",
            "The Docker container reference was not accepted.",
        )
        .with_remediation("Refresh the Docker page and choose a listed container."),
        DockerError::InvalidConfirmation { expected } => AppError::new(
            "DOCKER_CONFIRMATION_MISMATCH",
            "The Docker action confirmation did not match the required phrase.",
        )
        .with_safe_details(format!("Expected confirmation: {expected}"))
        .with_remediation("Type the exact confirmation phrase shown in the dialog."),
        DockerError::InvalidComposePath(message) => AppError::new(
            "INVALID_COMPOSE_PATH",
            "The Compose file path is outside the registered project scope or is unsafe.",
        )
        .with_safe_details(message)
        .with_remediation("Rescan the project and choose a Compose file detected under its root."),
        DockerError::ComposeIo(source) => AppError::new(
            "COMPOSE_FILE_READ_FAILED",
            "Mr Manager could not read the Compose file.",
        )
        .with_safe_details(source.to_string())
        .with_remediation("Verify the file still exists and is readable.")
        .retryable(),
    }
}

fn integration_error(error: IntegrationError) -> AppError {
    match error {
        IntegrationError::UnknownDetector(detector_id) => AppError::new(
            "UNKNOWN_INTEGRATION_DETECTOR",
            "Mr Manager does not know that integration detector.",
        )
        .with_safe_details(detector_id)
        .with_remediation("Refresh the integrations page and choose a listed detector."),
        IntegrationError::Timeout => AppError::new(
            "INTEGRATION_PROBE_TIMEOUT",
            "The integration probe timed out.",
        )
        .with_remediation("Retry after the local tool finishes starting or exiting.")
        .retryable(),
        IntegrationError::Io(source) => AppError::new(
            "INTEGRATION_PROBE_IO_ERROR",
            "Mr Manager could not run the exact integration probe.",
        )
        .with_safe_details(source.to_string())
        .with_remediation("Verify the local tool is installed and accessible to this Windows user.")
        .retryable(),
    }
}

fn network_error(error: NetworkError) -> AppError {
    match error {
        NetworkError::ExternalDiagnosticsDisabled => AppError::new(
            "EXTERNAL_NETWORK_DIAGNOSTICS_DISABLED",
            "External internet diagnostics are disabled.",
        )
        .with_remediation(
            "Enable external network checks in Settings, then click Run Internet Test again.",
        ),
        NetworkError::ExternalConsentMissing => AppError::new(
            "EXTERNAL_NETWORK_CONSENT_REQUIRED",
            "Running this diagnostic requires explicit consent to contact external servers.",
        )
        .with_remediation("Use the clearly labelled Run Internet Test action."),
        NetworkError::Timeout => AppError::new(
            "NETWORK_DIAGNOSTIC_TIMEOUT",
            "The network diagnostic timed out.",
        )
        .with_remediation("Retry when the network is less busy.")
        .retryable(),
        NetworkError::Io(source) => AppError::new(
            "NETWORK_DIAGNOSTIC_IO_ERROR",
            "Mr Manager could not run the network diagnostic.",
        )
        .with_safe_details(source.to_string())
        .with_remediation("Retry from a normal Windows user session.")
        .retryable(),
    }
}

fn system_diagnostics_error(error: SystemDiagnosticsError) -> AppError {
    match error {
        SystemDiagnosticsError::RecordingAlreadyActive => AppError::new(
            "METRIC_RECORDING_ALREADY_ACTIVE",
            "A metric recording session is already active.",
        )
        .with_remediation("Stop or delete the active recording before starting another."),
        SystemDiagnosticsError::NoActiveRecording => AppError::new(
            "NO_ACTIVE_METRIC_RECORDING",
            "No metric recording session is active.",
        )
        .with_remediation("Start a recording session first."),
        SystemDiagnosticsError::InvalidRecordingName => AppError::new(
            "INVALID_METRIC_RECORDING_NAME",
            "The recording name must be 1-80 characters.",
        )
        .with_remediation("Choose a short local-only session name."),
        SystemDiagnosticsError::InvalidAnnotation => AppError::new(
            "INVALID_METRIC_ANNOTATION",
            "The annotation must be 1-300 characters.",
        )
        .with_remediation("Enter a short note about what happened."),
        SystemDiagnosticsError::Timeout => {
            AppError::new("GPU_PROVIDER_TIMEOUT", "The GPU provider timed out.")
                .with_remediation("Retry after GPU driver tooling responds.")
                .retryable()
        }
        SystemDiagnosticsError::Io(source) => {
            AppError::new("GPU_PROVIDER_IO_ERROR", "The GPU provider could not run.")
                .with_safe_details(source.to_string())
                .with_remediation(
                    "Verify supported GPU tooling is installed for this Windows user.",
                )
                .retryable()
        }
    }
}

fn recorder_lock_error() -> AppError {
    AppError::new(
        "WAR_ROOM_UNAVAILABLE",
        "The System Diagnostics recorder is temporarily unavailable.",
    )
    .with_remediation("Retry after the current recorder operation completes.")
    .retryable()
}

fn recording_not_found(session_id: &str) -> AppError {
    AppError::new(
        "METRIC_RECORDING_NOT_FOUND",
        "The metric recording was not found.",
    )
    .with_safe_details(session_id.to_owned())
    .with_remediation("Refresh the recording list and choose an existing session.")
}

fn not_found(code: &'static str, message: &'static str) -> AppError {
    AppError::new(code, message).with_remediation("Refresh the project list and retry.")
}

fn parse_preview_url(value: &str) -> AppResult<tauri::Url> {
    let url = tauri::Url::parse(value).map_err(|error| {
        AppError::new(
            "INVALID_LOCAL_PREVIEW_URL",
            "The preview URL is not a valid HTTP or HTTPS URL.",
        )
        .with_safe_details(error.to_string())
        .with_remediation("Choose one of the URLs generated by Mr Manager.")
    })?;
    if is_allowed_preview_url(&url) {
        Ok(url)
    } else {
        Err(AppError::new(
            "LOCAL_PREVIEW_URL_NOT_ALLOWED",
            "Mr Manager previews only loopback and private-network HTTP(S) URLs.",
        )
        .with_remediation("Choose a loopback or private LAN URL generated by Mr Manager."))
    }
}

fn is_allowed_preview_url(url: &tauri::Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(is_loopback_or_private_host)
}

fn is_loopback_or_private_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(ip) => {
                ip.is_loopback()
                    || ip.is_private()
                    || ip.octets()[0] == 169 && ip.octets()[1] == 254
            }
            std::net::IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local(),
        };
    }
    false
}
