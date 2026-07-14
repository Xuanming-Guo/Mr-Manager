import { invoke } from "@tauri-apps/api/core";
import type { IntegrationStatus, OllamaStatus, WslStatus } from "../types/integrations";
import { isDesktopRuntime, normalizeAppError } from "./ipc";

async function integrationsCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "Integration data is available only in the Mr Manager application.",
      remediation: "Launch the Tauri desktop application to inspect local tools and services.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: false,
    });
  }
  return invoke<T>(command, args);
}

export const integrationsApi = {
  list: () => integrationsCall<IntegrationStatus[]>("list_integrations"),
  probe: (detectorId: string) =>
    integrationsCall<IntegrationStatus>("probe_integration", { detectorId }),
  ollama: () => integrationsCall<OllamaStatus>("get_ollama_status"),
  wsl: () => integrationsCall<WslStatus>("get_wsl_status"),
};
