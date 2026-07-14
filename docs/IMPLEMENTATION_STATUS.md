# Mr Manager implementation status

Last updated: 2026-07-14 (Asia/Shanghai)

## Current state

**Active phase:** Retired Phase 8 content capability and pre-Phase-9 reliability work complete; Phase 9 next  
**Overall status:** Runnable real-data Windows command center with route-resilient tasks, automatic live-service topology, projects, Docker/integrations/network, recordings, and Cleaner quarantine/purge  
**Release gate:** Phase 10 remains blocked until Phases 0-9 pass, an unsigned internal Windows executable is manually reviewed, and the user explicitly approves continuation.
**Scope update:** Phase 5 now explicitly includes a Network Throughput and Internet Diagnostics module; Phase 6 now explicitly includes the Network Timeline Recorder.

Mr Manager now builds as a Tauri 2 Windows application backed by real standard-user system, process, disk, network, battery-availability, native TCP/UDP ownership data, explicit project registration, bounded project discovery, local manifest/Git/environment metadata, allowlisted managed project commands, an evidence-based topology/localhost bridge, Docker/Compose, constrained integrations, local-first networking, System Diagnostics recordings, and transactional Cleaner quarantine. Browser mode explicitly reports that desktop data is unavailable and contains no fixture metrics disguised as real evidence.

## Completed

