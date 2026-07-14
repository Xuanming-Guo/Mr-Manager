import { useVirtualizer } from "@tanstack/react-virtual";
import * as Dialog from "@radix-ui/react-dialog";
import { ArrowDown, Search, X } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { formatBytes, formatPercent } from "../lib/format";
import type { ProcessSnapshot } from "../types/system";
import type { Project } from "../types/projects";

type SortKey = "cpu" | "memory" | "name" | "pid";

type ProcessScope = "all" | "listening" | "managed" | "external";
type AssociationFilter = "all" | "project" | "unassociated";
type AccessFilter = "all" | "accessible" | "protected";

export function ProcessTable({
  processes,
  projects,
}: {
  processes: ProcessSnapshot[];
  projects: Project[];
}) {
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortKey>("cpu");
  const [scope, setScope] = useState<ProcessScope>("all");
  const [association, setAssociation] = useState<AssociationFilter>("all");
  const [minimumCpu, setMinimumCpu] = useState(0);
  const [minimumMemory, setMinimumMemory] = useState(0);
  const [access, setAccess] = useState<AccessFilter>("all");
  const [selected, setSelected] = useState<ProcessSnapshot | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return processes
      .filter((process) => {
        if (scope === "listening") return process.listeningPortCount > 0;
        if (scope === "managed") return process.managedByMrManager;
        if (scope === "external") return !process.managedByMrManager;
        return true;
      })
      .filter((process) => {
        if (association === "all") return true;
        const associated = associatedProject(process, projects) !== null;
        return association === "project" ? associated : !associated;
      })
      .filter((process) => process.cpuPercent >= minimumCpu)
      .filter((process) => process.memoryBytes >= minimumMemory)
      .filter((process) => {
        if (access === "all") return true;
        return access === "accessible"
          ? process.protectedState === "accessible"
          : process.protectedState !== "accessible";
      })
      .filter(
        (process) =>
          !needle ||
          process.name.toLowerCase().includes(needle) ||
          String(process.key.pid).includes(needle) ||
          process.executablePath?.toLowerCase().includes(needle),
      )
      .sort((a, b) => {
        if (sort === "cpu") return b.cpuPercent - a.cpuPercent;
        if (sort === "memory") return b.memoryBytes - a.memoryBytes;
        if (sort === "pid") return a.key.pid - b.key.pid;
        return a.name.localeCompare(b.name);
      });
  }, [access, association, minimumCpu, minimumMemory, processes, projects, query, scope, sort]);

  // TanStack Virtual intentionally owns mutable measurement state; React Compiler should not memoize it.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 44,
    overscan: 12,
  });

  return (
    <div className="data-table-shell">
      <div className="table-toolbar">
        <label className="search-field">
          <Search size={15} />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search process name, PID, or path"
          />
        </label>
        <label className="select-field">
          Scope
          <select value={scope} onChange={(event) => setScope(event.target.value as ProcessScope)}>
            <option value="all">All processes</option>
            <option value="listening">Listening</option>
            <option value="managed">Mr Manager managed</option>
            <option value="external">External / read-only</option>
          </select>
        </label>
        <label className="select-field">
          Project
          <select
            value={association}
            onChange={(event) => setAssociation(event.target.value as AssociationFilter)}
          >
            <option value="all">Any association</option>
            <option value="project">Project-associated</option>
            <option value="unassociated">Unassociated</option>
          </select>
        </label>
        <label className="select-field">
          Min CPU
          <select
            value={minimumCpu}
            onChange={(event) => setMinimumCpu(Number(event.target.value))}
          >
            <option value={0}>Any</option>
            <option value={1}>1%</option>
            <option value={5}>5%</option>
            <option value={10}>10%</option>
          </select>
        </label>
        <label className="select-field">
          Min memory
          <select
            value={minimumMemory}
            onChange={(event) => setMinimumMemory(Number(event.target.value))}
          >
            <option value={0}>Any</option>
            <option value={100 * 1024 * 1024}>100 MB</option>
            <option value={500 * 1024 * 1024}>500 MB</option>
            <option value={1024 * 1024 * 1024}>1 GB</option>
          </select>
        </label>
        <label className="select-field">
          Access
          <select
            value={access}
            onChange={(event) => setAccess(event.target.value as AccessFilter)}
          >
            <option value="all">Any access</option>
            <option value="accessible">Accessible</option>
            <option value="protected">Protected / unknown</option>
          </select>
        </label>
        <label className="select-field">
          Sort
          <select value={sort} onChange={(event) => setSort(event.target.value as SortKey)}>
            <option value="cpu">CPU</option>
            <option value="memory">Memory</option>
            <option value="name">Name</option>
            <option value="pid">PID</option>
          </select>
        </label>
        <span className="row-count">
          {rows.length} of {processes.length}
        </span>
      </div>
      <div className="process-grid process-grid-header" role="row">
        <button onClick={() => setSort("name")} role="columnheader">
          Process <ArrowDown size={12} />
        </button>
        <button onClick={() => setSort("pid")} role="columnheader">
          PID
        </button>
        <span role="columnheader">Status</span>
        <button onClick={() => setSort("cpu")} role="columnheader">
          CPU
        </button>
        <button onClick={() => setSort("memory")} role="columnheader">
          Memory
        </button>
        <span role="columnheader">Ports</span>
      </div>
      <div ref={scrollRef} className="virtual-scroll" role="table" aria-rowcount={rows.length}>
        <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>
          {virtualizer.getVirtualItems().map((virtualRow) => {
            const process = rows[virtualRow.index];
            if (!process) return null;
            return (
              <div
                key={`${process.key.pid}:${process.key.startTime}`}
                className="process-grid process-grid-row"
                role="row"
                tabIndex={0}
                onClick={() => setSelected(process)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    setSelected(process);
                  }
                }}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  height: `${virtualRow.size}px`,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
                title={process.executablePath ?? process.name}
              >
                <div role="cell" className="process-name-cell">
                  <span className="process-dot" />
                  <span>
                    <strong>{process.name}</strong>
                    <small>{process.cwd ?? process.executablePath ?? "Path unavailable"}</small>
                  </span>
                </div>
                <code role="cell">{process.key.pid}</code>
                <span role="cell" className="muted-cell">
                  {process.status}
                </span>
                <strong role="cell">{formatPercent(process.cpuPercent)}</strong>
                <span role="cell">{formatBytes(process.memoryBytes)}</span>
                <span role="cell">{process.listeningPortCount || "—"}</span>
              </div>
            );
          })}
        </div>
      </div>
      <Dialog.Root open={selected !== null} onOpenChange={(open) => !open && setSelected(null)}>
        <Dialog.Portal>
          <Dialog.Overlay className="drawer-overlay" />
          <Dialog.Content className="process-drawer" aria-describedby={undefined}>
            <div className="drawer-heading">
              <div>
                <span className="eyebrow">Process identity</span>
                <Dialog.Title>{selected?.name ?? "Process"}</Dialog.Title>
                <p>
                  PID {selected?.key.pid} · started{" "}
                  {selected ? new Date(selected.key.startTime * 1000).toLocaleString() : "—"}
                </p>
              </div>
              <Dialog.Close className="icon-button" aria-label="Close process details">
                <X size={17} />
              </Dialog.Close>
            </div>
            {selected && (
              <div className="drawer-body">
                <div className="drawer-metrics">
                  <div>
                    <span>CPU</span>
                    <strong>{formatPercent(selected.cpuPercent)}</strong>
                  </div>
                  <div>
                    <span>Memory</span>
                    <strong>{formatBytes(selected.memoryBytes)}</strong>
                  </div>
                  <div>
                    <span>Read I/O</span>
                    <strong>{formatBytes(selected.diskReadBytes)}</strong>
                  </div>
                  <div>
                    <span>Write I/O</span>
                    <strong>{formatBytes(selected.diskWriteBytes)}</strong>
                  </div>
                </div>
                <dl className="detail-list">
                  <div>
                    <dt>Stable key</dt>
                    <dd>
                      <code>
                        {selected.key.pid}:{selected.key.startTime}
                      </code>
                    </dd>
                  </div>
                  <div>
                    <dt>Parent PID</dt>
                    <dd>{selected.parentPid ?? "Unavailable"}</dd>
                  </div>
                  <div>
                    <dt>Status</dt>
                    <dd>{selected.status}</dd>
                  </div>
                  <div>
                    <dt>Access</dt>
                    <dd>{selected.protectedState}</dd>
                  </div>
                  <div>
                    <dt>Listening ports</dt>
                    <dd>{selected.listeningPortCount}</dd>
                  </div>
                  <div>
                    <dt>Managed by Mr Manager</dt>
                    <dd>{selected.managedByMrManager ? "Yes" : "No — read only"}</dd>
                  </div>
                  <div className="detail-wide">
                    <dt>Executable</dt>
                    <dd>
                      <code>
                        {selected.executablePath ??
                          "Unavailable due to process lifetime or permissions"}
                      </code>
                    </dd>
                  </div>
                  <div className="detail-wide">
                    <dt>Working directory</dt>
                    <dd>
                      <code>{selected.cwd ?? "Unavailable"}</code>
                    </dd>
                  </div>
                  <div className="detail-wide">
                    <dt>Command line (redacted)</dt>
                    <dd>
                      <code>{selected.commandLineRedacted ?? "Unavailable"}</code>
                    </dd>
                  </div>
                </dl>
                <div className="drawer-notice">
                  External processes are inspection-only in this milestone. No terminate action is
                  exposed here.
                </div>
              </div>
            )}
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  );
}

function associatedProject(process: ProcessSnapshot, projects: Project[]): Project | null {
  if (!process.cwd) return null;
  const cwd = normalizeWindowsPath(process.cwd);
  return (
    projects.find((project) => {
      const root = normalizeWindowsPath(project.rootPath);
      return cwd === root || cwd.startsWith(`${root}\\`);
    }) ?? null
  );
}

function normalizeWindowsPath(value: string) {
  return value.replaceAll("/", "\\").replace(/\\+$/, "").toLowerCase();
}
