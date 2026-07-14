# Mr Manager decision log

This is a lightweight architecture decision record. Accepted decisions apply until a later entry explicitly supersedes them.

## D-001 - Mr Manager is the canonical product name

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Use **Mr Manager** in package metadata, UI, documentation, database/application directories, and artifacts.
- **Reason:** This is the requested brand. A legacy attachment filename is source context, not product naming authority.
- **Consequence:** New identifiers should use a stable Mr Manager form. Renaming stable persisted identifiers later requires a migration rather than an ad hoc string change.

## D-002 - Tauri/Rust is the privileged boundary

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** The React webview renders data and requests typed operations; Rust owns OS inspection, processes, ports, filesystem access, Docker, subprocesses, networking, persistence, and mutations.
- **Reason:** A webview must not inherit broad machine access.
- **Consequence:** No generic shell plugin or broad filesystem API is exposed to JavaScript. Tauri commands are narrow, scoped, validated in Rust, and backed by testable domain services.

## D-003 - Local-first means zero clean-launch egress

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** No login, telemetry, cloud persistence, API key, remote font/CDN, update check, or external probe occurs on a clean first launch.
- **Reason:** Offline usefulness and user trust are core product requirements.
- **Consequence:** Network-dependent diagnostics must be explicit, narrowly scoped, previewed, and disabled by default. Loopback probes are distinguished from external network access.

## D-004 - Windows-first, portable domain architecture

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Implement and verify Windows 11 first while defining platform traits and typed capability results.
- **Reason:** Windows is the first production target, but domain rules should not be coupled to one API.
- **Consequence:** Linux/macOS modules can return typed `Unsupported` results. The UI never substitutes fake values or panics for unsupported data.

## D-005 - Read-only by default with explicit mutation contracts

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Inspection is read-only. Every mutation uses a typed action, server-side resolution, exact target/effect preview, confirmation, and immediate revalidation.
- **Reason:** Mr Manager observes and manages valuable developer state.
- **Consequence:** Process termination, command launch, container actions, file replacement, quarantine, restore, purge, autostart, and similar actions cannot be hidden side effects of refresh or navigation.

## D-006 - Typed DTOs and stable application errors cross IPC

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Rust domain objects map to explicit TypeScript DTOs. Errors carry a stable code, safe message/details, remediation, retryability, and permission relevance.
- **Reason:** Arbitrary JSON and raw backend errors make partial/permission states hard to handle safely.
- **Consequence:** IPC payload changes are deliberate contracts. Raw panic traces, unredacted OS errors, and loosely shaped JSON are not UI APIs.

## D-007 - SQLite stores durable non-secret state

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Use versioned SQLite migrations for registered projects, user-authored metadata, settings, managed-run metadata, bounded recording data, and cleanup manifests. Retired schema-5 content records remain stored but inactive.
- **Reason:** SQLite provides local transactional persistence without a cloud dependency.
- **Consequence:** Foreign keys are enabled, multi-step changes use transactions, and raw environment values or indefinite full process snapshots are never stored.

## D-008 - Relationships require inspectable evidence

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Every topology edge records evidence and one of `certain`, `strong`, or `inferred`.
- **Reason:** Developer-machine relationships are frequently heuristic and must not be overstated.
- **Consequence:** PID ownership and parentage can be certain; path-based project association can be strong; port-name or VPN heuristics remain inferred. The UI exposes why and when an edge was made.

## D-009 - Stable process identity is PID plus start time

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Use `ProcessKey { pid, start_time }` for identity and revalidate it before termination or other sensitive process actions.
- **Reason:** Operating systems reuse PIDs.
- **Consequence:** A stale snapshot cannot authorize action against a new process with the same PID. Unverifiable identity fails safely.

## D-010 - Project commands use structured execution

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Resolve an executable and argument vector in Rust and launch without a shell unless a narrowly reviewed platform adapter makes a shell unavoidable.
- **Reason:** Project metadata and paths are untrusted command-injection inputs.
- **Consequence:** The UI previews exact structured execution. Free-form frontend command strings and interpolation into `cmd.exe` or PowerShell are prohibited.

## D-011 - Cleaner is a quarantine transaction, not a delete shortcut

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Scanning creates candidates; the user reviews an immutable plan; execution revalidates and moves eligible content into quarantine with a manifest; restore is supported; purge is separate and irreversible.
- **Reason:** Build-artifact rules and filesystem state can be wrong or change during a scan.
- **Consequence:** Dangerous roots, boundary escape, reparse points, locks, destination conflicts, and time-of-check/time-of-use changes must have explicit safe outcomes and fixture tests.

## D-012 - Preview content gets a separate zero-privilege webview

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Localhost previews use a separately labeled webview/window with no Mr Manager command capability, constrained navigation, and no inherited main-window privileges.
- **Reason:** A local development page can be compromised or intentionally hostile.
- **Consequence:** Convenience cannot justify embedding a privileged main webview. Unexpected origins open externally only after confirmation or are blocked.

## D-013 - Core generation and diagnostics are deterministic

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Project detection, Compose Doctor, performance correlation, cleaner rules, and OS Lab work without an LLM.
- **Reason:** The core product must work privately, offline, and reproducibly.
- **Consequence:** An optional future local Ollama assistant can explain existing evidence but cannot be required for correctness or safety decisions.

## D-014 - Phase 9 produces an unsigned internal executable, then work stops