- Tauri 2, React, strict TypeScript, Vite, Tailwind, TanStack Query, React Router, and pinned lockfiles.
- Narrow generated Tauri command permissions for the `main` window, strict CSP, and no generic shell/filesystem plugin.
- Typed camelCase Rust/TypeScript DTO contract and stable serializable application errors.
- Versioned SQLite migration, foreign keys, transactional local settings, and app-data storage.
- Rotating local logs with ANSI/control sanitization and secret redaction before persistence.
- Coordinated background collector for CPU, per-core CPU, RAM, swap, disks, network throughput, uptime, process metadata, PID-plus-start-time identity, and battery/AC availability.
- Native Windows IPv4/IPv6 TCP listener and UDP bound-endpoint enumeration with owning PIDs.
- Evidence-labelled binding scope and deliberately narrow URL-scheme inference.
- Explicit project folder selection and bounded project-root discovery with dangerous-root and reparse-point rejection.
- Node, Python/uv, Rust, mixed monorepo, conflicting lockfile, Compose-file, local Git, `.env` key-name, and local database-hint detection against synthetic fixtures.
- Persistent project registry fields for notes, tags, checklists, scan health, manifests, package manager evidence, scripts, Git summaries, environment metadata, and last scan time.
- Allowlisted managed command launch from detected scripts only, using exact executable/argument vectors without shell interpolation.
- Windows Job Object-backed managed process supervision, graceful `taskkill /T` stop request, confirmed force-stop, bounded redacted stdout/stderr/system logs, and live PID-to-port/URL association in the Projects screen.
- Shared Windows `CREATE_NO_WINDOW` construction for every captured non-interactive CLI probe and managed command, preventing Docker, Network, Git, WSL, integration, GPU, and stop operations from flashing terminal windows. A behavioral regression test launches a real child process and verifies that it has no console window.
- Windows GUI subsystem selection in both debug and release profiles, so directly launching either executable does not create an application console.
- Evidence topology graph joining projects, managed runs, process CWD associations, parent processes, Windows-owned ports, and inferred URL nodes with per-edge evidence and confidence.
- Live-service-first topology that automatically shows evidenced development process -> port -> URL chains even without a project association, plus project/all-evidence filters and confidence explanations.
- Rust-owned bounded in-session task registry and global task center for Cleaner scans/quarantine, project discovery, and explicit Internet Test runs across route navigation.
- Expanded process and port filters for project/managed/access/resource state, localhost/LAN binding, protocol, ownership, and common web/database development ports.
- Localhost bridge URL nodes with binding explanations, private LAN URL candidates when local evidence supports them, local QR codes, and an isolated preview window with no main-window command capability and same-origin navigation constraints.
- Docker status probing for CLI-missing, daemon-stopped, inaccessible, running, and error states, including Docker Desktop process evidence where available.
- Docker starting and partial-inventory states, serialized dashboard probes, longer bounded startup timeouts, retained prior UI data, and specific timeout/permission diagnostics.
- Docker inventory through exact structured Docker CLI arguments: containers, health, labels, resource usage, published ports, networks, mounts, volumes, and associated registered projects from Compose labels.
- Confirmed Docker container start/stop/restart actions with exact typed confirmation phrases; no image/container/network/volume deletion or prune action exists in the MVP.
- Bounded redacted Docker logs.
- Registered-project Compose discovery, canonical `docker compose config --format json` parsing when available, conservative fallback parsing otherwise, and a Compose visualizer for services, dependencies, networks, volumes, health checks, and mapped ports.
- Deterministic Compose Doctor rules for duplicate/conflicting ports, unresolved environment variables, missing bind mounts, dependency/health gaps, public database ports, floating `latest` tags, root users, missing restart policies, undefined networks/volumes, unhealthy containers, and missing runtime associations.
- Real-data Overview, virtualized process inspector, port ownership view, Projects, Topology, Docker, permissions center, refresh/privacy settings, inaccessible/unsupported states, and `Ctrl+K` command palette.
- Constrained detector registry for Git, VS Code, Docker/Compose, Ollama, WSL, Node/npm/pnpm/yarn/bun, Python/pip/uv/Poetry, Rust/Cargo, Go, Java, PostgreSQL, MySQL/MariaDB, Redis, MongoDB, local services, and common VPN clients using PATH/process/owned-port/loopback evidence and exact bounded version probes.
- Ollama status through local process/executable evidence and loopback API only, including version, installed models, running models, and unavailable API states without model pull/delete actions.
- Read-only WSL summary through exact `wsl.exe --list --verbose` with distro state/version parsing.
- Network dashboard with local adapter throughput, per-adapter and combined session totals, peaks, timelines, link speed where available, private LAN candidates, gateway reachability, DNS configured-count status, evidence-based probable VPN state, localhost-only dev-server warnings, LAN links/QR codes from binding evidence, and honest unavailable per-process throughput state.
- Explicit `Run Internet Test` diagnostics gated by Settings opt-in and click consent, with local-only/contacted-internet labels, endpoint disclosure, and conservative latency/DNS/packet-loss/download/route results. Upload speed remains unavailable unless a trusted upload endpoint is configured.
- System Diagnostics with coordinated frequent local metrics, responsive charts, virtualized ranked process views by CPU/memory/disk I/O, collector diagnostics, and Docker/local-port/network/VPN evidence.
- GPU provider abstraction with bounded exact `nvidia-smi.exe` telemetry when available and honest unsupported/error states otherwise; GPU fan, wattage, FPS, and missing vendor telemetry are never fabricated.
- Explicit metric recording sessions with an in-memory active ring buffer, SQLite persistence for completed sessions, event annotations, downsampling before persistence, redacted JSON export, delete controls, and local-only/internet-diagnostic flags in each sample.
- Network Timeline Recorder samples CPU, RAM, disk I/O, GPU provider state, battery, active top processes, adapter throughput, VPN state, Docker process evidence, and local dev-server ports over time.
- Deterministic non-causal “what changed?” analysis that identifies the largest observed process-resource delta without asserting causation.
- Bounded cancellable Cleaner scans for one to eight explicit non-overlapping roots, with dangerous-root rejection, canonical containment, fixed depth/file/candidate limits, and per-path issues.
- Deterministic build/cache candidates with category, reason, confidence, risk, size, file count, practical lock state, regeneration guidance, and metadata identity fingerprints; candidates are never preselected.
- Server-held scan evidence and persisted immutable cleanup plans that execute once and revalidate path, root, reparse ancestry, kind, size, count, and fingerprint immediately before mutation.
- App-managed quarantine with atomic same-volume moves, free-space-checked and verified cross-volume copy fallback, transactional durable manifests, and explicit partial-failure state.
- Exact restore with occupied-destination refusal and server-generated conflict-safe sibling restore, plus irreversible item-specific purge confirmation constrained to canonical quarantine storage.
- Dedicated Cleaner UI, command-palette/navigation entry, typed Tauri IPC/capabilities, safety documentation, and frontend review/confirmation coverage.
- Content Studio, README Forge, Portfolio Vault, and unfinished AI/DeepSeek/MCP/Obsidian work removed from active code, navigation, IPC, permissions, dependencies, and documentation by explicit user direction. Schema-5 content tables and existing application-data files remain untouched for data preservation.
- Product, package, crate, binary, active code, UI, documentation, and MM icon set renamed to Mr Manager while retaining the legacy Tauri identifier, SQLite filename, log filenames, and app-data location for compatibility.
- System Diagnostics replaces the former War Room across route, navigation, DTO, IPC, Rust module, command, permission, errors, and documentation; `/war-room` remains a redirect only.
- Normal 2.5-second and Fast 1-second metric refresh modes replace the three legacy modes through schema migration 6, which maps existing values without touching other settings or schema-5 content records.
- Network and recorded CPU/RAM timelines share a responsive labelled uPlot component with time/unit axes, grid, cursor legend, latest values, accessible summaries, and resize cleanup.
- Page-level introductory paragraphs and the Settings storage footnote removed while section-level safety, privacy, availability, and operational guidance remains.
- Locally generated Mr Manager icon set and branded Windows executable.
- Frontend/Rust unit tests, a live read-only Windows collector test, Playwright web smoke coverage, and GitHub Actions CI.
- Architecture, decision, threat-model, privacy, security, contribution, conduct, changelog, and current-status README documentation.

