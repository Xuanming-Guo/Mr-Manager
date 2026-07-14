import { invoke } from "@tauri-apps/api/core";
import { isDesktopRuntime, normalizeAppError } from "./ipc";
import type { CleanupScanRequest } from "../types/cleaner";
import type {
  BackgroundTask,
  BackgroundTaskDetail,
  StartProjectDiscoveryTaskRequest,
  StartQuarantineTaskRequest,
} from "../types/tasks";

async function taskCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "Background operations require the Mr Manager Windows application.",
    });
  }
  return invoke<T>(command, args);
}

export const taskApi = {
  list: () => taskCall<BackgroundTask[]>("list_background_tasks"),
  get: (taskId: string) => taskCall<BackgroundTaskDetail>("get_background_task", { taskId }),
  cancel: async (taskId: string) => {
    await taskCall<unknown>("cancel_background_task", { taskId });
  },
  startCleanupScan: (request: CleanupScanRequest) =>
    taskCall<BackgroundTask>("start_cleanup_scan_task", { request }),
  startQuarantine: (request: StartQuarantineTaskRequest) =>
    taskCall<BackgroundTask>("start_quarantine_task", { request }),
  startProjectDiscovery: ({
    rootPath,
    operationId,
    maximumDepth = 4,
  }: StartProjectDiscoveryTaskRequest) =>
    taskCall<BackgroundTask>("start_project_discovery_task", {
      rootPath,
      operationId,
      maximumDepth,
    }),
  startInternetTest: () => taskCall<BackgroundTask>("start_internet_test_task"),
};
