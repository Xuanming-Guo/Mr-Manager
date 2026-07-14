import { useQuery } from "@tanstack/react-query";
import { desktopApi } from "../lib/ipc";
import type { RefreshMode } from "../types/system";

export function refreshInterval(mode: RefreshMode | undefined): number {
  return mode === "fast" ? 1_000 : 2_500;
}

export function useSettingsQuery() {
  return useQuery({
    queryKey: ["settings"],
    queryFn: desktopApi.getSettings,
    staleTime: 30_000,
  });
}

export function useOverviewQuery(mode: RefreshMode | undefined) {
  return useQuery({
    queryKey: ["overview"],
    queryFn: desktopApi.getOverview,
    refetchInterval: refreshInterval(mode),
  });
}

export function useProcessesQuery(mode: RefreshMode | undefined) {
  return useQuery({
    queryKey: ["processes"],
    queryFn: desktopApi.listProcesses,
    refetchInterval: refreshInterval(mode),
  });
}

export function usePortsQuery(mode: RefreshMode | undefined) {
  return useQuery({
    queryKey: ["ports"],
    queryFn: desktopApi.listPorts,
    refetchInterval: refreshInterval(mode),
  });
}

export function useCapabilitiesQuery() {
  return useQuery({
    queryKey: ["capabilities"],
    queryFn: desktopApi.getCapabilities,
    staleTime: 60_000,
  });
}
