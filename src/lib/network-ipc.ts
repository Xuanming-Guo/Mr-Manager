import { invoke } from "@tauri-apps/api/core";
import type {
  NetworkDashboardSnapshot,
  NetworkDiagnosticReport,
  NetworkDiagnosticRequest,
} from "../types/network";
import { isDesktopRuntime, normalizeAppError } from "./ipc";

async function networkCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "Network data is available only in the Mr Manager application.",
      remediation: "Launch the Tauri desktop application to inspect local network state.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: false,
    });
  }
  return invoke<T>(command, args);
}

export const networkApi = {
  snapshot: () => networkCall<NetworkDashboardSnapshot>("get_network_snapshot"),
  diagnostic: (request: NetworkDiagnosticRequest) =>
    networkCall<NetworkDiagnosticReport>("run_network_diagnostic", { request }),
};
