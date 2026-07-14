import { invoke } from "@tauri-apps/api/core";
import type {
  ComposeDoctorIssue,
  ComposeProject,
  DockerContainer,
  DockerContainerActionRequest,
  DockerContainerActionResult,
  DockerInventory,
  DockerLogEntry,
  DockerNetwork,
  DockerStatus,
  DockerVolume,
} from "../types/docker";
import { isDesktopRuntime, normalizeAppError } from "./ipc";

async function dockerCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "Docker data is available only in the Mr Manager application.",
      remediation: "Launch the Tauri desktop application to inspect local Docker state.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: false,
    });
  }
  return invoke<T>(command, args);
}

export const dockerApi = {
  status: () => dockerCall<DockerStatus>("get_docker_status"),
  inventory: () => dockerCall<DockerInventory>("get_docker_inventory"),
  containers: () => dockerCall<DockerContainer[]>("list_containers"),
  networks: () => dockerCall<DockerNetwork[]>("list_docker_networks"),
  volumes: () => dockerCall<DockerVolume[]>("list_docker_volumes"),
  logs: (containerId: string, maxLines = 160) =>
    dockerCall<DockerLogEntry[]>("get_container_logs", { containerId, maxLines }),
  action: (request: DockerContainerActionRequest) =>
    dockerCall<DockerContainerActionResult>("container_action", { request }),
  composeProjects: () => dockerCall<ComposeProject[]>("list_compose_projects"),
  parseCompose: (projectId: string, composeFile: string) =>
    dockerCall<ComposeProject>("parse_compose_project", { projectId, composeFile }),
  doctor: (projectId: string, composeFile: string) =>
    dockerCall<ComposeDoctorIssue[]>("run_compose_doctor", { projectId, composeFile }),
};
