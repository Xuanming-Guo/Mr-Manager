import * as Dialog from "@radix-ui/react-dialog";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  Boxes,
  Database,
  ExternalLink,
  Network,
  Play,
  RefreshCw,
  RotateCw,
  ScrollText,
  ShieldCheck,
  Square,
} from "lucide-react";
import { useMemo, useState } from "react";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { AvailabilityBadge } from "../components/AvailabilityBadge";
import { dockerApi } from "../lib/docker-ipc";
import { formatTimestamp } from "../lib/format";
import { topologyApi } from "../lib/topology-ipc";
import type {
  ComposeDoctorIssue,
  ComposeProject,
  ComposeService,
  DockerAvailability,
  DockerContainer,
  DockerContainerActionKind,
  DockerDiagnostic,
  DockerPortMapping,
} from "../types/docker";

interface PendingAction {
  container: DockerContainer;
  action: DockerContainerActionKind;
}

export function DockerPage() {
  const client = useQueryClient();
  const [selectedContainerId, setSelectedContainerId] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [confirmation, setConfirmation] = useState("");

  const inventory = useQuery({
    queryKey: ["dockerInventory"],
    queryFn: dockerApi.inventory,
    refetchInterval: 5_000,
    placeholderData: (previous) => previous,
  });
  const compose = useQuery({
    queryKey: ["composeProjects"],
    queryFn: dockerApi.composeProjects,
    refetchInterval: 8_000,
    placeholderData: (previous) => previous,
  });

  const containers = inventory.data?.containers ?? [];
  const selectedContainer =
    containers.find((container) => container.id === selectedContainerId) ?? containers[0] ?? null;

  const logs = useQuery({
    queryKey: ["dockerContainerLogs", selectedContainer?.id],
    queryFn: () => dockerApi.logs(selectedContainer?.id ?? "", 160),
    enabled: selectedContainer !== null,
    refetchInterval: selectedContainer?.state === "running" ? 2_500 : false,
  });

  const action = useMutation({
    mutationFn: dockerApi.action,
    onSuccess: (result) => {
      setSelectedContainerId(result.container.id);
      setPendingAction(null);
      setConfirmation("");
      void client.invalidateQueries({ queryKey: ["dockerInventory"] });
      void client.invalidateQueries({ queryKey: ["dockerContainerLogs"] });
      void client.invalidateQueries({ queryKey: ["composeProjects"] });
    },
  });

  const preview = useMutation({
    mutationFn: topologyApi.openPreview,
  });

  const actionError =
    inventory.error ?? compose.error ?? logs.error ?? action.error ?? preview.error;

  if (inventory.isLoading) return <LoadingState label="Inspecting Docker state..." />;
  if (inventory.error || !inventory.data)
    return <ErrorState error={inventory.error} retry={() => void inventory.refetch()} />;

  const { status, networks, volumes } = inventory.data;

  return (
    <div className="page-stack page-fill">
      <header className="page-header">
        <div>
          <span className="eyebrow">Docker / Compose Doctor</span>
          <h1>Docker command center</h1>
        </div>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => {
            void inventory.refetch();
            void compose.refetch();
            void logs.refetch();
          }}
          disabled={inventory.isFetching || compose.isFetching}
        >
          <RefreshCw
            size={15}
            className={inventory.isFetching || compose.isFetching ? "spin" : ""}
          />
          Refresh Docker
        </button>
      </header>

      {actionError && <ErrorState error={actionError} />}

      <section className="docker-status-grid" aria-label="Docker engine status">
        <StatusTile
          label="Engine"
          value={availabilityLabel(status.availability)}
          state={availabilityToBadge(status.availability)}
          detail={status.daemonReachable ? "Daemon reachable" : "No daemon connection"}
        />
        <StatusTile
          label="CLI"
          value={status.cliDetected ? "Detected" : "Missing"}
          state={status.cliDetected ? "available" : "unavailable"}
          detail={status.clientVersion ? `Client ${status.clientVersion}` : "Version unavailable"}
        />
        <StatusTile
          label="Context"
          value={status.context ?? "Unavailable"}
          state={status.context ? "available" : "unavailable"}
          detail={status.serverVersion ? `Server ${status.serverVersion}` : "No server version"}
        />
        <StatusTile
          label="Docker Desktop"
          value={desktopStateLabel(status.dockerDesktopProcess)}
          state={status.dockerDesktopProcess === "running" ? "available" : "unavailable"}
          detail={`Updated ${formatTimestamp(status.collectedAtMs)}`}
        />
      </section>

      {status.diagnostics.length > 0 && <DiagnosticList diagnostics={status.diagnostics} />}

      <div className="docker-layout">
        <section className="panel panel-wide">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Runtime inventory</span>
              <h2>Containers</h2>
            </div>
            <Boxes size={18} />
          </div>

          {status.availability !== "running" ? (
            <div className="docker-unavailable">
              <ShieldCheck size={22} />
              <strong>{availabilityLabel(status.availability)}</strong>
              <span>
                Mr Manager will not fabricate Docker objects. Start Docker Desktop or fix access,
                then refresh.
              </span>
            </div>
          ) : containers.length === 0 ? (
            <div className="empty-inline">Docker is running, but no containers were returned.</div>
          ) : (
            <div className="docker-container-table">
              <div className="docker-container-row docker-container-header">
                <span>Container</span>
                <span>State</span>
                <span>Ports</span>
                <span>Project / Compose</span>
                <span>Actions</span>
              </div>
              {containers.map((container) => (
                <button
                  type="button"
                  key={container.id}
                  className={`docker-container-row ${
                    selectedContainer?.id === container.id ? "docker-container-row-selected" : ""
                  }`}
                  onClick={() => setSelectedContainerId(container.id)}
                >
                  <span className="docker-container-name">
                    <strong>{container.name}</strong>
                    <small>{container.image}</small>
                    <code>{container.shortId}</code>
                  </span>
                  <span>
                    <StatePill state={container.state} health={container.health} />
                    <small>{container.status}</small>
                  </span>
                  <span className="docker-port-stack">
                    {container.ports.length
                      ? container.ports
                          .slice(0, 3)
                          .map((port) => <code key={portLabel(port)}>{portLabel(port)}</code>)
                      : "No published ports"}
                  </span>
                  <span>
                    <strong>
                      {container.associatedProject?.projectName ??
                        container.composeProject ??
                        "Unassociated"}
                    </strong>
                    <small>{container.composeService ?? "No Compose service label"}</small>
                  </span>
                  <span className="docker-action-cell">
                    <button
                      type="button"
                      className="button button-secondary compact"
                      onClick={(event) => {
                        event.stopPropagation();
                        setPendingAction({ container, action: "start" });
                      }}
                      disabled={container.state === "running"}
                    >
                      <Play size={13} />
                      Start
                    </button>
                    <button
                      type="button"
                      className="button button-secondary compact"
                      onClick={(event) => {
                        event.stopPropagation();
                        setPendingAction({ container, action: "stop" });
                      }}
                      disabled={container.state !== "running"}
                    >
                      <Square size={13} />
                      Stop
                    </button>
                    <button
                      type="button"
                      className="button button-secondary compact"
                      onClick={(event) => {
                        event.stopPropagation();
                        setPendingAction({ container, action: "restart" });
                      }}
                      disabled={container.state !== "running"}
                    >
                      <RotateCw size={13} />
                      Restart
                    </button>
                  </span>
                </button>
              ))}
            </div>
          )}
        </section>

        <aside className="docker-side-panel">
          <section className="panel">
            <div className="panel-heading">
              <div>
                <span className="eyebrow">Bounded logs</span>
                <h2>{selectedContainer?.name ?? "No container"}</h2>
              </div>
              <ScrollText size={18} />
            </div>
            {selectedContainer ? (
              <>
                <div className="docker-detail-list">
                  <div>
                    <span>CPU</span>
                    <strong>{selectedContainer.resourceUsage?.cpuPercent ?? "Unavailable"}</strong>
                  </div>
                  <div>
                    <span>Memory</span>
                    <strong>{selectedContainer.resourceUsage?.memoryUsage ?? "Unavailable"}</strong>
                  </div>
                  <div>
                    <span>Networks</span>
                    <strong>{selectedContainer.networks.join(", ") || "None"}</strong>
                  </div>
                  <div>
                    <span>Mounts</span>
                    <strong>{selectedContainer.mounts.length}</strong>
                  </div>
                </div>
                <div className="docker-preview-actions">
                  {webUrls(selectedContainer).map((url) => (
                    <button
                      key={url}
                      type="button"
                      className="button button-secondary compact"
                      onClick={() => preview.mutate(url)}
                    >
                      <ExternalLink size={12} />
                      Preview {url}
                    </button>
                  ))}
                </div>
                <div className="log-pane docker-log-pane" aria-busy={logs.isFetching}>
                  {logs.data?.length ? (
                    logs.data.map((entry) => (
                      <div key={entry.sequence} className="log-line log-system">
                        <span>{entry.timestamp ? compactTimestamp(entry.timestamp) : "log"}</span>
                        <code>{entry.line}</code>
                      </div>
                    ))
                  ) : (
                    <div className="empty-inline">
                      {logs.isFetching ? "Loading logs..." : "No log lines returned."}
                    </div>
                  )}
                </div>
              </>
            ) : (
              <div className="empty-inline">Select a container to inspect logs and mappings.</div>
            )}
          </section>

          <section className="panel">
            <div className="panel-heading">
              <div>
                <span className="eyebrow">Docker objects</span>
                <h2>Networks & volumes</h2>
              </div>
              <Network size={18} />
            </div>
            <div className="docker-object-summary">
              <div>
                <strong>{networks.length}</strong>
                <span>networks</span>
              </div>
              <div>
                <strong>{volumes.length}</strong>
                <span>volumes</span>
              </div>
            </div>
            <div className="docker-chip-list">
              {[...networks.map((item) => item.name), ...volumes.map((item) => item.name)]
                .slice(0, 14)
                .map((name) => (
                  <span key={name}>{name}</span>
                ))}
              {networks.length + volumes.length === 0 && <small>No objects returned.</small>}
            </div>
          </section>
        </aside>
      </div>

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Registered projects only</span>
            <h2>Compose visualizer and doctor</h2>
          </div>
          <Database size={18} />
        </div>

        {compose.isLoading ? (
          <LoadingState label="Parsing registered Compose files..." />
        ) : compose.error ? (
          <ErrorState error={compose.error} retry={() => void compose.refetch()} />
        ) : (compose.data?.length ?? 0) === 0 ? (
          <div className="empty-inline">
            No Compose files are registered yet. Add or rescan a project that contains compose.yaml,
            compose.yml, or docker-compose.yml.
          </div>
        ) : (
          <div className="compose-grid">
            {(compose.data ?? []).map((project) => (
              <ComposeCard key={project.id} project={project} />
            ))}
          </div>
        )}
      </section>

      <Dialog.Root
        open={pendingAction !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingAction(null);
            setConfirmation("");
          }
        }}
      >
        <Dialog.Portal>
          <Dialog.Overlay className="dialog-overlay" />
          <Dialog.Content className="confirm-dialog" aria-describedby={undefined}>
            <div className="confirm-icon">
              <AlertTriangle size={20} />
            </div>
            <Dialog.Title>Confirm Docker {pendingAction?.action}</Dialog.Title>
            <p>
              Mr Manager will run a structured Docker CLI action against{" "}
              <strong>{pendingAction?.container.name}</strong>. It will not delete images, volumes,
              networks, or project files.
            </p>
            {pendingAction && (
              <>
                <div className="confirmation-phrase">
                  Type{" "}
                  <code>{confirmationPhrase(pendingAction.container, pendingAction.action)}</code>
                </div>
                <input
                  className="confirmation-input"
                  value={confirmation}
                  onChange={(event) => setConfirmation(event.target.value)}
                  placeholder={confirmationPhrase(pendingAction.container, pendingAction.action)}
                />
              </>
            )}
            <div className="confirm-actions">
              <Dialog.Close className="button button-secondary">Cancel</Dialog.Close>
              <button
                type="button"
                className="button button-danger"
                disabled={
                  !pendingAction ||
                  confirmation !==
                    confirmationPhrase(pendingAction.container, pendingAction.action) ||
                  action.isPending
                }
                onClick={() =>
                  pendingAction &&
                  action.mutate({
                    containerId: pendingAction.container.id,
                    action: pendingAction.action,
                    confirmation,
                  })
                }
              >
                Confirm {pendingAction?.action}
              </button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  );
}

