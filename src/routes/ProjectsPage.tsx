import * as Dialog from "@radix-ui/react-dialog";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  Boxes,
  FileKey2,
  FolderGit2,
  FolderPlus,
  GitBranch,
  Play,
  RefreshCw,
  Search,
  Square,
  Terminal,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { formatBytes } from "../lib/format";
import { desktopApi } from "../lib/ipc";
import { projectsApi, selectProjectFolder } from "../lib/projects-ipc";
import { taskApi } from "../lib/tasks-ipc";
import type { PortEndpoint } from "../types/system";
import type {
  ManagedCommand,
  ManagedCommandLogEntry,
  Project,
  ProjectScript,
} from "../types/projects";

export function ProjectsPage() {
  const client = useQueryClient();
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [removal, setRemoval] = useState<Project | null>(null);

  const backgroundTasks = useQuery({
    queryKey: ["backgroundTasks"],
    queryFn: taskApi.list,
    refetchInterval: 750,
  });
  const latestDiscovery = backgroundTasks.data?.find((task) => task.kind === "projectDiscovery");
  const discoveryActive =
    latestDiscovery?.state === "running" || latestDiscovery?.state === "cancelling";
  const projects = useQuery({
    queryKey: ["projects"],
    queryFn: projectsApi.list,
    refetchInterval: discoveryActive ? 1_000 : false,
  });
  const managed = useQuery({
    queryKey: ["managedProcesses"],
    queryFn: projectsApi.listManaged,
    refetchInterval: 1_500,
  });
  const ports = useQuery({
    queryKey: ["projectManagedPorts"],
    queryFn: desktopApi.listPorts,
    refetchInterval: 2_000,
  });
  const runLogs = useQuery({
    queryKey: ["managedProcessLogs", selectedRunId],
    queryFn: () => projectsApi.logs(selectedRunId ?? ""),
    enabled: selectedRunId !== null,
    refetchInterval: 1_000,
  });

  const refreshProjects = () => client.invalidateQueries({ queryKey: ["projects"] });
  const refreshRuns = () => client.invalidateQueries({ queryKey: ["managedProcesses"] });

  const add = useMutation({
    mutationFn: projectsApi.add,
    onSuccess: (project) => {
      setSelectedId(project.id);
      void refreshProjects();
    },
  });
  const discover = useMutation({
    mutationFn: ({ rootPath, operationId }: { rootPath: string; operationId: string }) =>
      taskApi.startProjectDiscovery({ rootPath, operationId }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["backgroundTasks"] });
    },
  });
  const rescan = useMutation({
    mutationFn: projectsApi.rescan,
    onSuccess: (project) => {
      client.setQueryData<Project[]>(
        ["projects"],
        (items) => items?.map((item) => (item.id === project.id ? project : item)) ?? [project],
      );
    },
  });
  const remove = useMutation({
    mutationFn: projectsApi.remove,
    onSuccess: () => {
      setRemoval(null);
      setSelectedId(null);
      void refreshProjects();
    },
  });
  const runCommand = useMutation({
    mutationFn: ({ projectId, scriptId }: { projectId: string; scriptId: string }) =>
      projectsApi.runCommand(projectId, scriptId),
    onSuccess: (run) => {
      setSelectedRunId(run.runId);
      void refreshRuns();
    },
  });
  const stopCommand = useMutation({
    mutationFn: ({ runId, force }: { runId: string; force: boolean }) =>
      projectsApi.stop(runId, force),
    onSuccess: (run) => {
      setSelectedRunId(run.runId);
      void refreshRuns();
      void client.invalidateQueries({ queryKey: ["managedProcessLogs", run.runId] });
    },
  });

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (projects.data ?? []).filter((project) => {
      const haystack = `${project.name} ${project.rootPath} ${project.detectedStacks.join(" ")}`;
      return !needle || haystack.toLowerCase().includes(needle);
    });
  }, [projects.data, query]);
  const selected = filtered.find((project) => project.id === selectedId) ?? filtered[0] ?? null;
  const selectedRuns = (managed.data ?? []).filter((run) => run.projectId === selected?.id);
  const visibleRun =
    selectedRuns.find((run) => run.runId === selectedRunId) ?? selectedRuns[0] ?? null;
  const visibleRunPorts = (ports.data ?? []).filter(
    (port) => port.owningProcessKey?.pid === visibleRun?.pid,
  );

  const chooseProject = async () => {
    const path = await selectProjectFolder("Choose one project folder");
    if (path) add.mutate(path);
  };
  const chooseRoot = async () => {
    const rootPath = await selectProjectFolder("Choose a bounded project-discovery root");
    if (rootPath) discover.mutate({ rootPath, operationId: crypto.randomUUID() });
  };
  const actionError =
    add.error ??
    discover.error ??
    rescan.error ??
    remove.error ??
    runCommand.error ??
    stopCommand.error ??
    managed.error;

  useEffect(() => {
    if (latestDiscovery?.state === "succeeded") {
      void client.invalidateQueries({ queryKey: ["projects"] });
    }
  }, [client, latestDiscovery?.completedAtMs, latestDiscovery?.state]);

  if (projects.isLoading) return <LoadingState label="Loading the local project registry..." />;
  if (projects.error)
    return <ErrorState error={projects.error} retry={() => void projects.refetch()} />;

  return (
    <div className="page-stack page-fill">
      <header className="page-header">
        <div>
          <span className="eyebrow">Explicit roots only</span>
          <h1>Projects</h1>
        </div>
        <div className="header-actions">
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void chooseRoot()}
            disabled={discover.isPending || discoveryActive}
          >
            <Boxes size={15} />
            Discover root
          </button>
          <button
            className="button button-primary"
            type="button"
            onClick={() => void chooseProject()}
            disabled={add.isPending}
          >
            <FolderPlus size={15} />
            Add project
          </button>
        </div>
      </header>

      {actionError && <ErrorState error={actionError} />}
      {(add.isPending || discover.isPending || discoveryActive) && (
        <div className="scan-banner">
          <RefreshCw className="spin" size={15} />
          <span>
            {discover.isPending || discoveryActive
              ? "Scanning the selected root within configured bounds..."
              : "Inspecting the selected project..."}
          </span>
        </div>
      )}

      {(projects.data?.length ?? 0) === 0 ? (
        <section className="project-empty">
          <FolderGit2 size={30} />
          <h2>No registered projects</h2>
          <p>
            Mr Manager has not scanned your computer. Select one project or an intentionally bounded
            root to begin.
          </p>
          <button
            className="button button-primary"
            type="button"
            onClick={() => void chooseProject()}
          >
            <FolderPlus size={15} />
            Choose project folder
          </button>
        </section>
      ) : (
        <div className="projects-layout">
          <section className="project-list-panel">
            <label className="search-field project-search">
              <Search size={15} />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Search registered projects"
              />
            </label>
            <div className="project-list">
              {filtered.map((project) => (
                <button
                  key={project.id}
                  type="button"
                  className={`project-list-item ${
                    selected?.id === project.id ? "project-list-item-selected" : ""
                  }`}
                  onClick={() => setSelectedId(project.id)}
                >
                  <span className="project-icon">
                    <FolderGit2 size={17} />
                  </span>
                  <span className="project-list-copy">
                    <strong>{project.name}</strong>
                    <small>{project.rootPath}</small>
                    <span>
                      {project.detectedStacks.length
                        ? project.detectedStacks.join(" - ")
                        : "Stack not detected"}
                    </span>
                  </span>
                  <span className={`health-dot health-${project.scanHealth.state}`} />
                </button>
              ))}
            </div>
          </section>

          {selected && (
            <ProjectInspector
              project={selected}
              runs={selectedRuns}
              selectedRun={visibleRun}
              runPorts={visibleRunPorts}
              logs={runLogs.data ?? []}
              logsLoading={runLogs.isFetching}
              onSelectRun={setSelectedRunId}
              onRun={(script) => runCommand.mutate({ projectId: selected.id, scriptId: script.id })}
              onStop={(runId, force) => stopCommand.mutate({ runId, force })}
              onRescan={() => rescan.mutate(selected.id)}
              onRemove={() => setRemoval(selected)}
              rescanning={rescan.isPending}
              runningScriptId={
                runCommand.isPending ? (runCommand.variables?.scriptId ?? null) : null
              }
              stoppingRunId={stopCommand.isPending ? (stopCommand.variables?.runId ?? null) : null}
            />
          )}
        </div>
      )}

      <Dialog.Root open={removal !== null} onOpenChange={(open) => !open && setRemoval(null)}>
        <Dialog.Portal>
          <Dialog.Overlay className="dialog-overlay" />
          <Dialog.Content className="confirm-dialog" aria-describedby={undefined}>
            <div className="confirm-icon">
              <Trash2 size={20} />
            </div>
            <Dialog.Title>Remove project from Mr Manager?</Dialog.Title>
            <p>
              This removes <strong>{removal?.name}</strong> from the local registry. It will not
              delete or modify <code>{removal?.rootPath}</code>.
            </p>
            <div className="confirm-actions">
              <Dialog.Close className="button button-secondary">Cancel</Dialog.Close>
              <button
                type="button"
                className="button button-danger"
                onClick={() => removal && remove.mutate(removal.id)}
                disabled={remove.isPending}
              >
                Remove from registry
              </button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  );
}

