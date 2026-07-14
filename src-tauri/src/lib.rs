mod cleaner;
mod collector;
mod commands;
mod db;
mod docker;
mod domain;
mod integrations;
mod logging;
mod networking;
mod platform;
mod projects;
mod security;
mod supervisor;
mod system_diagnostics;
mod tasks;
mod topology;

use std::io;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let log_directory = app.path().app_log_dir()?;
            let logging_guard = logging::initialize(&log_directory)
                .map_err(|error| io::Error::other(error.to_string()))?;
            // Keep the legacy filename so existing local data remains available after rebranding.
            let database = db::Database::open(&app_data.join("desktop-manager.sqlite3"))
                .map_err(|error| io::Error::other(error.to_string()))?;

            app.manage(AppState::new(
                database,
                logging_guard,
                app_data.join("quarantine"),
            ));
            tracing::info!("Mr Manager initialized in local-only mode");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_background_tasks,
            commands::get_background_task,
            commands::cancel_background_task,
            commands::get_overview_snapshot,
            commands::get_system_snapshot,
            commands::get_system_diagnostics_snapshot,
            commands::list_processes,
            commands::list_ports,
            commands::list_integrations,
            commands::probe_integration,
            commands::get_ollama_status,
            commands::get_wsl_status,
            commands::get_network_snapshot,
            commands::run_network_diagnostic,
            commands::start_internet_test_task,
            commands::start_metric_recording,
            commands::add_metric_annotation,
            commands::stop_metric_recording,
            commands::list_metric_sessions,
            commands::get_metric_recording,
            commands::export_metric_recording,
            commands::delete_metric_recording,
            commands::scan_cleanup_candidates,
            commands::start_cleanup_scan_task,
            commands::cancel_cleanup_scan,
            commands::create_cleanup_plan,
            commands::execute_quarantine_plan,
            commands::start_quarantine_task,
            commands::list_quarantine_manifests,
            commands::restore_quarantine_item,
            commands::purge_quarantine_item,
            commands::purge_quarantine_manifest,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::get_capability_report,
            commands::list_projects,
            commands::add_project,
            commands::add_project_root,
            commands::start_project_discovery_task,
            commands::scan_project,
            commands::remove_project,
            commands::cancel_project_scan,
            commands::update_project_metadata,
            commands::run_project_command,
            commands::list_managed_processes,
            commands::get_managed_process_logs,
            commands::stop_managed_process,
            commands::get_topology_graph,
            commands::get_docker_status,
            commands::get_docker_inventory,
            commands::list_containers,
            commands::list_docker_networks,
            commands::list_docker_volumes,
            commands::get_container_logs,
            commands::container_action,
            commands::list_compose_projects,
            commands::parse_compose_project,
            commands::run_compose_doctor,
            commands::open_local_preview,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Mr Manager");
}
