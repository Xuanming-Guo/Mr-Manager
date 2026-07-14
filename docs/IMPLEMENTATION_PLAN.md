# Mr Manager implementation plan

## Purpose and authority

This plan translates the product specification into sequential, verifiable vertical slices. The attached product specification is authoritative when it conflicts with a convenience choice in this plan. Later user direction is also authoritative: produce an unsigned internal Windows executable after Phase 9, stop before Phase 10, and wait for manual approval. The 2026-07-10 Network Throughput and Internet Diagnostics scope update remains part of the network and recording milestones. The 2026-07-14 directions retire Content Studio and unfinished agent work without deleting local data, establish Mr Manager as the product name, rename System Diagnostics, and reduce refresh modes to Normal and Fast while retaining legacy storage identifiers.

Mr Manager is Windows-first and cross-platform by architecture. It must use real local evidence, remain fully useful without AI or cloud services, make zero external requests on a clean first launch, and default to read-only inspection.

## Delivery method

Each phase must:

- start from a runnable prior phase;
- deliver an end-to-end workflow rather than disconnected stubs;
- expose unavailable, unsupported, permission-denied, and partial states honestly;
- include typed Rust/TypeScript boundaries and deterministic tests;
- add or update synthetic fixtures without touching real user data;
- run the checks appropriate to the changed surface;
- update `IMPLEMENTATION_STATUS.md`, `DECISIONS.md`, and security/privacy documentation where relevant.

A phase is not complete when its UI is mocked, its system data is fabricated, or its acceptance flow has not been verified.

## Dependency path

```text
Phase 0: secure foundation
  -> Phase 1: real process/system/port data
     -> Phase 2: projects + managed process supervisor
        -> Phase 3: evidence topology + localhost bridge
           -> Phase 4: Docker/Compose
           -> Phase 5: detector framework + network throughput/diagnostics integrations
              -> Phase 6: recordings, network timeline, and correlation
                 -> Phase 7: transactional cleaner
                    -> Phase 8: content capability retired; legacy data preserved
                       -> Phase 9: isolated educational simulators
                          -> unsigned internal .exe + manual review
                             -> STOP: explicit approval required
                                -> Phase 10: release quality
```

Phases remain sequential even where individual preparatory interfaces can be introduced earlier. Safety primitives such as typed errors, redaction, path validation, capability isolation, confirmation models, and fixture-only mutation tests are cross-cutting dependencies and cannot be deferred.

## Phase 0 - Repository and engineering foundation

### Deliverables

- [x] Tauri 2, React, strict TypeScript, and Vite dependency/configuration baseline exists.
- [x] Frontend lint, format, typecheck, test, and build scripts exist.
- [x] Rust crate manifest and pinned foundational dependencies exist.
- [x] Initial SQLite migration exists for settings and registered projects.
- [x] Initial narrow main-window capability manifest and CSP exist.
- [x] Initial TypeScript IPC/domain DTO files exist.
- [x] Complete the runnable app shell, navigation, accessibility baseline, and honest unavailable states.
- [x] Complete the Rust application entry point, typed command boundary, error model, migration runner, settings repository, and redacted logging.
- [x] Generate/verify Tauri command ACL schema bindings for the commands actually registered.
- [x] Add frontend and Rust foundation tests.
- [x] Add CI for frontend and Rust checks.
- [x] Add repository agent guidance and implementation/status/decision documentation.
- [x] Add architecture, threat model, privacy, and security documentation.
- [x] Run and pass every Phase 0 check on the integrated worktree.

### Exit criteria

The Tauri application launches without an account or external request, migrations apply to an app-data database, settings round-trip across typed IPC, capabilities expose no generic shell/filesystem access, and all foundation checks pass.

## Phase 1 - Real system foundation

### Deliverables

- [x] Implement a coordinated Windows system collector for CPU, memory, disk, uptime, and battery availability when supported.
- [x] Implement process enumeration with `ProcessKey { pid, start_time }`, protected/permission states, redacted command lines, and resource fields.
- [x] Implement TCP/UDP listening-port ownership through supported Windows APIs.
- [x] Add typed snapshot commands and refresh policies.
- [x] Build the overview from real data, a virtualized process table, filters, and a process detail drawer.
- [x] Expose unsupported, inaccessible, stale, and partial fields explicitly.
- [x] Add unit/integration tests around PID identity, redaction, binding/URL inference, Windows table parsing, and live read-only collection.

### Acceptance

On Windows 11, Mr Manager displays current real processes and resource information and maps common listening ports to owning PIDs. Absence or denied access is visible and does not crash the app.

## Phase 2 - Projects and managed commands