## Verification results

| Command / audit                                            | Current-worktree result                                                                                                                                                                 |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `npm run format` / `npm run format:check`                  | Pass; all supported frontend, configuration, and Markdown files are formatted.                                                                                                          |
| `npm run lint`                                             | Pass; zero warnings.                                                                                                                                                                    |
| `npm run typecheck`                                        | Pass; strict TypeScript.                                                                                                                                                                |
| `npm run test:run`                                         | Pass: 6 files, 22 tests, including Normal/Fast settings and responsive diagnostics-chart data/cleanup coverage.                                                                         |
| `npm run build`                                            | Pass; production frontend bundle built. Vite still reports the existing large-chunk warning.                                                                                            |
| `npm run test:e2e`                                         | Environment-blocked before test execution: the pinned Chromium binary was absent; its approved download stalled, and the installed Edge-channel fallback also stalled before progress.  |
| `cargo metadata --offline --locked --no-deps`              | Pass; the manifest/lockfile resolve with no AI, HTTP, MCP, schema-generation, or credential dependency.                                                                                 |
| Active-surface audit                                       | Pass; no retired branding, War Room command/module, Content Studio, README/portfolio permission, AI, DeepSeek, MCP, or Obsidian surface remains in active or generated code.            |
| Capability-permission audit                                | Pass; all 59 active custom allowlist entries match the 59 generated command-permission definitions exactly, with no missing or orphaned definition.                                     |
| Page-header and refresh-mode audit                         | Pass; no page-level header paragraph or retired Eco/Balanced/Realtime label remains in active UI/documentation.                                                                         |
| Child-process audit                                        | Pass; `Command::new` appears only in the shared hidden-process constructor, and a real child-process regression test confirms no console window is attached.                            |
| PE subsystem audit                                         | Pass; both current debug and release executables report Windows GUI subsystem `2`, not the console subsystem.                                                                           |
| `cargo fmt --all -- --check`                               | Pass.                                                                                                                                                                                   |
| `cargo check --all-targets --all-features --locked`        | Pass; the complete Rust target surface compiles.                                                                                                                                        |
| `cargo clippy --all-targets --all-features -- -D warnings` | Pass; zero warnings.                                                                                                                                                                    |
| `cargo test --all-features`                                | Pass: 63 tests, including schema-6 legacy refresh migration, live standard-user collection, and the real no-console child-process regression.                                           |
| `npm run tauri build -- --debug --no-bundle`               | Pass; unsigned debug artifact: `src-tauri/target/debug/mr-manager.exe`, SHA-256 `F6E454C77AC89C9EF3D33385CA8FF6AB163B1EFDB812CDB71AD119FFC8DB1B46`.                                     |
| `npm run tauri build -- --no-bundle`                       | Pass; unsigned release-profile artifact: `src-tauri/target/release/mr-manager.exe`, SHA-256 `8C1FDC118F05200E9C2BB35F200DAF000FD9C6C557B09CB31828C47B3027FD49`.                         |
| Manual GUI smoke                                           | Current executables are available; interactive repeated-refresh verification across Docker, Integrations, Network, and System Diagnostics remains for the user on the physical desktop. |

