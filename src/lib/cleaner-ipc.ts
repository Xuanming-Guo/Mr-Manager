import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  CleanupPlan,
  CleanupScanRequest,
  CleanupScanResult,
  QuarantineManifest,
  RestoreConflictStrategy,
} from "../types/cleaner";
import { isDesktopRuntime, normalizeAppError } from "./ipc";

async function cleanerCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "Cleaner can inspect local folders only inside the Mr Manager application.",
      remediation: "Launch the Tauri desktop application to choose an explicit folder.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: false,
    });
  }
  return invoke<T>(command, args);
}

export const cleanerApi = {
  scan: (request: CleanupScanRequest) =>
    cleanerCall<CleanupScanResult>("scan_cleanup_candidates", { request }),
  cancelScan: (operationId: string) => cleanerCall<unknown>("cancel_cleanup_scan", { operationId }),
  createPlan: (scanId: string, candidateIds: string[]) =>
    cleanerCall<CleanupPlan>("create_cleanup_plan", {
      request: { scanId, candidateIds },
    }),
  executePlan: (planId: string, confirmation: string) =>
    cleanerCall<QuarantineManifest>("execute_quarantine_plan", {
      request: { planId, confirmation },
    }),
  manifests: () => cleanerCall<QuarantineManifest[]>("list_quarantine_manifests"),
  restore: (manifestId: string, itemId: string, conflictStrategy: RestoreConflictStrategy) =>
    cleanerCall<QuarantineManifest>("restore_quarantine_item", {
      request: { manifestId, itemId, conflictStrategy },
    }),
  purge: (manifestId: string, itemId: string, confirmation: string) =>
    cleanerCall<QuarantineManifest>("purge_quarantine_item", {
      request: { manifestId, itemId, confirmation },
    }),
  purgeManifest: (manifestId: string, confirmation: string) =>
    cleanerCall<QuarantineManifest>("purge_quarantine_manifest", {
      request: { manifestId, confirmation },
    }),
};

export async function selectCleanupFolders(): Promise<string[]> {
  if (!isDesktopRuntime()) throw new Error("Desktop runtime required");
  const selection = await open({
    directory: true,
    multiple: true,
    title: "Choose explicit project or workspace folders to scan",
  });
  if (typeof selection === "string") return [selection];
  return selection ?? [];
}