function StatusTile({
  label,
  value,
  detail,
  state,
}: {
  label: string;
  value: string;
  detail: string;
  state: "available" | "unavailable" | "unsupported" | "permissionDenied" | "error";
}) {
  return (
    <div className="docker-status-tile">
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
      <AvailabilityBadge state={state} />
    </div>
  );
}

function DiagnosticList({ diagnostics }: { diagnostics: DockerDiagnostic[] }) {
  return (
    <div className="topology-issues">
      {diagnostics.map((diagnostic) => (
        <div key={`${diagnostic.code}:${diagnostic.message}`}>
          <AlertTriangle size={15} />
          <span>
            <strong>{diagnostic.code}</strong>
            {diagnostic.message}
            {diagnostic.remediation ? ` ${diagnostic.remediation}` : ""}
          </span>
        </div>
      ))}
    </div>
  );
}

function ComposeCard({ project }: { project: ComposeProject }) {
  const counts = useMemo(() => issueCounts(project.doctor), [project.doctor]);

  return (
    <article className="compose-card">
      <div className="compose-card-heading">
        <div>
          <span className="eyebrow">
            {project.source === "dockerComposeConfig"
              ? "Canonical Docker config"
              : "Fallback parser"}
          </span>
          <h3>{project.projectName}</h3>
          <code>{project.composeFile}</code>
        </div>
        <div className="doctor-counts" aria-label="Compose Doctor issue counts">
          <span className="severity-error">{counts.error}</span>
          <span className="severity-warning">{counts.warning}</span>
          <span className="severity-information">{counts.information}</span>
        </div>
      </div>

      <div className="compose-topology">
        {project.services.map((service) => (
          <ServiceNode key={service.name} service={service} />
        ))}
        {project.services.length === 0 && <div className="empty-inline">No services parsed.</div>}
      </div>

      <div className="compose-section-row">
        <span>Networks</span>
        <strong>{project.networks.map((network) => network.name).join(", ") || "default"}</strong>
      </div>
      <div className="compose-section-row">
        <span>Volumes</span>
        <strong>{project.volumes.map((volume) => volume.name).join(", ") || "none"}</strong>
      </div>

      {project.parseDiagnostics.length > 0 && (
        <div className="doctor-list">
          {project.parseDiagnostics.map((diagnostic) => (
            <DoctorIssueRow
              key={`${diagnostic.code}:${diagnostic.message}`}
              issue={{
                code: diagnostic.code,
                severity: diagnostic.severity,
                service: null,
                message: diagnostic.message,
                remediation: diagnostic.remediation,
                evidence: diagnostic.evidence,
              }}
            />
          ))}
        </div>
      )}

      <div className="doctor-list">
        {project.doctor.slice(0, 12).map((issue) => (
          <DoctorIssueRow
            key={`${issue.code}:${issue.service ?? ""}:${issue.message}`}
            issue={issue}
          />
        ))}
        {project.doctor.length === 0 && (
          <div className="empty-inline">Compose Doctor did not find deterministic issues.</div>
        )}
        {project.doctor.length > 12 && (
          <div className="empty-inline compact">+{project.doctor.length - 12} more issues</div>
        )}
      </div>
    </article>
  );
}

