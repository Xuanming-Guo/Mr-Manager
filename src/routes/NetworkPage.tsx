import * as Dialog from "@radix-ui/react-dialog";
import { useMutation, useQuery } from "@tanstack/react-query";
import { AlertTriangle, ExternalLink, Gauge, Network, QrCode, RefreshCw, Wifi } from "lucide-react";
import QRCode from "react-qr-code";
import { useMemo, useState } from "react";
import { refreshInterval, useSettingsQuery } from "../app/queries";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { AvailabilityBadge } from "../components/AvailabilityBadge";
import { TimeSeriesChart } from "../components/TimeSeriesChart";
import { formatBytes, formatTimestamp } from "../lib/format";
import { desktopApi } from "../lib/ipc";
import { networkApi } from "../lib/network-ipc";
import { taskApi } from "../lib/tasks-ipc";
import type {
  AdapterThroughput,
  NetworkAdapterSnapshot,
  NetworkDiagnosticReport,
  NetworkDiagnosticState,
} from "../types/network";

export function NetworkPage() {
  const settings = useSettingsQuery();
  const externalEnabled = settings.data?.externalNetworkChecks ?? false;
  const snapshot = useQuery({
    queryKey: ["networkSnapshot"],
    queryFn: networkApi.snapshot,
    refetchInterval: refreshInterval(settings.data?.refreshMode),
  });
  const ports = useQuery({
    queryKey: ["ports", "networkDashboard"],
    queryFn: desktopApi.listPorts,
    refetchInterval: 5_000,
  });
  const [internetConfirmOpen, setInternetConfirmOpen] = useState(false);
  const tasks = useQuery({
    queryKey: ["backgroundTasks"],
    queryFn: taskApi.list,
    refetchInterval: 750,
  });
  const latestInternetTask = tasks.data?.find((task) => task.kind === "internetDiagnostics");
  const internetDetail = useQuery({
    queryKey: ["backgroundTask", latestInternetTask?.id],
    queryFn: () => taskApi.get(latestInternetTask?.id ?? ""),
    enabled: latestInternetTask !== undefined,
    refetchInterval:
      latestInternetTask?.state === "running" || latestInternetTask?.state === "cancelling"
        ? 750
        : false,
  });
  const reports =
    internetDetail.data?.output?.kind === "internetDiagnostics"
      ? internetDetail.data.output.value
      : [];

  const runInternetTest = useMutation({
    mutationFn: taskApi.startInternetTest,
    onSuccess: () => {
      setInternetConfirmOpen(false);
      void tasks.refetch();
    },
  });
  const internetRunning = runInternetTest.isPending || latestInternetTask?.state === "running";

  const shareLinks = useMemo(() => {
    return (ports.data ?? [])
      .flatMap((endpoint) =>
        endpoint.lanUrls.map((url) => ({
          url,
          port: endpoint.localPort,
          processName: endpoint.owningProcessName,
        })),
      )
      .slice(0, 8);
  }, [ports.data]);

  const error =
    snapshot.error ?? ports.error ?? tasks.error ?? internetDetail.error ?? runInternetTest.error;

  if (snapshot.isLoading) return <LoadingState label="Reading local adapter counters..." />;
  if (snapshot.error || !snapshot.data) {
    return <ErrorState error={snapshot.error} retry={() => void snapshot.refetch()} />;
  }

  const data = snapshot.data;

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Network Throughput / Internet Diagnostics</span>
          <h1>Network dashboard</h1>
        </div>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => {
            void snapshot.refetch();
            void ports.refetch();
          }}
          disabled={snapshot.isFetching || ports.isFetching}
        >
          <RefreshCw size={15} className={snapshot.isFetching ? "spin" : ""} />
          Refresh local network
        </button>
      </header>

      {error && <ErrorState error={error} />}

      <section className="network-summary-grid">
        <ThroughputTile
          label="Current download"
          value={formatRate(data.combined.receivedBytesPerSecond)}
          detail={`${formatBytes(data.combined.sessionReceivedBytes)} received this session`}
        />
        <ThroughputTile
          label="Current upload"
          value={formatRate(data.combined.transmittedBytesPerSecond)}
          detail={`${formatBytes(data.combined.sessionTransmittedBytes)} sent this session`}
        />
        <ThroughputTile
          label="Peak download"
          value={formatRate(data.combined.peakReceivedBytesPerSecond)}
          detail={`${formatBytes(data.combined.totalReceivedBytes)} total received`}
        />
        <ThroughputTile
          label="Peak upload"
          value={formatRate(data.combined.peakTransmittedBytesPerSecond)}
          detail={`${formatBytes(data.combined.totalTransmittedBytes)} total sent`}
        />
      </section>

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Combined timeline</span>
            <h2>Local adapter throughput</h2>
          </div>
          <Gauge size={18} />
        </div>
        <Timeline throughput={data.combined} />
        <p className="panel-note">
          Last local sample {formatTimestamp(data.collectedAtMs)}. {data.privacyNote}
        </p>
      </section>

      <section className="network-two-column">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Adapters / routes</span>
              <h2>Evidence summary</h2>
            </div>
            <Network size={18} />
          </div>
          <StatusLine
            label="Gateway reachability"
            state={data.gatewayReachability.state}
            value={
              data.gatewayReachability.gateway
                ? `${data.gatewayReachability.gateway}${
                    data.gatewayReachability.latencyMs
                      ? ` · ${data.gatewayReachability.latencyMs} ms`
                      : ""
                  }`
                : "No gateway reported"
            }
          />
          <StatusLine
            label="DNS status"
            state={data.dnsStatus.state}
            value={`${data.dnsStatus.configuredServerCount} configured DNS server(s), addresses redacted`}
          />
          <StatusLine
            label="Probable VPN state"
            state={data.vpnState.likelyActive ? "warn" : "pass"}
            value={data.vpnState.label}
          />
          <div className="mini-list">
            {(data.vpnState.evidence.length ? data.vpnState.evidence : data.dnsStatus.evidence)
              .slice(0, 4)
              .map((item) => (
                <span key={`${item.source}-${item.detail}`}>
                  {item.source}: {item.detail}
                </span>
              ))}
          </div>
          <div className="lan-candidates">
            <strong>LAN IP candidates</strong>
            {data.lanIpCandidates.length > 0 ? (
              data.lanIpCandidates.map((ip) => <span key={ip}>{ip}</span>)
            ) : (
              <span>No private LAN IP found.</span>
            )}
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">External opt-in</span>
              <h2>Internet diagnostics</h2>
            </div>
            <ExternalLink size={18} />
          </div>
          <div className="internet-test-callout">
            <AlertTriangle size={18} />
            <div>
              <strong>Run Internet Test contacts external servers and may consume data.</strong>
              <span>
                Clean launch makes zero external requests. Enable external checks in Settings, then
                use this explicit action.
              </span>
            </div>
          </div>
          <button
            className="button button-primary"
            type="button"
            onClick={() => setInternetConfirmOpen(true)}
            disabled={!externalEnabled || internetRunning}
          >
            {internetRunning ? "Running Internet Test..." : "Run Internet Test"}
          </button>
          {!externalEnabled && (
            <p className="panel-note">
              External network checks are disabled. Toggle them in Settings before running this
              test.
            </p>
          )}
          <DiagnosticReports reports={reports} />
        </section>
      </section>

      <Dialog.Root open={internetConfirmOpen} onOpenChange={setInternetConfirmOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="dialog-overlay" />
          <Dialog.Content
            className="confirm-dialog internet-test-dialog"
            aria-describedby={undefined}
          >
            <div className="confirm-icon">
              <ExternalLink size={20} />
            </div>
            <Dialog.Title>Run external Internet Test?</Dialog.Title>
            <p>
              This is a bounded connectivity diagnostic, not a full multi-server speed benchmark. It
              contacts <code>example.com</code> only after you confirm.
            </p>
            <ul className="internet-test-list">
              <li>External DNS lookup and TCP connection latency to example.com.</li>
              <li>Four ICMP ping attempts; firewalls or VPNs may block them.</li>
              <li>A small HTTP download probe. It may consume a small amount of data.</li>
              <li>
                Upload speed is reported unavailable because no trusted upload endpoint is
                configured.
              </li>
              <li>Route and probable VPN evidence remains local-only.</li>
            </ul>
            <div className="confirm-actions">
              <Dialog.Close className="button button-secondary">Cancel</Dialog.Close>
              <button
                className="button button-primary"
                type="button"
                disabled={runInternetTest.isPending}
                onClick={() => runInternetTest.mutate()}
              >
                Contact external servers
              </button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Per-adapter views</span>
            <h2>Adapters</h2>
          </div>
          <Wifi size={18} />
        </div>
        <div className="adapter-grid">
          {data.adapters.map((adapter) => (
            <AdapterCard key={adapter.id} adapter={adapter} />
          ))}
        </div>
      </section>

      <section className="network-two-column">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Localhost sharing</span>
              <h2>LAN links and QR codes</h2>
            </div>
            <QrCode size={18} />
          </div>
          {shareLinks.length === 0 ? (
            <div className="empty-inline">
              No LAN URL candidates are currently available. Apps bound only to localhost need a
              0.0.0.0 or LAN-interface bind before another device can open them.
            </div>
          ) : (
            <div className="share-link-grid">
              {shareLinks.map((link) => (
                <div key={link.url} className="share-link-card">
                  <QRCode value={link.url} size={96} />
                  <div>
                    <strong>{link.url}</strong>
                    <span>
                      Port {link.port}
                      {link.processName ? ` · ${link.processName}` : ""}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Binding warnings</span>
              <h2>Loopback-only dev servers</h2>
            </div>
            <AlertTriangle size={18} />
          </div>
          {data.localDevServerWarnings.length === 0 ? (
            <div className="empty-inline">No loopback-only local app warning is active.</div>
          ) : (
            <div className="warning-list">
              {data.localDevServerWarnings.map((warning) => (
                <article key={`${warning.address}-${warning.port}`} className="doctor-warning">
                  <strong>
                    Port {warning.port}
                    {warning.processName ? ` · ${warning.processName}` : ""}
                  </strong>
                  <span>{warning.message}</span>
                  <small>{warning.remediation}</small>
                </article>
              ))}
            </div>
          )}
          <div className="per-process-note">
            <AvailabilityBadge state={data.perProcessUsage.availability.state} />
            <span>{data.perProcessUsage.availability.reason}</span>
          </div>
        </section>
      </section>
    </div>
  );
}

function ThroughputTile({
  label,
  value,
  detail,
}: {
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <article className="docker-status-tile">
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </article>
  );
}

function AdapterCard({ adapter }: { adapter: NetworkAdapterSnapshot }) {
  return (
    <article className="adapter-card">
      <div className="adapter-card-header">
        <div>
          <h3>{adapter.displayName}</h3>
          <span>
            {adapter.adapterType} · {adapter.operationalStatus}
          </span>
        </div>
        <span
          className={`state-pill state-${adapter.operationalStatus === "up" ? "running" : "unknown"}`}
        >
          {adapter.operationalStatus}
        </span>
      </div>
      <dl className="compact-facts">
        <div>
          <dt>Down</dt>
          <dd>{formatRate(adapter.throughput.receivedBytesPerSecond)}</dd>
        </div>
        <div>
          <dt>Up</dt>
          <dd>{formatRate(adapter.throughput.transmittedBytesPerSecond)}</dd>
        </div>
        <div>
          <dt>Link</dt>
          <dd>
            {adapter.linkSpeedBitsPerSecond ? formatBits(adapter.linkSpeedBitsPerSecond) : "N/A"}
          </dd>
        </div>
        <div>
          <dt>Wi-Fi signal</dt>
          <dd>
            {adapter.wifiSignalQualityPercent !== null
              ? `${adapter.wifiSignalQualityPercent}%`
              : "not available"}
          </dd>
        </div>
      </dl>
      <Timeline throughput={adapter.throughput} compact />
      <div className="mini-list">
        <span>IPv4: {adapter.ipv4Addresses.join(", ") || "none"}</span>
        <span>Gateway: {adapter.gatewayAddresses.join(", ") || "none"}</span>
        <span>DNS: {adapter.dnsServerCount} configured, addresses redacted</span>
      </div>
    </article>
  );
}

function Timeline({
  throughput,
  compact = false,
}: {
  throughput: AdapterThroughput;
  compact?: boolean;
}) {
  const samples = throughput.timeline.slice(-48);
  if (samples.length === 0) {
    return <div className="empty-inline">Waiting for throughput samples...</div>;
  }
  const latest = samples.at(-1);
  return (
    <TimeSeriesChart
      compact={compact}
      timestampsMs={samples.map((sample) => sample.collectedAtMs)}
      series={[
        {
          label: "Download",
          values: samples.map((sample) => sample.receivedBytesPerSecond * 8),
          stroke: "#4bd6de",
          fill: "rgba(75, 214, 222, 0.12)",
          formatValue: formatBits,
        },
        {
          label: "Upload",
          values: samples.map((sample) => sample.transmittedBytesPerSecond * 8),
          stroke: "#a855f7",
          fill: "rgba(168, 85, 247, 0.08)",
          formatValue: formatBits,
        },
      ]}
      formatAxisValue={formatBits}
      minimumDynamicMaximum={1_000}
      ariaLabel={`Network throughput chart. Latest download ${formatBits((latest?.receivedBytesPerSecond ?? 0) * 8)}, latest upload ${formatBits((latest?.transmittedBytesPerSecond ?? 0) * 8)}.`}
    />
  );
}

function StatusLine({
  label,
  state,
  value,
}: {
  label: string;
  state: NetworkDiagnosticState;
  value: string;
}) {
  return (
    <div className="status-line">
      <span>{label}</span>
      <strong>{value}</strong>
      <em className={`diagnostic-${state}`}>{state}</em>
    </div>
  );
}

function DiagnosticReports({ reports }: { reports: NetworkDiagnosticReport[] }) {
  if (reports.length === 0) return null;
  return (
    <div className="diagnostic-reports">
      {reports.map((report) => (
        <article key={`${report.kind}-${report.completedAtMs}`} className="diagnostic-report">
          <div>
            <strong>{report.kind}</strong>
            <span>
              {report.contactedInternet ? "contacted internet" : "local-only"} · endpoints:{" "}
              {report.endpointsContacted.join(", ") || "none"}
            </span>
          </div>
          {report.results.map((result) => (
            <StatusLine
              key={`${report.kind}-${result.label}`}
              label={result.label}
              state={result.state}
              value={result.value ?? (result.contactedInternet ? "external test" : "local-only")}
            />
          ))}
          {report.warnings.map((warning) => (
            <small key={warning} className="inline-warning">
              {warning}
            </small>
          ))}
        </article>
      ))}
    </div>
  );
}

function formatRate(bytesPerSecond: number) {
  return `${formatBytes(bytesPerSecond)}/s`;
}

function formatBits(bitsPerSecond: number) {
  if (bitsPerSecond >= 1_000_000_000) return `${(bitsPerSecond / 1_000_000_000).toFixed(1)} Gbps`;
  if (bitsPerSecond >= 1_000_000) return `${(bitsPerSecond / 1_000_000).toFixed(1)} Mbps`;
  if (bitsPerSecond >= 1_000) return `${(bitsPerSecond / 1_000).toFixed(1)} Kbps`;
  return `${bitsPerSecond} bps`;
}
