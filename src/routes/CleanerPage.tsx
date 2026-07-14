import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  ArchiveRestore,
  CheckCircle2,
  FolderPlus,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";
import { ErrorState } from "../components/AsyncState";
import { cleanerApi, selectCleanupFolders } from "../lib/cleaner-ipc";
import { formatBytes } from "../lib/format";
import { taskApi } from "../lib/tasks-ipc";
import type { CleanupCandidate, CleanupPlan, QuarantineItem } from "../types/cleaner";

function candidateLabel(candidate: CleanupCandidate): string {
  return candidate.category.replace(/([A-Z])/g, " $1").toLowerCase();
}

export function CleanerPage() {
  const client = useQueryClient();
  const [roots, setRoots] = useState<string[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [plan, setPlan] = useState<CleanupPlan | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const [purgeTarget, setPurgeTarget] = useState<{
    manifestId: string;
    item: QuarantineItem;
  } | null>(null);
  const [purgeConfirmation, setPurgeConfirmation] = useState("");
  const [purgeManifestTarget, setPurgeManifestTarget] = useState<string | null>(null);
  const [purgeManifestConfirmation, setPurgeManifestConfirmation] = useState("");
  const [activeScanId, setActiveScanId] = useState<string | null>(null);

  const manifests = useQuery({
    queryKey: ["quarantineManifests"],
    queryFn: cleanerApi.manifests,
    refetchInterval: 2_000,
  });
  const tasks = useQuery({
    queryKey: ["backgroundTasks"],
    queryFn: taskApi.list,
    refetchInterval: 750,
  });
  const scanStart = useMutation({
    mutationFn: taskApi.startCleanupScan,
    onSuccess: () => {
      setSelectedIds(new Set());
      setPlan(null);
      setConfirmation("");
      void client.invalidateQueries({ queryKey: ["backgroundTasks"] });
    },
  });
  const latestScanTask =
    (activeScanId
      ? tasks.data?.find((task) => task.id === activeScanId)
      : tasks.data?.find((task) => task.kind === "cleanupScan")) ??
    scanStart.data ??
    null;
  const scanDetail = useQuery({
    queryKey: ["backgroundTask", latestScanTask?.id],
    queryFn: () => taskApi.get(latestScanTask?.id ?? ""),
    enabled: latestScanTask !== null,
    refetchInterval:
      latestScanTask?.state === "running" || latestScanTask?.state === "cancelling" ? 750 : false,
  });
  const scanResult =
    scanDetail.data?.output?.kind === "cleanupScan" ? scanDetail.data.output.value : null;
  const scanPending =
    scanStart.isPending ||
    latestScanTask?.state === "running" ||
    latestScanTask?.state === "cancelling";
  const createPlan = useMutation({
    mutationFn: ({ scanId, candidateIds }: { scanId: string; candidateIds: string[] }) =>
      cleanerApi.createPlan(scanId, candidateIds),
    onSuccess: (nextPlan) => {
      setPlan(nextPlan);
      setConfirmation("");
    },
  });
  const execute = useMutation({
    mutationFn: ({ planId, phrase }: { planId: string; phrase: string }) =>
      taskApi.startQuarantine({ planId, confirmation: phrase }),
    onSuccess: () => {
      setPlan(null);
      setSelectedIds(new Set());
      setConfirmation("");
      void client.invalidateQueries({ queryKey: ["backgroundTasks"] });
      void client.invalidateQueries({ queryKey: ["quarantineManifests"] });
    },
  });
  const restore = useMutation({
    mutationFn: ({
      manifestId,
      itemId,
      safe,
    }: {
      manifestId: string;
      itemId: string;
      safe: boolean;
    }) => cleanerApi.restore(manifestId, itemId, safe ? "safeAlternative" : "fail"),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["quarantineManifests"] }),
  });
  const purge = useMutation({
    mutationFn: ({
      manifestId,
      itemId,
      phrase,
    }: {
      manifestId: string;
      itemId: string;
      phrase: string;
    }) => cleanerApi.purge(manifestId, itemId, phrase),
    onSuccess: () => {
      setPurgeTarget(null);
      setPurgeConfirmation("");
      void client.invalidateQueries({ queryKey: ["quarantineManifests"] });
    },
  });
  const purgeManifest = useMutation({
    mutationFn: ({ manifestId, confirmation }: { manifestId: string; confirmation: string }) =>
      cleanerApi.purgeManifest(manifestId, confirmation),
    onSuccess: () => {
      setPurgeManifestTarget(null);
      setPurgeManifestConfirmation("");
      void client.invalidateQueries({ queryKey: ["quarantineManifests"] });
    },
  });

  const selected = useMemo(
    () => scanResult?.candidates.filter((candidate) => selectedIds.has(candidate.id)) ?? [],
    [scanResult?.candidates, selectedIds],
  );
  const selectedBytes = selected.reduce(
    (total, candidate) => total + candidate.estimatedSizeBytes,
    0,
  );
  const actionError =
    scanStart.error ??
    scanDetail.error ??
    createPlan.error ??
    execute.error ??
    restore.error ??
    purge.error ??
    purgeManifest.error;

  const chooseRoots = async () => {
    const selectedRoots = await selectCleanupFolders();
    if (selectedRoots.length) {
      setRoots(selectedRoots);
      setPlan(null);
      setActiveScanId("");
      scanStart.reset();
    }
  };
  const startScan = () => {
    if (!roots.length) return;
    const operationId = crypto.randomUUID();
    setActiveScanId(operationId);
    scanStart.mutate({ operationId, rootPaths: roots });
  };
  const toggleCandidate = (id: string) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
    setPlan(null);
  };

  return (
    <div className="page-stack page-fill">
      <header className="page-header">
        <div>
          <span className="eyebrow">Review · quarantine · restore</span>
          <h1>Cleaner</h1>
        </div>
        <div className="header-actions">
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void chooseRoots()}
          >
            <FolderPlus size={15} /> Choose folders
          </button>
          <button
            className="button button-primary"
            type="button"
            disabled={!roots.length || scanPending}
            onClick={startScan}
          >
            <RefreshCw className={scanPending ? "spin" : ""} size={15} /> Scan selected roots
          </button>
        </div>
      </header>

      <section className="cleaner-safety-strip">
        <ShieldCheck size={18} />
        <div>
          <strong>Filesystem safety boundary</strong>
          <span>
            Dangerous roots and reparse points are rejected. Scans do not read file contents, follow
            links, cross selected-root boundaries, or generate deletion commands.
          </span>
        </div>
      </section>

      <section className="cleaner-space-note">
        <ArchiveRestore size={18} />
        <div>
          <strong>Quarantine is reversible; purge reclaims disk space.</strong>
          <span>
            Quarantine moves selected artifacts out of projects into managed storage. It does not
            permanently delete them and usually does not free space until you explicitly purge.
          </span>
        </div>
      </section>

      {roots.length > 0 && (
        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Explicit scope</span>
              <h2>
                {roots.length} selected root{roots.length === 1 ? "" : "s"}
              </h2>
            </div>
          </div>
          <div className="cleaner-root-list">
            {roots.map((root) => (
              <div key={root}>
                <code>{root}</code>
                <button
                  className="icon-button"
                  type="button"
                  aria-label={`Remove ${root}`}
                  onClick={() => setRoots((items) => items.filter((item) => item !== root))}
                >
                  <X size={14} />
                </button>
              </div>
            ))}
          </div>
        </section>
      )}

      {scanPending && (
        <section className="scan-banner">
          <RefreshCw className="spin" size={15} />
          <span>Inspecting metadata within fixed depth, file-count, and candidate limits…</span>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => {
              if (latestScanTask?.id) void taskApi.cancel(latestScanTask.id);
            }}
          >
            Cancel
          </button>
        </section>
      )}
      {actionError && <ErrorState error={actionError} />}

      {scanResult && (
        <section className="panel">
          <div className="panel-heading cleaner-review-heading">
            <div>
              <span className="eyebrow">Read-only scan evidence</span>
              <h2>{scanResult.candidates.length} reviewable candidates</h2>
              <p>
                {formatBytes(scanResult.totalCandidateBytes)} across {scanResult.visitedEntries}{" "}
                bounded entries. Candidates are never selected automatically.
              </p>
            </div>
            <button
              className="button button-primary"
              type="button"
              disabled={
                !selectedIds.size ||
                createPlan.isPending ||
                scanResult.limitsReached ||
                scanResult.cancelled
              }
              onClick={() =>
                createPlan.mutate({
                  scanId: scanResult.scanId,
                  candidateIds: Array.from(selectedIds),
                })
              }
            >
              Review plan · {formatBytes(selectedBytes)}
            </button>
          </div>
          {(scanResult.cancelled || scanResult.limitsReached) && (
            <div className="inline-warning">
              This scan was {scanResult.cancelled ? "cancelled" : "truncated at a safety limit"} and
              cannot authorize a cleanup plan. Run a fresh, narrower scan.
            </div>
          )}
          <div className="cleaner-candidate-list">
            {scanResult.candidates.map((candidate) => (
              <label key={candidate.id} className="cleaner-candidate-row">
                <input
                  type="checkbox"
                  checked={selectedIds.has(candidate.id)}
                  onChange={() => toggleCandidate(candidate.id)}
                />
                <span className="cleaner-candidate-main">
                  <strong>{candidate.displayName}</strong>
                  <code>{candidate.canonicalPath}</code>
                  <small>{candidate.reason}</small>
                  <em>{candidate.regenerationInstructions}</em>
                </span>
                <span className={`cleaner-risk cleaner-risk-${candidate.risk}`}>
                  {candidate.risk}
                </span>
                <span>{candidateLabel(candidate)}</span>
                <span>{candidate.confidence}</span>
                <span>{formatBytes(candidate.estimatedSizeBytes)}</span>
                <span>{candidate.fileCount.toLocaleString()} files</span>
              </label>
            ))}
            {scanResult.candidates.length === 0 && (
              <div className="empty-command">No deterministic cleanup candidates were found.</div>
            )}
          </div>
          {scanResult.issues.length > 0 && (
            <details className="cleaner-issues">
              <summary>{scanResult.issues.length} skipped or inaccessible paths</summary>
              {scanResult.issues.slice(0, 50).map((issue, index) => (
                <div key={`${issue.path}-${index}`}>
                  <strong>{issue.code}</strong>
                  <code>{issue.path}</code>
                  <span>{issue.message}</span>
                </div>
              ))}
            </details>
          )}
        </section>
      )}

      {plan && (
        <section className="panel cleaner-plan">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Immutable cleanup plan</span>
              <h2>
                {plan.items.length} reviewed items · {formatBytes(plan.totalSizeBytes)}
              </h2>
              <p>
                Each item will be revalidated immediately before it moves. Changed, linked, locked,
                or escaped paths fail safely and stay in place.
              </p>
            </div>
            <CheckCircle2 size={22} />
          </div>
          <div className="cleaner-plan-items">
            {plan.items.map((item) => (
              <div key={item.id}>
                <code>{item.canonicalPath}</code>
                <span>{formatBytes(item.estimatedSizeBytes)}</span>
              </div>
            ))}
          </div>
          <label className="confirmation-phrase">
            Type <code>{plan.confirmationPhrase}</code> to move these items into managed quarantine.
            <input
              className="confirmation-input"
              value={confirmation}
              onChange={(event) => setConfirmation(event.target.value)}
              autoComplete="off"
            />
          </label>
          <button
            className="button button-primary"
            type="button"
            disabled={confirmation !== plan.confirmationPhrase || execute.isPending}
            onClick={() => execute.mutate({ planId: plan.id, phrase: confirmation })}
          >
            <ArchiveRestore size={15} /> Execute reversible quarantine
          </button>
        </section>
      )}

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Durable manifests</span>
            <h2>Quarantine history</h2>
            <p>Restore targets are exact by default. Existing content is never overwritten.</p>
          </div>
        </div>
        <div className="quarantine-list">
          {(manifests.data ?? []).map((manifest) => (
            <article key={manifest.id} className="quarantine-manifest">
              <header>
                <div>
                  <strong>Manifest {manifest.id.slice(0, 8)}</strong>
                  <span>{new Date(manifest.updatedAtMs).toLocaleString()}</span>
                </div>
                <div className="header-actions">
                  <span className={`state-pill state-${manifest.state}`}>{manifest.state}</span>
                  {manifest.items.some(
                    (item) =>
                      item.purgeEligible &&
                      (item.state === "quarantined" || item.state === "partial"),
                  ) && (
                    <button
                      className="button button-danger compact"
                      type="button"
                      onClick={() => {
                        setPurgeManifestTarget(manifest.id);
                        setPurgeManifestConfirmation("");
                      }}
                    >
                      <Trash2 size={13} /> Purge manifest
                    </button>
                  )}
                </div>
              </header>
              {manifest.items.map((item) => (
                <div key={item.id} className="quarantine-item-row">
                  <span>
                    <strong>{item.originalCanonicalPath}</strong>
                    <small>
                      {item.state} · {item.verification} · {formatBytes(item.sizeBytes)}
                    </small>
                    {item.error && <em>{item.error}</em>}
                  </span>
                  {(item.state === "quarantined" || item.state === "partial") && (
                    <div>
                      {item.state === "quarantined" && (
                        <>
                          <button
                            className="button button-secondary"
                            type="button"
                            disabled={restore.isPending}
                            onClick={() =>
                              restore.mutate({
                                manifestId: manifest.id,
                                itemId: item.id,
                                safe: false,
                              })
                            }
                          >
                            <RotateCcw size={13} /> Restore exact
                          </button>
                          <button
                            className="button button-secondary"
                            type="button"
                            disabled={restore.isPending}
                            title="Use only when the original path is occupied"
                            onClick={() =>
                              restore.mutate({
                                manifestId: manifest.id,
                                itemId: item.id,
                                safe: true,
                              })
                            }
                          >
                            Restore beside existing
                          </button>
                        </>
                      )}
                      <button
                        className="button button-danger"
                        type="button"
                        onClick={() => {
                          setPurgeTarget({ manifestId: manifest.id, item });
                          setPurgeConfirmation("");
                        }}
                      >
                        <Trash2 size={13} /> Purge
                      </button>
                    </div>
                  )}
                </div>
              ))}
            </article>
          ))}
          {manifests.isLoading && <div className="empty-command">Loading local manifests…</div>}
          {!manifests.isLoading && (manifests.data?.length ?? 0) === 0 && (
            <div className="empty-command">No quarantine operations have been executed.</div>
          )}
        </div>
      </section>

      {purgeTarget && (
        <div
          className="cleaner-purge-callout"
          role="dialog"
          aria-modal="true"
          aria-label="Confirm purge"
        >
          <AlertTriangle size={22} />
          <div>
            <h2>Permanent purge</h2>
            <p>
              This permanently removes the managed quarantine copy. It cannot be restored by Desktop
              Manager afterward.
            </p>
            <code>{purgeTarget.item.quarantinePath}</code>
            <label className="confirmation-phrase">
              Type <code>{purgeTarget.item.purgeConfirmationPhrase}</code>
              <input
                className="confirmation-input"
                value={purgeConfirmation}
                onChange={(event) => setPurgeConfirmation(event.target.value)}
                autoComplete="off"
              />
            </label>
            <div className="confirm-actions">
              <button
                className="button button-secondary"
                type="button"
                onClick={() => setPurgeTarget(null)}
              >
                Cancel
              </button>
              <button
                className="button button-danger"
                type="button"
                disabled={
                  purgeConfirmation !== purgeTarget.item.purgeConfirmationPhrase || purge.isPending
                }
                onClick={() =>
                  purge.mutate({
                    manifestId: purgeTarget.manifestId,
                    itemId: purgeTarget.item.id,
                    phrase: purgeConfirmation,
                  })
                }
              >
                Permanently purge
              </button>
            </div>
          </div>
        </div>
      )}

      {purgeManifestTarget && (
        <div
          className="cleaner-purge-callout"
          role="dialog"
          aria-modal="true"
          aria-label="Confirm manifest purge"
        >
          <AlertTriangle size={22} />
          <div>
            <h2>Permanently purge this quarantine manifest?</h2>
            <p>
              Every eligible quarantined item in this manifest will be deleted. Mr Manager cannot
              restore purged items.
            </p>
            <label className="confirmation-phrase">
              Type <code>PURGE MANIFEST {purgeManifestTarget}</code>
              <input
                className="confirmation-input"
                value={purgeManifestConfirmation}
                onChange={(event) => setPurgeManifestConfirmation(event.target.value)}
                autoComplete="off"
              />
            </label>
            <div className="confirm-actions">
              <button
                className="button button-secondary"
                type="button"
                onClick={() => setPurgeManifestTarget(null)}
              >
                Cancel
              </button>
              <button
                className="button button-danger"
                type="button"
                disabled={
                  purgeManifestConfirmation !== `PURGE MANIFEST ${purgeManifestTarget}` ||
                  purgeManifest.isPending
                }
                onClick={() =>
                  purgeManifest.mutate({
                    manifestId: purgeManifestTarget,
                    confirmation: purgeManifestConfirmation,
                  })
                }
              >
                Permanently purge all eligible items
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