The initial `rusqlite 0.40.1` pin required an unstable compiler feature through `libsqlite3-sys 0.38.1`; it was replaced with the official compatible `rusqlite 0.39.0` / bundled `libsqlite3-sys 0.37.0` release so stable Rust 1.94 builds cleanly.

## Known limitations

- Phase 1 shows battery percentage and AC state when Windows exposes them; battery health and charge rate remain unavailable until a later provider can report them reliably.
- GPU telemetry depends on provider support. NVIDIA telemetry is read only through bounded `nvidia-smi.exe` when available; unsupported vendors/data remain explicitly unavailable.
- Process paths/command lines can be unavailable for protected processes; Mr Manager does not elevate itself to fill the gap.
- OS Lab belongs to Phase 9 and is not implemented yet.
- Docker lifecycle actions are implemented only for listed containers and require exact confirmation. Delete/prune actions are intentionally absent.
- Compose fallback parsing is conservative and fixture-tested; when Docker Compose is available Mr Manager prefers Docker's canonical resolved JSON config.
- Managed run history is still in memory for Phase 2; metric recording sessions are now durable only when explicitly started and stopped by the user.
- External process termination is intentionally not exposed in Phase 2. Only Mr Manager-managed runs can be stopped from the Projects page.
- Companion-device reachability checks are still not performed; QR/LAN links are generated from local binding evidence and never claim another device can actually reach the app.
- Phase 5 reports Wi-Fi signal quality as unavailable when the current Windows provider does not expose it reliably.
- Phase 5 reports per-process network throughput as unavailable rather than estimating from socket ownership. A future ETW-backed provider may add reliable per-process counters.
- Upload speed diagnostics remain unavailable until a trusted upload endpoint and data-size policy are configured.
- System Diagnostics records Docker activity as process evidence in Phase 6 to avoid repeated Docker CLI polling during high-frequency recording. Rich container lifecycle deltas can be added later.
- Retired schema-5 Content Studio records and files are preserved but no longer accessible through the application UI or IPC.
- The console-free release-profile executable is an unsigned pre-Phase-9 test artifact, not the mandatory post-Phase-9 review build or a signed public installer.
- In-session background tasks survive route navigation but do not survive a full app exit; Cleaner mutation recovery remains durable through SQLite manifests.
- The legacy Tauri identifier `com.desktopmanager.commandcenter`, database filename, and log filenames are intentionally retained so existing app data is not orphaned by the Mr Manager rename.

## Safety and privacy notes

- Collection is read-only and makes no external request.
- No user project directory has been scanned.
- No project command was launched during automated checks; the managed-command path is covered by structured launch/log redaction tests and the built executable.
- No Docker lifecycle action was executed during automated checks; Docker start/stop/restart is implemented behind typed confirmation only.
- No external internet diagnostic was executed during automated checks; clean launch, local network collection, and System Diagnostics recording do not contact internet endpoints.
- No external process, Docker object, network/firewall/VPN setting, Git state, or user file has been modified.
- Cleaner mutation tests used only temporary synthetic fixtures. No real project was scanned, quarantined, restored, or purged; automatic purge does not exist.
- Content retirement did not delete or modify any existing schema-5 record, README backup, portfolio asset, or other application-data file.
- Generated binaries, target directories, logs, databases, and machine snapshots remain outside version control.

## Exact next milestone

Implement Phase 9's clearly separated OS Lab: deterministic scheduling, memory-allocation, page-replacement, and deadlock/resource simulations with step controls, comparison metrics, unambiguous Simulation labels, algorithm fixtures, correctness tests, and a final unsigned internal executable/manual-review gate before Phase 10.
