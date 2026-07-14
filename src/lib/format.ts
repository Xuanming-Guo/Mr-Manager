const byteUnits = ["B", "KB", "MB", "GB", "TB"] as const;

export function formatBytes(value: number, precision = 1): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), byteUnits.length - 1);
  return `${(value / 1024 ** index).toFixed(index === 0 ? 0 : precision)} ${byteUnits[index]}`;
}

export function formatPercent(value: number): string {
  return `${Math.max(0, value).toFixed(1)}%`;
}

export function formatDuration(seconds: number): string {
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  return [days ? `${days}d` : "", hours ? `${hours}h` : "", `${minutes}m`]
    .filter(Boolean)
    .join(" ");
}

export function formatTimestamp(epochMs: number): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(epochMs));
}