### Deliverables

- [x] Add explicit project and bounded project-root selection.
- [x] Detect Node, Python/uv, Rust, and mixed/monorepo fixtures.
- [x] Detect manifests, package manager conflicts, scripts, Git branch/status without fetching, Compose files, environment key differences, and setup health.
- [x] Persist notes, checklists, tags, and last scan health.
- [x] Add exact command preview and allowlisted structured launch.
- [x] Implement the managed process supervisor, bounded redacted logs, lifecycle events, and graceful/force tree-stop workflow.
- [x] Keep external process termination separate and more strongly confirmed.

### Acceptance

A fixture development server can be registered, scanned, started with a displayed executable/argument vector, observed through redacted logs, associated with its discovered port, and stopped with its verified process tree.

## Phase 3 - Topology and Localhost Bridge

### Deliverables

- [x] Construct graph nodes and edges only from evidence.
- [x] Attach evidence and `certain`, `strong`, or `inferred` confidence to every edge.
- [x] Join project, managed/external process, parent process, port, URL, and relevant tool nodes.
- [x] Infer schemes conservatively and explain loopback, all-interface, and specific-interface bindings.
- [x] Generate local/private LAN URL candidates without claiming remote reachability.
- [x] Generate QR codes locally.
- [x] Create an isolated preview window with no privileged Mr Manager IPC and constrained navigation.

### Acceptance

Starting the fixture server creates a visible project -> process -> port -> URL chain with inspectable evidence. Preview content cannot invoke Mr Manager commands.

## Phase 4 - Docker and Compose Doctor

### Deliverables

- [x] Detect Docker absent, installed/stopped, inaccessible, and running states.
- [x] Enumerate containers, health, labels, resource usage, mappings, networks, and volumes using structured output.
- [x] Add strongly confirmed start/stop/restart actions and bounded redacted logs.
- [x] Discover and parse Compose through canonical Docker configuration when available.
- [x] Render Compose services, networks, volumes, dependencies, health, and mapped ports.
- [x] Implement deterministic doctor rules for conflicts, missing variables, risky mounts, readiness/health gaps, and association evidence.

### Acceptance

A synthetic Compose fixture renders deterministically and receives repeatable diagnostics. Docker absence never causes a crash or a fabricated state.

## Phase 5 - Local services, Ollama, WSL, and Network Throughput/Internet Diagnostics

### Deliverables

- [x] Implement a constrained detector registry with evidence types, timeouts, safe version parsing, and typed outcomes.
- [x] Detect relevant runtimes, editors, databases, Docker, WSL, and local services without arbitrary custom shell strings.
- [x] Detect Ollama through local process/executable/endpoints and its loopback API; list installed and running models.
- [x] Show WSL distributions and state through a safe adapter.
- [x] Collect adapters, routes, local addresses, Wi-Fi evidence, and private LAN candidates.
- [x] Report VPN as evidence-based `likely active` rather than certainty.
- [x] Add an explicit Network dashboard module that distinguishes local adapter throughput from external internet speed/diagnostic testing.
- [x] Collect local adapter throughput with zero external network requests by reading OS/network counters for current download rate, current upload rate, session bytes received, session bytes sent, peak throughput, and timeline samples.
- [x] Support per-adapter views and a combined total view, including link speed where available and Wi-Fi signal quality where available.
- [x] Show gateway reachability, DNS status, probable VPN state with evidence, LAN IP candidates for localhost sharing, and warnings when a local dev server is bound only to loopback instead of `0.0.0.0` or a LAN interface.
- [x] Where Windows exposes reliable counters, show per-process network usage mapped to processes such as browsers, editors, Docker Desktop, Ollama, VPN clients, package managers, and local dev servers.
- [x] If reliable per-process network throughput is unavailable, display `not available on this system` rather than estimating or guessing.
- [x] Generate QR codes and LAN links for local apps from local binding evidence without claiming another device can reach them.
- [x] Keep external internet diagnostics disabled by default behind a clearly labelled `Run Internet Test` action explaining that it contacts external servers and may consume data.
- [x] When explicitly run, support internet diagnostics where feasible for latency, DNS resolution, packet loss, download speed, upload speed, and route/VPN behaviour.
- [x] Label every network diagnostic result as `local-only` or `contacted internet`, and expose which external endpoints were contacted before and after a test.
- [x] Add privacy controls and safe opt-in diagnostics that redact SSIDs, MAC addresses, public IP addresses, remote endpoints, DNS servers, and network history from exports unless the user explicitly includes them.

### Acceptance

