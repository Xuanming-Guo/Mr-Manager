import type { AvailabilityState } from "../types/system";

const labels: Record<AvailabilityState, string> = {
  available: "Available",
  unavailable: "Unavailable",
  unsupported: "Unsupported",
  permissionDenied: "Permission denied",
  error: "Error",
};

export function AvailabilityBadge({ state }: { state: AvailabilityState }) {
  return <span className={`availability-badge availability-${state}`}>{labels[state]}</span>;
}
