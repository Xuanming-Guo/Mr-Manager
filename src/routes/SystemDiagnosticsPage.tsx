import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  Activity,
  BatteryCharging,
  Database,
  Download,
  Gauge,
  HardDrive,
  MemoryStick,
  PauseCircle,
  PlayCircle,
  RefreshCw,
  Trash2,
  Upload,
} from "lucide-react";
import { useRef, useState } from "react";
import { refreshInterval, useSettingsQuery } from "../app/queries";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { AvailabilityBadge } from "../components/AvailabilityBadge";
import { TimeSeriesChart } from "../components/TimeSeriesChart";
import { buildRecordingTimelineData } from "../lib/diagnostics-chart";
import { formatBytes, formatPercent, formatTimestamp } from "../lib/format";
import { systemDiagnosticsApi } from "../lib/system-diagnostics-ipc";
import type {
  CorrelationFinding,
  MetricRecordingDetail,
  RankedProcess,
  RecordingSessionSummary,
} from "../types/system-diagnostics";

type RankKind = "cpu" | "memory" | "disk";

export function SystemDiagnosticsPage() {
  const queryClient = useQueryClient();
  const settings = useSettingsQuery();
  const [recordingName, setRecordingName] = useState("Fixture build");
  const [annotation, setAnnotation] = useState("");
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [exportText, setExportText] = useState("");
  const [rankKind, setRankKind] = useState<RankKind>("cpu");

  const snapshot = useQuery({
    queryKey: ["systemDiagnosticsSnapshot"],
    queryFn: systemDiagnosticsApi.snapshot,
    refetchInterval: refreshInterval(settings.data?.refreshMode),
  });
  const sessions = useQuery({
    queryKey: ["metricSessions"],
    queryFn: systemDiagnosticsApi.sessions,
    refetchInterval: 5_000,
  });
  const activeSessionId = snapshot.data?.activeRecording?.id ?? null;
  const firstSessionId = sessions.data?.[0]?.id ?? null;
  const effectiveSelectedSessionId = selectedSessionId ?? activeSessionId ?? firstSessionId;
  const detail = useQuery({
    queryKey: ["metricRecording", effectiveSelectedSessionId],
    queryFn: () => systemDiagnosticsApi.detail(effectiveSelectedSessionId ?? ""),
    enabled: effectiveSelectedSessionId !== null,
    refetchInterval: activeSessionId === effectiveSelectedSessionId ? 1_000 : false,
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["systemDiagnosticsSnapshot"] });
    void queryClient.invalidateQueries({ queryKey: ["metricSessions"] });
    void queryClient.invalidateQueries({ queryKey: ["metricRecording"] });
  };

  const start = useMutation({
    mutationFn: () => systemDiagnosticsApi.startRecording({ name: recordingName }),
    onSuccess: (summary) => {
      setSelectedSessionId(summary.id);
      setExportText("");
      invalidate();
    },
  });
  const stop = useMutation({
    mutationFn: systemDiagnosticsApi.stopRecording,
    onSuccess: (summary) => {
      setSelectedSessionId(summary.id);
      invalidate();
    },
  });
  const annotate = useMutation({
    mutationFn: () => systemDiagnosticsApi.addAnnotation({ label: annotation }),
    onSuccess: () => {
      setAnnotation("");
      invalidate();
    },
  });
  const exportRecording = useMutation({
    mutationFn: (sessionId: string) => systemDiagnosticsApi.export(sessionId),
    onSuccess: (payload) => setExportText(JSON.stringify(payload, null, 2)),
  });
  const deleteRecording = useMutation({
    mutationFn: (sessionId: string) => systemDiagnosticsApi.delete(sessionId),
    onSuccess: (_, sessionId) => {
      setSelectedSessionId((current) => (current === sessionId ? null : current));
      setExportText("");
      invalidate();
    },
  });

  const error =
    snapshot.error ??
    sessions.error ??
    detail.error ??
    start.error ??
    stop.error ??
    annotate.error ??
    exportRecording.error ??
    deleteRecording.error;

  if (snapshot.isLoading) return <LoadingState label="Opening System Diagnostics..." />;
  if (snapshot.error || !snapshot.data) {
    return <ErrorState error={snapshot.error} retry={() => void snapshot.refetch()} />;
  }

  const data = snapshot.data;
  const ranked =
    rankKind === "cpu"
      ? data.rankedProcesses.topCpu
      : rankKind === "memory"
        ? data.rankedProcesses.topMemory
        : data.rankedProcesses.topDiskIo;

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Performance / Timeline Recorder</span>
          <h1>System Diagnostics</h1>
        </div>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => {
            void snapshot.refetch();
            void sessions.refetch();
            void detail.refetch();
          }}
          disabled={snapshot.isFetching}
        >
          <RefreshCw size={15} className={snapshot.isFetching ? "spin" : ""} />
          Refresh diagnostics
        </button>
      </header>

      {error && <ErrorState error={error} />}

      <section className="diagnostics-summary-grid">
        <DiagnosticsMetricCard
          icon={Gauge}
          label="CPU"
          value={formatPercent(data.system.cpu.totalUsagePercent)}
          detail={`${data.system.cpu.logicalCoreCount} logical cores · ${data.refreshMode === "fast" ? "Fast" : "Normal"}`}
        />
        <DiagnosticsMetricCard
          icon={MemoryStick}
          label="RAM"
          value={formatPercent(
            memoryPercent(data.system.memory.usedBytes, data.system.memory.totalBytes),
          )}
          detail={`${formatBytes(data.system.memory.usedBytes)} / ${formatBytes(data.system.memory.totalBytes)}`}
        />
        <DiagnosticsMetricCard
          icon={HardDrive}
          label="Disk I/O"
          value={formatBytes(
            data.rankedProcesses.topDiskIo[0]
              ? data.rankedProcesses.topDiskIo[0].diskReadBytes +
                  data.rankedProcesses.topDiskIo[0].diskWriteBytes
              : 0,
          )}
          detail="Top observed process total I/O"
        />
        <DiagnosticsMetricCard
          icon={Download}
          label="Download"
          value={`${formatBytes(data.network.combined.receivedBytesPerSecond)}/s`}
          detail={`${formatBytes(data.network.combined.sessionReceivedBytes)} session`}
        />
        <DiagnosticsMetricCard
          icon={Upload}
          label="Upload"
          value={`${formatBytes(data.network.combined.transmittedBytesPerSecond)}/s`}
          detail={`${formatBytes(data.network.combined.sessionTransmittedBytes)} session`}
        />
        <DiagnosticsMetricCard
          icon={BatteryCharging}
          label="Battery"
          value={
            data.system.battery.percentage !== null ? `${data.system.battery.percentage}%` : "N/A"
          }
          detail={data.system.battery.availability.reason ?? "Battery unavailable"}
        />
      </section>

      <section className="system-diagnostics-layout">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">GPU provider abstraction</span>
              <h2>GPU</h2>
            </div>
            <Activity size={18} />
          </div>
          <AvailabilityBadge state={data.gpu.availability.state} />
          <p className="panel-note">{data.gpu.availability.reason}</p>
          {data.gpu.adapters.length === 0 ? (
            <div className="empty-inline">
              GPU utilization, VRAM, and temperature are not available on this hardware/API.
            </div>
          ) : (
            <div className="mini-table">
              {data.gpu.adapters.map((adapter) => (
                <div key={adapter.name} className="mini-table-row">
                  <strong>{adapter.name}</strong>
                  <span>{adapter.utilizationPercent ?? "?"}%</span>
                  <span>
                    {adapter.vramUsedBytes && adapter.vramTotalBytes
                      ? `${formatBytes(adapter.vramUsedBytes)} / ${formatBytes(adapter.vramTotalBytes)}`
                      : "VRAM N/A"}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Collector diagnostics</span>
              <h2>Collector health</h2>
            </div>
            <Database size={18} />
          </div>
          <dl className="compact-facts">
            <div>
              <dt>Duration</dt>
              <dd>{data.collectorDiagnostics.collectionDurationMs} ms</dd>
            </div>
            <div>
              <dt>Processes</dt>
              <dd>{data.collectorDiagnostics.processCount}</dd>
            </div>
            <div>
              <dt>Ports</dt>
              <dd>{data.collectorDiagnostics.portCount}</dd>
            </div>
            <div>
              <dt>Dropped samples</dt>
              <dd>{data.collectorDiagnostics.droppedRecordingSamples}</dd>
            </div>
          </dl>
          {data.collectorDiagnostics.warnings.length > 0 && (
            <div className="mini-list">
              {data.collectorDiagnostics.warnings.slice(0, 4).map((warning) => (
                <span key={warning}>{warning}</span>
              ))}
            </div>
          )}
        </section>
      </section>

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Virtualized ranked process views</span>
            <h2>Top processes</h2>
          </div>
          <Activity size={18} />
        </div>
        <div className="filter-row">
          {(["cpu", "memory", "disk"] as const).map((kind) => (
            <button
              key={kind}
              type="button"
              className={rankKind === kind ? "chip active" : "chip"}
              onClick={() => setRankKind(kind)}
            >
              {kind === "disk" ? "Disk I/O" : kind.toUpperCase()}
            </button>
          ))}
        </div>
        <VirtualProcessList processes={ranked} kind={rankKind} />
        <p className="panel-note">{data.rankedProcesses.topGpu.reason}</p>
      </section>

      <section className="system-diagnostics-layout">
        <RecordingControls
          active={data.activeRecording}
          recordingName={recordingName}
          setRecordingName={setRecordingName}
          annotation={annotation}
          setAnnotation={setAnnotation}
          start={() => start.mutate()}
          stop={() => stop.mutate()}
          annotate={() => annotate.mutate()}
          busy={start.isPending || stop.isPending || annotate.isPending}
        />
        <RecordingSessions
          sessions={sessions.data ?? []}
          selectedSessionId={effectiveSelectedSessionId}
          onSelect={(id) => {
            setSelectedSessionId(id);
            setExportText("");
          }}
          onExport={(id) => exportRecording.mutate(id)}
          onDelete={(id) => deleteRecording.mutate(id)}
        />
      </section>

      <RecordingDetailPanel
        detail={detail.data}
        loading={detail.isFetching}
        exportText={exportText}
      />

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">What changed?</span>
            <h2>Change analysis</h2>
          </div>
          <Activity size={18} />
        </div>
        <FindingList findings={data.recentFindings} />
      </section>
    </div>
  );
}

function DiagnosticsMetricCard({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof Gauge;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <article className="metric-card metric-cyan">
      <div className="metric-heading">
        <span>{label}</span>
        <Icon size={17} />
      </div>
      <div className="metric-value">{value}</div>
      <div className="metric-detail">{detail}</div>
    </article>
  );
}

function VirtualProcessList({ processes, kind }: { processes: RankedProcess[]; kind: RankKind }) {
  const parentRef = useRef<HTMLDivElement | null>(null);
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: processes.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48,
    overscan: 8,
  });

  return (
    <div className="rank-list" ref={parentRef}>
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => {
          const process = processes[row.index];
          if (!process) return null;
          return (
            <div
              key={`${process.key.pid}-${process.key.startTime}-${kind}`}
              className="rank-row"
              style={{ transform: `translateY(${row.start}px)` }}
            >
              <strong>{process.name}</strong>
              <span>PID {process.key.pid}</span>
              <span>{formatPercent(process.cpuPercent)}</span>
              <span>{formatBytes(process.memoryBytes)}</span>
              <span>{formatBytes(process.diskReadBytes + process.diskWriteBytes)}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function RecordingControls({
  active,
  recordingName,
  setRecordingName,
  annotation,
  setAnnotation,
  start,
  stop,
  annotate,
  busy,
}: {
  active: RecordingSessionSummary | null;
  recordingName: string;
  setRecordingName: (value: string) => void;
  annotation: string;
  setAnnotation: (value: string) => void;
  start: () => void;
  stop: () => void;
  annotate: () => void;
  busy: boolean;
}) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Recording mode</span>
          <h2>{active ? active.name : "Start a recording"}</h2>
        </div>
        {active ? <PauseCircle size={18} /> : <PlayCircle size={18} />}
      </div>
      {active ? (
        <>
          <dl className="compact-facts">
            <div>
              <dt>Samples</dt>
              <dd>{active.sampleCount}</dd>
            </div>
            <div>
              <dt>Annotations</dt>
              <dd>{active.annotationCount}</dd>
            </div>
          </dl>
          <div className="recording-form">
            <input
              value={annotation}
              onChange={(event) => setAnnotation(event.target.value)}
              placeholder="Add annotation, e.g. Started npm build"
            />
            <button
              className="button button-secondary"
              type="button"
              onClick={annotate}
              disabled={busy}
            >
              Annotate
            </button>
          </div>
          <button className="button button-primary" type="button" onClick={stop} disabled={busy}>
            Stop and save recording
          </button>
        </>
      ) : (
        <>
          <div className="recording-form">
            <input
              value={recordingName}
              onChange={(event) => setRecordingName(event.target.value)}
              placeholder="Recording name"
            />
            <button className="button button-primary" type="button" onClick={start} disabled={busy}>
              Start recording
            </button>
          </div>
          <p className="panel-note">
            Recording captures local CPU, RAM, disk I/O, GPU provider state, battery, top processes,
            adapter throughput, VPN evidence, Docker process evidence, and local dev-server ports.
          </p>
        </>
      )}
    </section>
  );
}

function RecordingSessions({
  sessions,
  selectedSessionId,
  onSelect,
  onExport,
  onDelete,
}: {
  sessions: RecordingSessionSummary[];
  selectedSessionId: string | null;
  onSelect: (id: string) => void;
  onExport: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Retention / export / delete</span>
          <h2>Recording sessions</h2>
        </div>
        <Database size={18} />
      </div>
      {sessions.length === 0 ? (
        <div className="empty-inline">No saved recording sessions yet.</div>
      ) : (
        <div className="session-list">
          {sessions.map((session) => (
            <article
              key={session.id}
              className={selectedSessionId === session.id ? "session-card active" : "session-card"}
            >
              <button type="button" onClick={() => onSelect(session.id)}>
                <strong>{session.name}</strong>
                <span>
                  {session.status} · {session.sampleCount} samples ·{" "}
                  {formatTimestamp(session.startedAtMs)}
                </span>
              </button>
              <div>
                <button
                  className="icon-button bordered"
                  type="button"
                  onClick={() => onExport(session.id)}
                >
                  <Download size={15} />
                </button>
                <button
                  className="icon-button danger"
                  type="button"
                  onClick={() => onDelete(session.id)}
                >
                  <Trash2 size={15} />
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function RecordingDetailPanel({
  detail,
  loading,
  exportText,
}: {
  detail: MetricRecordingDetail | undefined;
  loading: boolean;
  exportText: string;
}) {
  if (!detail) return null;
  return (
    <section className="panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Timeline inspector</span>
          <h2>{detail.summary.name}</h2>
        </div>
        <Activity size={18} className={loading ? "spin" : ""} />
      </div>
      <RecordingTimeline detail={detail} />
      {detail.annotations.length > 0 && (
        <div className="annotation-list">
          {detail.annotations.map((item) => (
            <span key={item.id}>
              {formatTimestamp(item.atMs)} · {item.label}
            </span>
          ))}
        </div>
      )}
      <FindingList findings={detail.summary.findings} />
      {exportText && <textarea className="export-preview" readOnly value={exportText} />}
    </section>
  );
}

function RecordingTimeline({ detail }: { detail: MetricRecordingDetail }) {
  const timeline = buildRecordingTimelineData(detail.samples);
  if (timeline.timestampsMs.length === 0) {
    return <div className="empty-inline">Waiting for samples...</div>;
  }
  const latestCpu = timeline.cpuPercent.at(-1) ?? 0;
  const latestRam = timeline.ramPercent.at(-1) ?? 0;
  return (
    <TimeSeriesChart
      className="recording-chart"
      timestampsMs={timeline.timestampsMs}
      series={[
        {
          label: "CPU",
          values: timeline.cpuPercent,
          stroke: "#4bd6de",
          fill: "rgba(75, 214, 222, 0.12)",
          formatValue: formatChartPercent,
        },
        {
          label: "RAM",
          values: timeline.ramPercent,
          stroke: "#a855f7",
          fill: "rgba(168, 85, 247, 0.08)",
          formatValue: formatChartPercent,
        },
      ]}
      formatAxisValue={(value) => `${Math.round(value)}%`}
      fixedMaximum={100}
      ariaLabel={`System diagnostics timeline. Latest CPU ${formatChartPercent(latestCpu)}, latest RAM ${formatChartPercent(latestRam)}.`}
    />
  );
}

function FindingList({ findings }: { findings: CorrelationFinding[] }) {
  if (findings.length === 0) {
    return <div className="empty-inline">No deterministic correlation finding yet.</div>;
  }
  return (
    <div className="finding-list">
      {findings.map((finding) => (
        <article key={`${finding.title}-${finding.detail}`} className="doctor-warning">
          <strong>{finding.title}</strong>
          <span>{finding.detail}</span>
          <small>{finding.nonCausalDisclaimer}</small>
        </article>
      ))}
    </div>
  );
}

function memoryPercent(used: number, total: number) {
  return total > 0 ? (used / total) * 100 : 0;
}

function formatChartPercent(value: number) {
  return `${value.toFixed(1)}%`;
}
