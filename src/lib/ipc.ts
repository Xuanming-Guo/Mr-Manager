import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppSettings,
  CapabilityReport,
  OverviewSnapshot,
  PortEndpoint,
  ProcessSnapshot,
  SystemSnapshot,
} from "../types/system";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isDesktopRuntime(): boolean {
  return typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;
}

function runtimeRequiredError(): AppError {
  return {
    code: "DESKTOP_RUNTIME_REQUIRED",
    message: "Real system data is available only in the Mr Manager application.",
    remediation: "Run `npm run tauri dev` or launch the installed Windows application.",
    technicalDetails: null,
    retryable: false,
    permissionRelevant: false,
  };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw runtimeRequiredError();
  }
  return invoke<T>(command, args);
}

export const desktopApi = {
  getOverview: () => call<OverviewSnapshot>("get_overview_snapshot"),
  getSystem: () => call<SystemSnapshot>("get_system_snapshot"),
  listProcesses: () => call<ProcessSnapshot[]>("list_processes"),
  listPorts: () => call<PortEndpoint[]>("list_ports"),
  getSettings: () => call<AppSettings>("get_app_settings"),
  updateSettings: (settings: AppSettings) => call<AppSettings>("update_app_settings", { settings }),
  getCapabilities: () => call<CapabilityReport>("get_capability_report"),
};

export function normalizeAppError(error: unknown): AppError {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    return error as AppError;
  }
  return {
    code: "UNEXPECTED_ERROR",
    message: error instanceof Error ? error.message : "An unexpected error occurred.",
    remediation: "Retry the operation. If it persists, open Diagnostics.",
    technicalDetails: null,
    retryable: true,
    permissionRelevant: false,
  };
}