function ServiceNode({ service }: { service: ComposeService }) {
  return (
    <div className="compose-service-node">
      <div>
        <strong>{service.name}</strong>
        <span>{service.image ?? service.build ?? "No image/build parsed"}</span>
      </div>
      <small>
        {service.ports.map((port) => port.raw).join(", ") || "No ports"}
        {service.dependsOn.length ? ` · depends on ${service.dependsOn.join(", ")}` : ""}
      </small>
      <em>{service.healthcheckPresent ? "healthcheck" : "no healthcheck"}</em>
    </div>
  );
}

function DoctorIssueRow({ issue }: { issue: ComposeDoctorIssue }) {
  return (
    <div className={`doctor-row doctor-${issue.severity}`}>
      <strong>{issue.code}</strong>
      <span>
        {issue.service ? `${issue.service}: ` : ""}
        {issue.message}
      </span>
      {issue.remediation && <small>{issue.remediation}</small>}
    </div>
  );
}

function StatePill({ state, health }: { state: string; health: string | null }) {
  const running = state === "running";
  return (
    <span className={`run-state ${running ? "run-state-running" : "run-state-exited"}`}>
      {state}
      {health ? ` / ${health}` : ""}
    </span>
  );
}

function availabilityLabel(value: DockerAvailability): string {
  const labels: Record<DockerAvailability, string> = {
    cliMissing: "CLI missing",
    installedStopped: "Installed / stopped",
    starting: "Docker Desktop starting",
    inaccessible: "Inaccessible",
    running: "Running",
    error: "Error",
  };
  return labels[value];
}

