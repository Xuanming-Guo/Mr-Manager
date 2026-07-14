import { invoke } from "@tauri-apps/api/core";
import type {
  AddMetricAnnotationRequest,
  MetricRecordingDetail,
  MetricRecordingExport,
  RecordingAnnotation,
  RecordingSessionSummary,
  StartMetricRecordingRequest,
  SystemDiagnosticsSnapshot,
} from "../types/system-diagnostics";
import { isDesktopRuntime, normalizeAppError } from "./ipc";

async function systemDiagnosticsCall<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "System Diagnostics data is available only in the Mr Manager application.",
      remediation: "Launch the Tauri desktop application to inspect and record local metrics.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: false,
    });
  }
  return invoke<T>(command, args);
}

export const systemDiagnosticsApi = {
  snapshot: () =>
    systemDiagnosticsCall<SystemDiagnosticsSnapshot>("get_system_diagnostics_snapshot"),
  startRecording: (request: StartMetricRecordingRequest) =>
    systemDiagnosticsCall<RecordingSessionSummary>("start_metric_recording", { request }),
  addAnnotation: (request: AddMetricAnnotationRequest) =>
    systemDiagnosticsCall<RecordingAnnotation>("add_metric_annotation", { request }),
  stopRecording: () => systemDiagnosticsCall<RecordingSessionSummary>("stop_metric_recording"),
  sessions: () => systemDiagnosticsCall<RecordingSessionSummary[]>("list_metric_sessions"),
  detail: (sessionId: string) =>
    systemDiagnosticsCall<MetricRecordingDetail>("get_metric_recording", { sessionId }),
  export: (sessionId: string) =>
    systemDiagnosticsCall<MetricRecordingExport>("export_metric_recording", { sessionId }),
  delete: async (sessionId: string) => {
    await systemDiagnosticsCall<unknown>("delete_metric_recording", { sessionId });
  },
};