interface ProjectInspectorProps {
  project: Project;
  runs: ManagedCommand[];
  selectedRun: ManagedCommand | null;
  runPorts: PortEndpoint[];
  logs: ManagedCommandLogEntry[];
  logsLoading: boolean;
  onSelectRun: (runId: string) => void;
  onRun: (script: ProjectScript) => void;
  onStop: (runId: string, force: boolean) => void;
  onRescan: () => void;
  onRemove: () => void;
  rescanning: boolean;
  runningScriptId: string | null;
  stoppingRunId: string | null;
}

function ProjectInspector({
  project,
  runs,
  selectedRun,
  runPorts,
  logs,
  logsLoading,
  onSelectRun,
  onRun,
  onStop,
  onRescan,
  onRemove,
  rescanning,
  runningScriptId,
  stoppingRunId,
}: ProjectInspectorProps) {
  return (
    <section className="project-inspector">
      <div className="project-inspector-header">
        <div>
          <span className="eyebrow">Registered project</span>
          <h2>{project.name}</h2>
          <code>{project.rootPath}</code>
        </div>
        <div className="header-actions">
          <button
            type="button"
            className="icon-button bordered"
            onClick={onRescan}
            title="Rescan project"
          >
            <RefreshCw size={15} className={rescanning ? "spin" : ""} />
          </button>
          <button
            type="button"
            className="icon-button bordered danger"
            onClick={onRemove}
            title="Remove from registry"
          >
            <Trash2 size={15} />
          </button>
        </div>
      </div>

      <div className="project-summary-grid">
        <div>
          <span>Scan health</span>
          <strong className={`health-text health-${project.scanHealth.state}`}>
            {project.scanHealth.state}
          </strong>
        </div>
        <div>
          <span>Package manager</span>
          <strong>{project.packageManager?.name ?? "Not detected"}</strong>
        </div>
        <div>
          <span>Manifests</span>
          <strong>{project.manifests.length}</strong>
        </div>
        <div>
          <span>Runnable scripts</span>
          <strong>{project.scripts.length}</strong>
        </div>
      </div>

      {project.scanHealth.issues.length > 0 && (
        <div className="project-issues">
          {project.scanHealth.issues.map((issue) => (
            <div key={`${issue.code}:${issue.message}`}>
              <AlertTriangle size={14} />
              <span>
                <strong>{issue.code}</strong>
                {issue.message}
                {issue.remediation && <small>{issue.remediation}</small>}
              </span>
            </div>
          ))}
        </div>
      )}

      <div className="project-section">
        <div className="section-title">
          <Play size={15} />
          <h3>Structured commands</h3>
        </div>
        {project.scripts.length ? (
          <div className="script-list">
            {project.scripts.map((script) => (
              <div key={script.id} className="script-row script-row-action">
                <div>
                  <strong>{script.label}</strong>
                  <span>{script.source}</span>
                </div>
                <code>{commandText(script.executable, script.arguments)}</code>
                <small>{script.workingDirectory}</small>
                <button
                  className="button button-secondary compact"
                  type="button"
                  onClick={() => onRun(script)}
                  disabled={runningScriptId === script.id}
                >
                  <Play size={14} />
                  Run
                </button>
              </div>
            ))}
          </div>
        ) : (
          <div className="empty-inline">No supported scripts detected.</div>
        )}
      </div>

      <div className="project-section">
        <div className="section-title">
          <Terminal size={15} />
          <h3>Managed runs</h3>
        </div>
        {runs.length ? (
          <div className="managed-runs">
            <div className="managed-run-list">
              {runs.map((run) => (
                <button
                  type="button"
                  key={run.runId}
                  className={`managed-run-row ${
                    selectedRun?.runId === run.runId ? "managed-run-row-selected" : ""
                  }`}
                  onClick={() => onSelectRun(run.runId)}
                >
                  <span className={`run-state run-state-${run.state}`}>{run.state}</span>
                  <strong>{run.label}</strong>
                  <small>
                    PID {run.pid ?? "n/a"} - {formatRunTime(run.startedAtMs)}
                  </small>
                </button>
              ))}
            </div>
            {selectedRun && (
              <div className="managed-run-detail">
                <div className="managed-run-toolbar">
                  <code>{commandText(selectedRun.executable, selectedRun.arguments)}</code>
                  <div className="header-actions">
                    <button
                      type="button"
                      className="button button-secondary compact"
                      onClick={() => onStop(selectedRun.runId, false)}
                      disabled={
                        isTerminalState(selectedRun.state) || stoppingRunId === selectedRun.runId
                      }
                    >
                      <Square size={13} />
                      Stop
                    </button>
                    <button
                      type="button"
                      className="button button-danger compact"
                      onClick={() => onStop(selectedRun.runId, true)}
                      disabled={
                        isTerminalState(selectedRun.state) || stoppingRunId === selectedRun.runId
                      }
                    >
                      Force
                    </button>
                  </div>
                </div>
                <div className="managed-port-list">
                  {runPorts.length ? (
                    runPorts.map((port) => (
                      <div key={`${port.protocol}:${port.localAddress}:${port.localPort}`}>
                        <span>{port.protocol.toUpperCase()}</span>
                        <code>{port.localUrl ?? `${port.localAddress}:${port.localPort}`}</code>
                        <small>{port.bindingScope}</small>
                      </div>
                    ))
                  ) : (
                    <div>No listening port is currently owned by this PID.</div>
                  )}
                </div>
                <div className="log-pane" aria-busy={logsLoading}>
                  {logs.length ? (
                    logs.map((entry) => (
                      <div key={entry.sequence} className={`log-line log-${entry.stream}`}>
                        <span>{entry.stream}</span>
                        <code>{entry.line}</code>
                      </div>
                    ))
                  ) : (
                    <div className="empty-inline">No log lines captured yet.</div>
                  )}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="empty-inline">
            No managed commands have been started for this project.
          </div>
        )}
      </div>

      <div className="project-section">
        <div className="section-title">
          <GitBranch size={15} />
          <h3>Local Git state</h3>
        </div>
        {project.gitSummary ? (
          <div className="git-summary">
            <div>
              <span>Branch</span>
              <strong>
                {project.gitSummary.branch ??
                  (project.gitSummary.detachedHead ? "Detached HEAD" : "Unknown")}
              </strong>
            </div>
            <div>
              <span>Modified</span>
              <strong>{project.gitSummary.modified}</strong>
            </div>
            <div>
              <span>Staged</span>
              <strong>{project.gitSummary.staged}</strong>
            </div>
            <div>
              <span>Untracked</span>
              <strong>{project.gitSummary.untracked}</strong>
            </div>
          </div>
        ) : (
          <div className="empty-inline">Not a Git repository or Git unavailable.</div>
        )}
      </div>

      <div className="project-section">
        <div className="section-title">
          <FileKey2 size={15} />
          <h3>Environment metadata</h3>
        </div>
        {project.environmentFiles.length ? (
          <div className="env-list">
            {project.environmentFiles.map((file) => (
              <div key={file.relativePath}>
                <span>
                  <code>{file.relativePath}</code>
                  <small>{formatBytes(file.sizeBytes)}</small>
                </span>
                <span>{file.keyNames.length} key names - values hidden</span>
              </div>
            ))}
          </div>
        ) : (
          <div className="empty-inline">No environment files detected.</div>
        )}
      </div>
    </section>
  );
}

function commandText(executable: string, args: string[]) {
  return [executable, ...args].join(" ");
}

function isTerminalState(state: ManagedCommand["state"]) {
  return state === "exited" || state === "failed";
}

function formatRunTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
