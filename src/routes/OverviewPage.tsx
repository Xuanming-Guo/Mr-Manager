import {
  Activity,
  Cpu,
  HardDrive,
  MemoryStick,
  Network,
  Radio,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { Link } from "react-router-dom";
import { AvailabilityBadge } from "../components/AvailabilityBadge";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { MetricCard } from "../components/MetricCard";
import { formatBytes, formatDuration, formatPercent, formatTimestamp } from "../lib/format";
import { useOverviewQuery, useSettingsQuery } from "../app/queries";

export function OverviewPage() {
  const settings = useSettingsQuery();
  const overview = useOverviewQuery(settings.data?.refreshMode);

  if (overview.isLoading) return <LoadingState />;
  if (overview.error || !overview.data)
    return <ErrorState error={overview.error} retry={() => void overview.refetch()} />;

  const { system, processes, ports, collectorIssues } = overview.data;
  const memoryPercent =
    system.memory.totalBytes > 0 ? (system.memory.usedBytes / system.memory.totalBytes) * 100 : 0;
  const primaryDisk = system.disks[0];
  const diskPercent =
    primaryDisk && primaryDisk.totalBytes > 0
      ? ((primaryDisk.totalBytes - primaryDisk.availableBytes) / primaryDisk.totalBytes) * 100
      : 0;
  const networkRate =
    system.network.receivedBytesPerSecond + system.network.transmittedBytesPerSecond;

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Live Windows state</span>
          <h1>Operational overview</h1>
        </div>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => void overview.refetch()}
          disabled={overview.isFetching}
        >
          <RefreshCw size={15} className={overview.isFetching ? "spin" : ""} /> Refresh
        </button>
      </header>

      <div className="snapshot-strip">
        <div>
          <span className="status-dot" />
          Live snapshot #{system.sequence}
        </div>
        <span>
          {system.operatingSystem}
          {system.operatingSystemVersion ? ` ${system.operatingSystemVersion}` : ""}
        </span>
        <span>Uptime {formatDuration(system.uptimeSeconds)}</span>
        <span>
          Battery{" "}
          {system.battery.percentage !== null
            ? `${system.battery.percentage}%`
            : system.battery.availability.state}
          {system.battery.acOnline === true
            ? " · AC"
            : system.battery.acOnline === false
              ? " · battery"
              : ""}
        </span>
        <span>Updated {formatTimestamp(system.collectedAtMs)}</span>
      </div>

      <section className="metric-grid" aria-label="System metrics">
        <MetricCard
          label="CPU"
          value={formatPercent(system.cpu.totalUsagePercent)}
          detail={`${system.cpu.logicalCoreCount} logical cores`}
          icon={Cpu}
          percent={system.cpu.totalUsagePercent}
        />
        <MetricCard
          label="Memory"
          value={formatBytes(system.memory.usedBytes)}
          detail={`${formatBytes(system.memory.availableBytes)} available`}
          icon={MemoryStick}
          percent={memoryPercent}
          accent="violet"
        />
        <MetricCard
          label="Primary disk"
          value={
            primaryDisk
              ? formatBytes(primaryDisk.totalBytes - primaryDisk.availableBytes)
              : "Unavailable"
          }
          detail={
            primaryDisk
              ? `${formatBytes(primaryDisk.availableBytes)} free on ${primaryDisk.mountPoint}`
              : "No disk data returned"
          }
          icon={HardDrive}
          percent={primaryDisk ? diskPercent : undefined}
          accent="amber"
        />
        <MetricCard
          label="Network I/O"
          value={`${formatBytes(networkRate)}/s`}
          detail={`${formatBytes(system.network.receivedBytesPerSecond)}/s down · ${formatBytes(system.network.transmittedBytesPerSecond)}/s up`}
          icon={Network}
          accent="green"
        />
      </section>

      <div className="overview-grid">
        <section className="panel panel-wide">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Resource leaders</span>
              <h2>Active processes</h2>
            </div>
            <Link to="/processes" className="text-link">
              Inspect all {processes.total}
            </Link>
          </div>
          <div className="process-leaders">
            {processes.topCpu.slice(0, 6).map((process) => (
              <div className="leader-row" key={`${process.key.pid}:${process.key.startTime}`}>
                <div className="process-avatar">{process.name.slice(0, 2).toUpperCase()}</div>
                <div className="leader-name">
                  <strong>{process.name}</strong>
                  <span>
                    PID {process.key.pid}
                    {process.listeningPortCount ? ` · ${process.listeningPortCount} listening` : ""}
                  </span>
                </div>
                <div className="leader-stat">
                  <strong>{formatPercent(process.cpuPercent)}</strong>
                  <span>{formatBytes(process.memoryBytes)}</span>
                </div>
              </div>
            ))}
            {processes.topCpu.length === 0 && (
              <div className="empty-state">No accessible processes were returned.</div>
            )}
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Localhost surface</span>
              <h2>Listening endpoints</h2>
            </div>
            <Radio size={18} />
          </div>
          <div className="endpoint-summary">
            <strong>{ports.totalListening}</strong>
            <span>TCP/UDP listeners</span>
          </div>
          <div className="endpoint-list">
            {ports.endpoints.slice(0, 7).map((endpoint) => (
              <div
                className="endpoint-row"
                key={`${endpoint.protocol}:${endpoint.localAddress}:${endpoint.localPort}:${endpoint.owningProcessKey?.pid ?? 0}`}
              >
                <code>{endpoint.localPort}</code>
                <span>{endpoint.owningProcessName ?? "Owner unavailable"}</span>
                <small>{endpoint.bindingScope === "loopback" ? "Loopback" : "LAN candidate"}</small>
              </div>
            ))}
            {ports.endpoints.length === 0 && (
              <div className="empty-state compact">No listening endpoints detected.</div>
            )}
          </div>
          <Link to="/processes" className="button button-secondary button-full">
            Open port ownership
          </Link>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Evidence quality</span>
              <h2>Collector status</h2>
            </div>
            <ShieldCheck size={18} />
          </div>
          <div className="availability-list">
            <div>
              <span>Processes & metrics</span>
              <AvailabilityBadge state="available" />
            </div>
            <div>
              <span>
                Battery
                {system.battery.percentage !== null ? ` · ${system.battery.percentage}%` : ""}
              </span>
              <AvailabilityBadge state={system.battery.availability.state} />
            </div>
            <div>
              <span>GPU telemetry</span>
              <AvailabilityBadge state={system.gpu.state} />
            </div>
          </div>
          {(collectorIssues.length > 0 || system.issues.length > 0) && (
            <div className="issue-list">
              {[...collectorIssues, ...system.issues].slice(0, 4).map((issue) => (
                <div key={`${issue.code}:${issue.message}`}>
                  <Activity size={14} />
                  <span>
                    <strong>{issue.code}</strong>
                    {issue.message}
                  </span>
                </div>
              ))}
            </div>
          )}
          <Link to="/permissions" className="text-link panel-link">
            View evidence and permissions
          </Link>
        </section>
      </div>
    </div>
  );
}
