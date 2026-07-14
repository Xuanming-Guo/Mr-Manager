# Mr Manager architecture

## Trust boundary

Mr Manager is a Tauri 2 application. React renders local state and invokes a small typed API; Rust owns operating-system access, persistence, subprocesses, networking, Docker, and every state-changing operation. The webview receives no generic shell or filesystem capability.

The main window is identified by the stable `main` label and receives only generated allow-permissions for registered commands. Future localhost preview windows use a different label and receive no application-command capability. Local content is therefore treated as untrusted even when it is served from loopback.

## Frontend

- React and strict TypeScript are organized by app shell, routes, reusable components, typed IPC, queries, and domain DTOs.
- TanStack Query owns backend state, retries, staleness, and refresh intervals. UI state remains component-local.
- Browser/web mode never fabricates system data; it shows `DESKTOP_RUNTIME_REQUIRED`.
- Process rows are virtualized and metric/log histories must remain bounded.
- Unsupported, unavailable, permission-denied, partial, and error states remain distinct.

## Rust backend

The backend is split into narrow modules:

- `domain`: serialized contracts, stable errors, process identity, availability, and settings;
- `collector`: coordinated `sysinfo` refresh plus joins between process and owned-port evidence;
- `platform`: Windows-specific TCP/UDP ownership, power, network-adapter inventory, and hidden non-interactive child-process construction behind portable functions;
- `projects`: explicit-root validation, bounded project discovery, manifest/Git/environment scanners, and project metadata normalization;
- `supervisor`: allowlisted project command launch, Windows Job Object assignment, bounded redacted logs, and managed stop controls;
- `docker`: exact Docker CLI adapters, structured inventory parsing, confirmed container lifecycle actions, bounded redacted logs, registered-project Compose parsing, and deterministic Compose Doctor rules;
- `integrations`: constrained detector registry for runtimes, editors, databases, Docker, Ollama, WSL, VPN clients, and local services using evidence, timeouts, and exact probe vectors;
- `networking`: local adapter throughput monitor, private LAN candidates, gateway/DNS/VPN evidence, localhost-sharing warnings, unavailable per-process throughput state, and explicit opt-in internet diagnostics;
- `system_diagnostics`: coordinated high-frequency local metrics, GPU provider abstraction, bounded recording sessions, annotations, downsampling, redacted export/delete, and deterministic non-causal correlation analysis;
- `cleaner`: explicit-root bounded/cancellable traversal, deterministic candidate rules, server-held scan evidence, immutable cleanup plans, revalidation, verified quarantine/restore/purge, and reparse-boundary enforcement;
- `tasks`: bounded in-session Rust-owned task registry for Cleaner scans/quarantine, project discovery, and explicit Internet Test orchestration, with typed results and cancellation only at safe checkpoints;
- `commands`: allowlisted Tauri commands and background blocking work;
- `db`: versioned SQLite migrations and transactional durable state;
- `security`: log/command redaction and future trust-boundary validators;
- `logging`: bounded rotating local logs with redaction before persistence.
- `topology`: evidence graph construction for projects, managed runs, processes, ports, and URLs.

`ProcessKey { pid, start_time }` prevents PID reuse from silently changing identity. Windows owned endpoint tables provide certain PID-to-port evidence. Scheme guesses remain explicitly inferred. Managed project runs additionally record the launched PID and use the existing port collector to display current listening URLs owned by that PID.

The Topology page defaults to live development services. A listening development URL can produce process, port, and URL nodes even when no project association is proven; project links are added only from managed launch, process CWD, redacted command-line root, or other recorded evidence. URL preview opens in a separately labeled webview. The main capability manifest grants commands only to the `main` window, so preview windows receive no Mr Manager IPC permissions.

The Docker page uses Docker's structured CLI output rather than shell strings. It distinguishes missing CLI, stopped daemon, inaccessible daemon, and running states. Container lifecycle actions are limited to start, stop, and restart, and require an exact confirmation phrase based on the selected Docker object. Compose files are discovered only through registered project metadata; canonical Docker Compose JSON is preferred when available, with a conservative bounded fallback parser labeled in the UI.

The Integrations page uses a fixed detector registry. Detectors may look at PATH, running processes, owned local ports, known loopback endpoints, WSL state, and bounded exact version commands. They do not accept custom shell strings. Ollama is inspected only through local process/executable evidence and the loopback API on `127.0.0.1:11434`; Mr Manager never pulls or deletes models. WSL state is read with `wsl.exe --list --verbose` only.

The Network dashboard separates local-only adapter throughput from external diagnostics. Local views use OS/network counters and labelled bit-rate charts without external requests. DNS server addresses, MAC addresses, SSIDs, public IP addresses, remote endpoints, and network history are not exposed by default. Internet tests require the Settings opt-in, an explicit preflight confirmation, and the `Run Internet Test` action; every result records whether it was local-only or contacted the internet.

System Diagnostics combines coherent collector snapshots, local network snapshots, Docker process evidence, local dev-server ports, and GPU provider output. The first GPU provider is a bounded exact `nvidia-smi.exe` invocation when present; missing or unsupported GPU data is shown as unsupported rather than fabricated. Metric recordings are explicit sessions only. Active sessions use an in-memory ring buffer; completed sessions are downsampled before persistence when necessary and can be exported as redacted JSON or deleted. Network and recorded CPU/RAM timelines share one responsive uPlot presentation primitive with unit-specific axes.

The Cleaner never accepts an arbitrary deletion path. A scan starts from explicit roots and produces bounded server-held evidence. Candidate IDs create a persisted immutable plan; execution loads that plan server-side and revalidates canonical path, root containment, reparse ancestry, kind, size, count, and fingerprint. Same-volume quarantine uses atomic rename. Cross-volume fallback checks free space, copies, verifies, and only then removes the source. SQLite manifests are updated after each item. Restore never overwrites. Item purge and whole-manifest purge are separate irreversible actions with exact confirmation, constrained to application quarantine storage. See [Cleaner safety](CLEANER_SAFETY.md).

## Persistence

SQLite lives in the Tauri application-data directory, never the repository. Foreign keys are enabled and migrations run transactionally. Settings, project registry metadata, explicit completed metric recording sessions, immutable cleanup plans, and quarantine manifests are active durable state; raw environment values, managed process logs, and indefinite process snapshots are not. Schema-5 content tables remain dormant solely to preserve data from the retired Content Studio feature and are not exposed through application commands. Migration 6 maps legacy refresh values to `normal` or `fast`. The legacy Tauri identifier, `desktop-manager.sqlite3`, and log filenames remain unchanged so rebranding cannot orphan existing data. Managed run history is in memory in Phase 2 and can become a bounded durable history feature later.

Logs and quarantine content use scoped application-data directories. Retired Content Studio files are left untouched in application data. Generated databases, logs, captured machine state, and binaries are ignored by Git.

## Collection and events

Phase 1 exposes coherent snapshots through background blocking collection. System Diagnostics builds frequent typed snapshots and recording samples from the same narrow collectors. Normal mode refreshes metric views every 2.5 seconds and Fast mode every second. Later phases can evolve this into one cancellable event coordinator that publishes typed sequence-numbered deltas for metrics, lifecycle, logs, scans, Docker changes, and recordings. Expensive work is reduced while hidden unless a recording is active.

## Future CLI path

Domain rules, parsers, detectors, cleaner planning, topology construction, and platform traits must remain independent of Tauri command wrappers. A future CLI can call the same services without inheriting webview concerns.