- **Status:** Accepted; hard gate
- **Date:** 2026-07-10
- **Decision:** After all Phase 0-9 acceptance and checks, build a raw unsigned Windows `mr-manager.exe` for internal manual review, record its SHA-256, keep it out of Git, and stop.
- **Reason:** The user requires hands-on application review before final open-source/release work.
- **Consequence:** Phase 10 cannot begin without explicit user approval. The internal executable is not a public release or installer and must be labeled as unsigned.

## D-015 - Release signing and updater integrity belong to Phase 10

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Do not enable updater installation or claim a trusted distributable until release signing, provenance, hash publication, rollback, and workflow integrity are deliberately configured after approval.
- **Reason:** An updater is a high-impact supply-chain boundary.
- **Consequence:** Development and the Phase 9 internal build do not perform update checks. Any future updater uses verified signed metadata and an explicit documented policy.

## D-016 - Managed project commands are allowlisted and in-memory in Phase 2

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Phase 2 launches only scanner-detected project scripts for registered projects, stores live run state/logs in memory, assigns Windows child processes to a Job Object, and exposes stop controls only for Mr Manager-managed runs.
- **Reason:** Project manifests are untrusted, and durable process history needs separate retention/privacy design.
- **Consequence:** The UI shows exact executable/argument vectors and redacted bounded logs. Free-form command execution and external-process termination remain unavailable until separately designed and confirmed.

## D-017 - Local throughput and internet diagnostics are separate trust zones

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** The Network dashboard separates local adapter throughput from external internet diagnostics. Local throughput reads OS counters and may run on clean launch without external requests; latency, DNS, packet-loss, download, upload, public-route, or VPN leak-style checks that contact internet endpoints remain disabled until the user explicitly starts a labelled test.
- **Reason:** Adapter counters help diagnose local slowdowns offline, while speed tests and public diagnostics disclose network context, may consume data, and violate the clean-launch egress guarantee if run implicitly.
- **Consequence:** Network results must state whether they were local-only or contacted the internet. Exports redact SSIDs, MAC addresses, public IPs, remote endpoints, DNS servers, and network history by default, and unavailable per-process network usage is shown as unavailable rather than guessed.

## D-018 - Docker uses exact CLI operations and excludes destructive MVP actions

- **Status:** Accepted
- **Date:** 2026-07-10
- **Decision:** Phase 4 Docker inspection uses exact `docker` executable invocations with fixed argument arrays, structured output, timeouts, bounded redacted logs, and typed confirmation for container start/stop/restart. Mr Manager does not expose Docker delete, remove, prune, volume deletion, network deletion, or image deletion actions in the MVP.
- **Reason:** Docker is a high-impact local state boundary. The app needs visibility and safe lifecycle controls without becoming a Docker Desktop replacement or risking destructive state changes.
- **Consequence:** Docker absence, stopped daemon, inaccessible daemon, and malformed output are typed states. Compose parsing prefers canonical `docker compose config --format json` when available and falls back to a conservative bounded parser with explicit source labeling.

## D-019 - Content generation is reviewed evidence, not automated authorship

- **Status:** Superseded by D-021
- **Date:** 2026-07-11
- **Decision:** README Forge and Portfolio Vault use deterministic templates over registered project evidence and user-authored fields. README apply is a confirmed stale-checked backup transaction; assets default to references and are copied only through explicit managed mode.
- **Reason:** Documentation and portfolio claims can leak secrets, overwrite valuable text, or fabricate accomplishments if generation is treated as authoritative.
- **Consequence:** Commands retain verified/not-verified labels, environment values are unavailable, common secret shapes are redacted, exports omit asset paths, imports are bounded/versioned, and no LLM or external request is part of correctness.

## D-020 - Long operations are Rust-owned and topology is live-service first

- **Status:** Accepted
- **Date:** 2026-07-12
- **Decision:** Cleaner scans/quarantine, project discovery, and explicit Internet Test runs use a bounded typed in-session task registry. Topology includes evidenced live development listeners even when no project association exists.
- **Reason:** Route-local mutations lost status after navigation, and project-gated topology hid the live process/port information users expected.
- **Consequence:** Navigation does not cancel active tasks. Non-durable scans still stop with the app process, quarantine manifests remain recoverable, and unassociated services stay visibly unassociated rather than receiving guessed project edges.

## D-021 - Content Studio and Phase 8A agents are retired without data deletion

- **Status:** Accepted
- **Date:** 2026-07-14
- **Decision:** Remove Content Studio, README Forge, Portfolio Vault, and all unfinished AI, DeepSeek, MCP, and Obsidian work from active code and IPC. Preserve schema-5 tables and existing application-data files without exposing or mutating them.
- **Reason:** The user explicitly removed these features from the product scope and requested a smaller reliable command center.
- **Consequence:** `/portfolio` redirects to Projects, content commands and permissions are absent, no AI/cloud dependencies remain, and no destructive database migration or application-data cleanup runs.

## D-022 - Mr Manager rebranding preserves the established local-data identity

- **Status:** Accepted
- **Date:** 2026-07-14
- **Decision:** Rename the product, package, executable, active code, and documentation to Mr Manager; rename the monitoring surface to System Diagnostics; and expose only Normal and Fast refresh modes. Retain the legacy Tauri identifier, SQLite filename, log filenames, metric-recording tables, and application-data location.
- **Reason:** Branding and clearer diagnostics language should not orphan existing projects, settings, recordings, quarantine manifests, or dormant schema-5 content records.
- **Consequence:** The new binary is `mr-manager.exe`, `/war-room` redirects to `/system-diagnostics`, migration 6 maps legacy refresh values to `normal` or `fast`, and explicit compatibility identifiers remain the only intentional `desktop-manager` strings.