function availabilityToBadge(
  value: DockerAvailability,
): "available" | "unavailable" | "unsupported" | "permissionDenied" | "error" {
  if (value === "running") return "available";
  if (value === "starting") return "unavailable";
  if (value === "inaccessible") return "permissionDenied";
  if (value === "error") return "error";
  return "unavailable";
}

function desktopStateLabel(value: string): string {
  if (value === "running") return "Process detected";
  if (value === "notDetected") return "Not detected";
  return "Unknown";
}

function portLabel(port: DockerPortMapping): string {
  const protocol = port.protocol.toUpperCase();
  const host =
    port.hostPort === null
      ? ""
      : `${port.hostIp && port.hostIp !== "0.0.0.0" ? `${port.hostIp}:` : ""}${port.hostPort}->`;
  return `${host}${port.containerPort}/${protocol}`;
}

function confirmationPhrase(container: DockerContainer, action: DockerContainerActionKind): string {
  return `${action} ${container.name || container.shortId}`;
}

function webUrls(container: DockerContainer): string[] {
  const commonWebPorts = new Set([80, 3000, 3001, 4173, 4200, 5000, 5173, 5174, 8000, 8080, 8787]);
  return container.ports
    .filter((port) => port.hostPort !== null)
    .filter(
      (port) => commonWebPorts.has(port.hostPort ?? 0) || commonWebPorts.has(port.containerPort),
    )
    .slice(0, 3)
    .map((port) => `http://localhost:${port.hostPort}`);
}

function compactTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime())
    ? timestamp.slice(0, 12)
    : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function issueCounts(issues: ComposeDoctorIssue[]) {
  return issues.reduce(
    (counts, issue) => {
      counts[issue.severity] += 1;
      return counts;
    },
    { error: 0, warning: 0, information: 0 },
  );
}