Every missing integration has a stable unavailable state. Present integrations show timestamped evidence, version/state when known, and no unsupported certainty. The Network dashboard works offline, local throughput uses OS counters only, clean launch makes no external request, internet testing only begins after explicit user action, unavailable per-process throughput is labeled honestly, and default exports redact sensitive network details.

## Phase 6 - System Diagnostics and recordings

### Deliverables

- [x] Add coordinated frequent metrics and a GPU provider abstraction with honest unsupported fields.
- [x] Build bounded responsive charts and virtualized ranked process views.
- [x] Implement recording sessions, event annotations, downsampling, retention, export, and deletion.
- [x] Add a Network Timeline Recorder that captures CPU, RAM, disk I/O, GPU, battery, active processes, adapter throughput, VPN state, Docker activity, and local dev-server ports over time.
- [x] Clearly distinguish local-only recorded samples from any explicitly triggered internet diagnostics included in a recording.
- [x] Correlate lifecycle/resource deltas deterministically and use non-causal language.
- [x] Add developer diagnostics for collector timing and dropped events.
- [x] Rename the active page, DTO, IPC, command, permission, and module surface to System Diagnostics while redirecting the legacy route.
- [x] Use a labelled responsive CPU/RAM recording chart consistent with the Network timeline.
- [x] Offer only Normal 2.5-second and Fast 1-second metric refresh modes, with a forward migration from legacy values.

### Acceptance

A recorded fixture build produces a bounded timeline with system, process, Docker, local-port, VPN, and adapter-throughput evidence, then identifies the largest process-resource delta without asserting causation. Default recording exports redact sensitive network details unless the user explicitly includes them.

## Phase 7 - Cleaner and quarantine

### Deliverables

- [x] Implement bounded, cancellable scanning only below explicit roots.
- [x] Reject dangerous roots and guard canonicalization, traversal, reparse points, mount/junction escape, parser size, file count, and depth.
- [x] Categorize deterministic build/cache candidates with reason, risk, confidence, size, file count, and regeneration instructions.
- [x] Create immutable reviewable cleanup plans.
- [x] Revalidate immediately before moving; quarantine atomically where practical.
- [x] Write complete manifests and support exact restore plus occupied-destination conflict handling.
- [x] Separate irreversible purge behind strong confirmation.
- [x] Add comprehensive fixture-only security and round-trip tests.

### Acceptance

A synthetic `node_modules` tree can be quarantined and restored byte-for-byte and path-for-path without affecting any source file.

## Phase 8 - Retired content capability

### Deliverables

- [x] Remove Content Studio, README Forge, and Portfolio Vault from navigation and routing.
- [x] Remove their frontend DTO/IPC surface and Tauri command permissions.
- [x] Remove active Rust content services and commands.
- [x] Remove unfinished AI, DeepSeek, MCP, and Obsidian dependencies, DTOs, and migration work.
- [x] Preserve schema-5 tables and existing application-data files without reading, mutating, or deleting them.
- [x] Redirect the retired `/portfolio` route to Projects.

### Acceptance

No Content Studio or agent capability is exposed through the webview or IPC, clean launch remains offline, and previously stored schema-5 content data is left untouched.

## Phase 9 - OS Lab

### Pre-Phase-9 usability and reliability pass

- [x] Move Cleaner scans/quarantine, project discovery, and Internet Test orchestration into typed Rust-owned background tasks with a global task center.
- [x] Make live development process -> port -> URL topology visible without requiring a registered-project association.
- [x] Add binding, protocol, ownership, common-service, project, managed-state, CPU, memory, and access filters without guessing unavailable owner data.
- [x] Return partial Docker inventory with specific starting, timeout, and permission diagnostics and serialize overlapping dashboard refreshes.
- [x] Add labelled Mbps/Gbps network charts and an explicit Internet Test preflight.
- [x] Explain quarantine retention and add separately confirmed whole-manifest purge.
- [x] Rename the transparency surface to Data access & evidence and explain confidence levels.

### Deliverables

- [ ] Keep the Real System Map distinct from educational simulation.
- [ ] Implement deterministic scheduling simulations with step controls and comparison metrics.
- [ ] Implement memory-allocation and page-replacement simulations.
- [ ] Implement deadlock/resource simulations.
- [ ] Label all simulated values and controls unambiguously.
- [ ] Add algorithm fixtures and correctness tests.

### Acceptance

Each simulator reproduces known fixture results, never mixes simulated values with live telemetry, and remains usable with keyboard and reduced motion.

## Mandatory post-Phase 9 verification gate

Only after Phases 0-9 meet their exit criteria:

