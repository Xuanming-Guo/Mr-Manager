import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ManagedCommand,
  ManagedCommandLogEntry,
  Project,
  ProjectDiscoveryResult,
} from "../types/projects";
import { isDesktopRuntime, normalizeAppError } from "./ipc";

async function projectCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "Project folders can be inspected only by the Mr Manager application.",
      remediation: "Launch the Tauri desktop application to select a local folder.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: false,
    });
  }
  return invoke<T>(command, args);
}

export const projectsApi = {
  list: () => projectCall<Project[]>("list_projects"),
  add: (path: string) => projectCall<Project>("add_project", { path }),
  discoverRoot: (rootPath: string, operationId: string, maximumDepth = 4) =>
    projectCall<ProjectDiscoveryResult>("add_project_root", {
      rootPath,
      operationId,
      maximumDepth,
    }),
  rescan: (projectId: string) => projectCall<Project>("scan_project", { projectId }),
  remove: async (projectId: string) => {
    await projectCall<unknown>("remove_project", { projectId });
  },
  cancelScan: async (operationId: string) => {
    await projectCall<unknown>("cancel_project_scan", { operationId });
  },
  runCommand: (projectId: string, scriptId: string) =>
    projectCall<ManagedCommand>("run_project_command", { projectId, scriptId }),
  listManaged: () => projectCall<ManagedCommand[]>("list_managed_processes"),
  logs: (runId: string) =>
    projectCall<ManagedCommandLogEntry[]>("get_managed_process_logs", { runId }),
  stop: (runId: string, force: boolean) =>
    projectCall<ManagedCommand>("stop_managed_process", { runId, force }),
};

export async function selectProjectFolder(
  title = "Choose a project folder",
): Promise<string | null> {
  if (!isDesktopRuntime()) {
    throw new Error("Desktop runtime required");
  }
  const selection = await open({
    directory: true,
    multiple: false,
    title,
  });
  return typeof selection === "string" ? selection : null;
}
