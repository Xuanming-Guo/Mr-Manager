import { invoke } from "@tauri-apps/api/core";
import type { TopologyGraph } from "../types/topology";
import { isDesktopRuntime, normalizeAppError } from "./ipc";

async function topologyCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktopRuntime()) {
    throw normalizeAppError({
      code: "DESKTOP_RUNTIME_REQUIRED",
      message: "Topology is available only in the Mr Manager application.",
      remediation: "Launch the Tauri desktop application to inspect local relationships.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: false,
    });
  }
  return invoke<T>(command, args);
}

export const topologyApi = {
  graph: () => topologyCall<TopologyGraph>("get_topology_graph"),
  openPreview: async (url: string) => {
    await topologyCall<unknown>("open_local_preview", { url });
  },
};