- [ ] Run formatting, linting, type checking, frontend tests, Rust tests, Clippy, frontend production build, and a Windows Tauri build.
- [ ] Run fixture acceptance flows and a manual Windows smoke test.
- [ ] Build an unsigned internal executable with `npm run tauri build -- --no-bundle` (or document an equivalent exact command).
- [ ] Calculate and record a SHA-256 checksum.
- [ ] Record the uncommitted/generated artifact path and do not add the binary to source control.
- [ ] Clearly label the build internal, unsigned, and unsuitable for public distribution.
- [ ] Stop all implementation work and ask the user to inspect it.

## Phase 10 - Open-source release quality (blocked pending approval)

**Do not begin this phase until the user explicitly approves the manually reviewed Phase 9 executable.**

Planned only after approval:

- [ ] System tray and deliberate opt-in autostart.
- [ ] Notifications with privacy-conscious content.
- [ ] Signed-release updater configuration; never install updates without verified integrity.
- [ ] Windows installer and signed release pipeline.
- [ ] GitHub Actions release workflow.
- [ ] Public README, contribution guide, detector spec, cleaner spec, changelog, license confirmation, fixture screenshots, and demo script.
- [ ] Release provenance, hashes, signing, rollback, and disclosure process validation.

## Cross-cutting verification matrix

| Area             | Required evidence                                                                                                           |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Correctness      | Unit tests for pure parsers/rules; integration tests against synthetic fixtures                                             |
| Safety           | Destructive tests target temporary fixtures only; confirmations and revalidation are exercised                              |
| Security         | IPC isolation, traversal, injection, redaction, oversized input, ANSI, PID reuse, and race regressions                      |
| Privacy          | Clean-launch network observation; explicit internet-test gate; diagnostics/export preview, redaction, and deletion controls |
| Accessibility    | Keyboard operation, focus behavior, labels, non-color status, reduced motion                                                |
| Performance      | 1,000-row responsiveness, bounded logs/history, collector timing, cancellation                                              |
| Platform honesty | Unsupported/permission/partial states covered without fake values                                                           |
| Packaging        | Windows executable starts, version matches metadata, no generated binary is committed                                       |

## Major risks

| Risk                                   | Impact                                                          | Mitigation and proof                                                                                           |
| -------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Windows API/version differences        | Missing or inaccurate process/port/network data                 | Platform traits, capability probes, typed partial results, Windows fixture and hardware smoke tests            |
| Elevated/protected processes           | Incomplete details and unsafe termination assumptions           | No admin requirement, permission states, PID+start-time revalidation, explicit external-process confirmation   |
| Secret leakage                         | Credentials exposed in UI, logs, exports, or docs               | Central redaction, key-only environment inspection, bounded safe diagnostics, adversarial regression tests     |
| Filesystem traversal or reparse escape | Cleanup affects data outside selected root                      | Canonical boundaries, component-wise validation, reparse handling, immutable plan, pre-mutation revalidation   |
| Hostile local preview content          | Local page invokes privileged desktop actions                   | Separate zero-capability preview window, navigation policy, no inherited IPC                                   |
| Command injection                      | Arbitrary execution through a project/script value              | Exact executable and argument vectors, server-side resolution, allowlisted action types, injection fixtures    |
| PID reuse and collection races         | Wrong process association or termination                        | Stable process key, snapshot timestamps, identity recheck immediately before action                            |
| Collector load                         | Mr Manager becomes a resource problem                           | Coordinated refresh modes, cancellation, visibility throttling, deltas, bounded storage, diagnostics timings   |
| Heuristic overclaiming                 | User trusts a false relationship, VPN, URL, or correlation      | Evidence objects, confidence vocabulary, timestamps, conservative copy, no causal claims                       |
| Accidental external network request    | Local-first/privacy guarantee is broken                         | Clean-launch egress test, disabled-by-default internet diagnostics, endpoint preview, local-only result labels |
| Sensitive network data exposure        | SSIDs, MACs, public IPs, remote endpoints, DNS, or history leak | Default export redaction, explicit include toggles, no unnecessary persistence, adversarial export fixtures    |
| Unsigned artifact confusion            | Internal executable is mistaken for a release                   | No-bundle internal build, checksum, explicit warning, no publication, hard Phase 10 gate                       |
| Supply-chain/updater compromise        | Malicious release or dependency reaches users                   | Pinned dependencies, lockfiles, minimal plugins, deferred signed updater, provenance and release review        |

## Definition of done

The application is complete only when all specification acceptance criteria are backed by real behavior and passing checks. Phase completion, internal artifact creation, and public release readiness are three separate states. The Phase 9 executable proves only internal manual-review readiness; it does not authorize Phase 10 or public distribution.
