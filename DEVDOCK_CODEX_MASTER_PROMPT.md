# MASTER CODEX PROMPT — Mr Manager LOCAL DEVELOPMENT COMMAND CENTER

You are Codex acting as the lead engineer, product architect, security reviewer, UX engineer, and release engineer for a serious open-source desktop application.

Build a production-quality application with the working name * Mr Manager**.
 Mr Manager is:

> One local desktop command center that understands and manages a developer's laptop: projects, processes, ports, localhost apps, Docker, local AI tools, system resources, network/VPN state, safe project cleanup, project documentation, portfolio assets, and an educational OS lab.

The application must be genuinely useful on a real Windows development laptop, not merely a visually attractive prototype. Its core features must use real local system data. It must work without cloud services, accounts, AI API keys, or telemetry.

The repository is intended to be public and open source. Build it as software that other developers could install, trust, understand, extend, and contribute to. This should be a .exe executable file

---

## 0. OPERATING MODE

Follow these instructions throughout the task.

1. **Inspect first.**
   - Inspect the repository before changing anything.
   - If it is empty, scaffold the project cleanly.
   - If code already exists, preserve working behavior and adapt this plan to the existing structure.

2. **Plan, then implement.**
   - Create `docs/IMPLEMENTATION_PLAN.md` with milestones, dependencies, major risks, and an implementation checklist.
   - Create `docs/IMPLEMENTATION_STATUS.md` and update it continuously.
   - Do not stop at planning. Begin implementation immediately after the plan.

3. **Work in vertical slices.**
   - Keep the application runnable after each milestone.
   - Finish one useful end-to-end workflow before starting five half-built modules.
   - Do not create fake dashboards with placeholder metrics.
   - Hide unfinished navigation items or mark them clearly as unavailable; never show fabricated data.

4. **Use current official documentation.**
   - Verify current stable APIs and package names before using them.
   - Prefer official Tauri, Rust, Microsoft Windows, Docker, Git, and Ollama documentation.
   - Use stable releases that are mutually compatible and pin them in lockfiles.

5. **Use sound engineering judgment.**
   - Do not ask me routine implementation-choice questions.
   - Resolve reasonable ambiguity yourself and record the decision in `docs/DECISIONS.md`.
   - Ask only when blocked by missing credentials, missing hardware, or an irreversible product decision.

6. **Actually verify the work.**
   - Run formatting, linting, type checking, tests, and builds after each major milestone.
   - Fix failures instead of merely reporting them.
   - Record commands and results in `docs/IMPLEMENTATION_STATUS.md`.

7. **Protect the user's machine.**
   - Never delete user files during development or tests.
   - Never recursively scan an entire drive without explicit user selection.
   - Never execute arbitrary shell strings from frontend input.
   - Never require administrator privileges for the normal application.
   - Never silently modify firewall rules, VPN settings, environment files, Git state, Docker volumes, or system startup configuration.

8. **Keep the public repository clean.**
   - Do not commit secrets, machine-specific absolute paths, generated binaries, local databases, logs, or captured system data.
   - Provide fixtures containing synthetic project structures for tests and screenshots.

---

## 1. PRODUCT MISSION
 Mr Manager should solve this real problem:

A developer has many projects, forgotten commands, local servers, ports, Docker containers, local databases, environment files, package managers, language runtimes, VPNs, local model servers, build artifacts, and hardware-performance issues. The operating system exposes these through separate tools that do not understand the relationships between them.
 Mr Manager should build a coherent local model of the laptop and answer questions such as:

- Which projects are currently active?
- Which process belongs to which project?
- What command launched that process?
- Which process owns port 5173?
- Which localhost service can I open in my browser?
- Is it reachable from my second laptop or phone on the LAN?
- Is it bound only to `127.0.0.1`?
- Which Docker container maps to that port?
- Which Compose project and service does that container belong to?
- Which database is running for this project?
- What changed immediately before CPU, RAM, GPU, or battery usage spiked?
- Is Ollama installed and running? Which models are installed and loaded?
- Is a VPN likely active, and what evidence supports that conclusion?
- Which build artifacts can be safely quarantined and restored later?
- What is wrong with this `compose.yaml`?
- Can Mr Manager produce a useful deterministic README and portfolio case study from project metadata?

The app must feel like a **developer mission-control product**, not a collection of unrelated utility screens.

The unifying concept is an evidence-based graph:

```text
Project
  ├── manifest and package manager
  ├── Git repository
  ├── runnable scripts
  ├── Mr Manager-managed process
  │      ├── child processes
  │      ├── logs
  │      └── listening port
  │              ├── localhost URL
  │              └── LAN URL
  ├── Docker Compose project
  │      ├── service
  │      ├── container
  │      ├── network
  │      ├── volume
  │      └── mapped port
  ├── local database
  ├── environment profile
  ├── README and documentation
  └── portfolio assets
```

Every relationship shown in the UI must carry evidence and, where inferred, a confidence level.

---

## 2. PRODUCT PRINCIPLES AND NON-NEGOTIABLES

### 2.1 Local-first and private

- No login.
- No mandatory internet connection.
- No analytics or telemetry by default.
- No cloud database.
- No API keys.
- No uploaded project files.
- All data is stored locally.
- Network-dependent features must be explicitly enabled and clearly labelled.
- The application must make zero external network requests on a clean first launch.

### 2.2 Read-only by default

Scanning and inspection are read-only.

Actions that alter state must require deliberate user interaction, including:

- terminating a process;
- stopping or restarting a container;
- replacing an environment file;
- quarantining files;
- purging quarantine;
- enabling autostart;
- applying a generated README;
- running a project script.

Show the exact target, effect, and command or operation before execution.

### 2.3 Safe rather than magical

- Prefer accurate “unsupported” states over guesses.
- Label inferred relationships as inferred.
- Do not claim that a LAN URL is reachable from another device merely because it works locally.
- Do not call a network adapter a VPN solely because its name contains “VPN”; show the evidence and say “likely active.”
- Do not claim causation when correlating a process start with a performance spike.
- Do not expose secret values in UI, logs, exports, screenshots, crash reports, or test snapshots.

### 2.4 Windows-first, cross-platform by architecture

The first production target is **Windows 11**, because that is the primary user's machine.

Implement Windows fully first. Define platform traits/interfaces so Linux and macOS support can be added later without rewriting product logic.

Unsupported platform features must return typed `Unsupported` results and produce graceful UI states—not panics, fake values, or `todo!()` crashes.

### 2.5 Useful without AI

The complete core product must work with no LLM.

An optional local-only assistant using an already-running Ollama model may be considered later, but it must never be required for detection, management, safety checks, README generation, diagnostics, or reports.

---

## 3. SCOPE BOUNDARIES

Build an ambitious application, but maintain disciplined boundaries.

### Include

- Local project discovery and registry.
- Process inspection and process-to-project association.
- Parent/child process relationships.
- Listening TCP/UDP port ownership.
- Localhost and LAN URL generation.
- Managed development-process start/stop and logs.
- Docker and Docker Compose visibility and safe lifecycle actions.
- Docker Compose visualizer and doctor.
- Local app/tool discovery, including Ollama.
- System performance dashboard and recording sessions.
- Network adapter, Wi-Fi quality, local address, route, and probable VPN visibility.
- Safe build-artifact scanner with quarantine and restoration.
- Deterministic README generator.
- Local portfolio asset and case-study manager.
- Real live process/system map.
- Separate educational OS-algorithm simulator.
- Tray mode, optional autostart, notifications, updates, and packaging when the core is stable.

### Do not include in the initial production scope

- Remote control of another computer.
- Opening inbound internet tunnels.
- Remote port forwarding.
- Automatic router configuration.
- Automatic Windows firewall changes.
- Antivirus or malware-removal claims.
- Kernel drivers.
- Process injection.
- Packet interception.
- Credential capture.
- Arbitrary command execution from a browser-like frontend.
- Cloud sync.
- A full terminal emulator.
- A Docker Desktop replacement.
- A complete database GUI.
- A full IDE.
- Automatic dependency upgrades.
- Destructive Docker image, volume, or network pruning in the MVP.
- Git push, reset, checkout, merge, rebase, or other mutating Git operations in the MVP.

---

## 4. TECHNICAL STACK

Use current stable compatible versions at implementation time and commit all lockfiles.

### Desktop shell

- **Tauri 2**.
- Rust owns all privileged/system operations.
- The frontend must never receive broad shell or filesystem access.
- Use Tauri capabilities and scopes with least privilege.
- Use separate capabilities per window/webview.
- The localhost preview webview must have no access to application IPC.

### Frontend (idk which are required, but make a .exe file and make it look good as well)

- React.
- TypeScript in strict mode.
- Vite.
- Tailwind CSS.
- Radix primitives or shadcn-style components.
- React Router.
- TanStack Query for backend state, caching, refresh, and error handling.
- A minimal local UI store only for transient interface state.
- `@xyflow/react` or the current maintained React Flow package for topology graphs.
- A performant time-series chart library suitable for frequent metric updates.
- A virtualized table/list implementation for hundreds or thousands of processes/files.
- A local QR-code library with no external requests.
- Vitest and React Testing Library.
- Playwright for frontend/web-mode smoke tests where practical.

### Rust backend

Prefer a compact dependency set. Likely components include:

- `tauri` and required official plugins;
- `serde` / `serde_json`;
- `tokio`;
- `tracing` and rotating local logs with redaction;
- `thiserror` for typed domain errors;
- `anyhow` only at top-level application boundaries where appropriate;
- `sysinfo` for portable system/process information where accurate;
- the `windows` crate for Windows APIs not adequately exposed cross-platform;
- `rusqlite` with migrations for local state;
- `reqwest` configured for local health checks and optional explicit network tests;
- `walkdir` or `ignore` for bounded filesystem traversal;
- parsers for JSON, TOML, YAML, and dotenv syntax;
- a safe URL parser;
- cryptographic hashes only where genuinely necessary;
- an OS keychain/keyring library only if future features store secrets.

Do not add a Node sidecar for operations Rust can perform directly.

### Tauri plugins

Use only the plugins that are needed, with narrow permissions. Likely:

- dialog;
- opener;
- window-state;
- notification;
- updater later;
- autostart only after the user explicitly enables it.

Avoid exposing a generic shell plugin to the main frontend. If a sidecar becomes necessary later, grant only exact executable and argument scopes.

### Local persistence

Use SQLite for:

- registered projects;
- project notes/checklists/tags;
- detected integrations and last-known status;
- process-run metadata owned by Mr Manager;
- metric recording sessions and downsampled samples;
- cleanup plans and quarantine manifests;
- portfolio records;
- settings and feature flags.

Use app-data directories for:

- the SQLite database;
- logs;
- cached icons/thumbnails;
- quarantine manifests;
- optional managed portfolio assets;
- exported reports.

Do not store raw `.env` values or secrets in SQLite.

---

## 5. REPOSITORY STRUCTURE

Use a clear modular structure. Adapt if the repository already has conventions.

Suggested structure:

```text
/
├─ src/                         # React frontend
│  ├─ app/
│  ├─ routes/
│  ├─ components/
│  ├─ features/
│  │  ├─ dashboard/
│  │  ├─ projects/
│  │  ├─ topology/
│  │  ├─ processes/
│  │  ├─ localhost/
│  │  ├─ docker/
│  │  ├─ integrations/
│  │  ├─ system/
│  │  ├─ network/
│  │  ├─ cleaner/
│  │  ├─ readme-forge/
│  │  ├─ portfolio/
│  │  ├─ os-lab/
│  │  └─ settings/
│  ├─ lib/
│  │  ├─ ipc/
│  │  ├─ query/
│  │  ├─ formatting/
│  │  └─ validation/
│  └─ types/
├─ src-tauri/
│  ├─ capabilities/
│  ├─ migrations/
│  └─ src/
│     ├─ app.rs
│     ├─ commands/
│     ├─ domain/
│     ├─ db/
│     ├─ events/
│     ├─ platform/
│     │  ├─ mod.rs
│     │  ├─ windows/
│     │  ├─ linux/
│     │  └─ macos/
│     ├─ collectors/
│     │  ├─ system.rs
│     │  ├─ processes.rs
│     │  ├─ ports.rs
│     │  ├─ network.rs
│     │  ├─ battery.rs
│     │  └─ gpu.rs
│     ├─ projects/
│     │  ├─ discovery.rs
│     │  ├─ manifests.rs
│     │  ├─ scripts.rs
│     │  ├─ git.rs
│     │  ├─ environment.rs
│     │  └─ doctor.rs
│     ├─ supervisor/
│     ├─ docker/
│     ├─ integrations/
│     │  ├─ registry.rs
│     │  ├─ ollama.rs
│     │  ├─ docker.rs
│     │  ├─ vscode.rs
│     │  ├─ runtimes.rs
│     │  ├─ databases.rs
│     │  ├─ vpn.rs
│     │  └─ wsl.rs
│     ├─ topology/
│     ├─ metrics/
│     ├─ cleaner/
│     ├─ readme/
│     ├─ portfolio/
│     ├─ security/
│     └─ lib.rs
├─ fixtures/
│  ├─ projects/
│  ├─ compose/
│  ├─ git-output/
│  ├─ ollama/
│  └─ cleaner/
├─ docs/
├─ scripts/
├─ .github/
├─ AGENTS.md
├─ CONTRIBUTING.md
├─ SECURITY.md
├─ PRIVACY.md
├─ CODE_OF_CONDUCT.md
├─ CHANGELOG.md
├─ LICENSE
└─ README.md
```

Keep domain logic independent from Tauri commands where possible so it is testable and reusable by a future CLI.

---

## 6. CORE DOMAIN MODEL

Create typed Rust domain objects and matching TypeScript DTOs. Do not pass arbitrary JSON blobs between frontend and backend.

At minimum, model these concepts.

### Project

```text
Project
- id
- name
- root_path
- canonical_root_path
- tags
- notes
- checklist
- detected_stacks
- manifests
- package_manager
- scripts
- git_summary
- compose_files
- environment_files
- local_database_hints
- last_scanned_at
- scan_health
```

### Process identity

PIDs are reused. A stable process identity should include at least:

```text
ProcessKey
- pid
- start_time
```

### Process snapshot

```text
ProcessSnapshot
- key
- parent_key or parent_pid
- name
- executable_path when accessible
- cwd when accessible
- command_line_redacted
- status
- user when accessible
- cpu_percent
- memory_bytes
- disk_read_bytes
- disk_write_bytes
- start_time
- is_elevated_or_protected if knowable
- project_association
- managed_by Mr Manager
- listening_port_count
```

### Port endpoint

```text
PortEndpoint
- protocol
- local_address
- local_port
- remote_address only in explicitly enabled connection view
- remote_port only in explicitly enabled connection view
- state
- owning_process_key
- binding_scope: loopback | all_interfaces | specific_interface
- inferred_scheme
- local_url
- lan_urls
- reachability_state
```

### Integration

```text
IntegrationStatus
- detector_id
- display_name
- category
- installed_state
- running_state
- version
- executable_paths
- process_keys
- endpoints
- capabilities
- evidence
- last_checked_at
- errors
```

### Docker objects

Model engines, Compose projects, services, containers, networks, volumes, port mappings, health state, resource usage, labels, and project associations.

### Topology graph

```text
TopologyNode
- id
- kind
- label
- status
- metadata
- supported_actions

TopologyEdge
- id
- source
- target
- relation
- evidence
- confidence: certain | strong | inferred
```

Do not create edges without evidence. Examples:

- parent process edge: certain;
- PID owns listening port: certain;
- container maps a port: certain;
- process CWD lies inside project root: strong;
- command line references project root: strong;
- common port implies a database type: inferred;
- VPN based only on adapter naming: inferred.

### Cleaner models

Model candidate, reason, category, confidence, risk level, estimated size, file count, lock state, selected state, regeneration instructions, cleanup plan, quarantine manifest, restoration state, and purge state.

### Metric session

Model system samples, process deltas, annotations, thresholds, session metadata, downsampling level, and retention.

### Portfolio item

Model project link, asset references, screenshots, videos, demo links, repository link, technology tags, achievements, metrics, case-study fields, exports, and last updated time.

---

## 7. APPLICATION INFORMATION ARCHITECTURE

Use one coherent desktop layout.

### Persistent chrome

- Left navigation rail.
- Top status strip.
- Global command palette on `Ctrl+K`.
- Global search across projects, processes, ports, containers, integrations, and commands.
- Compact notification/activity panel.
- Connection/privacy indicator showing whether any external-network feature is enabled.

### Navigation

1. **Overview**
2. **Topology**
3. **Projects**
4. **Processes & Ports**
5. **Localhost Bridge**
6. **Docker**
7. **Local Services**
8. **System Diagnostics**
9. **Network & VPN**
10. **Cleaner**
11. **Portfolio Vault**
12. **OS Lab**
13. **Settings & Permissions**

README Forge and Compose Doctor should also appear contextually inside each project rather than only as isolated global tools.

### Overview page

Show a concise operational summary:

- active Mr Manager-managed projects;
- running local servers;
- listening development ports;
- Docker state and active containers;
- Ollama state and loaded models;
- CPU, RAM, GPU, disk, battery, and Wi-Fi summary;
- likely VPN state with evidence link;
- cleanup space available from the last scan;
- recent warnings and failed starts;
- quick actions.

Do not overload the landing page with every chart.

### Project detail page

Tabs:

- Overview
- Run
- Git
- Services
- Docker
- Environment
- Doctor
- Logs
- Docs
- Assets
- Notes & Checklist

### Topology page

This is the flagship visual experience.

The graph must support:

- zoom, pan, fit, focus, and minimap;
- filters by node and edge type;
- grouping by project;
- grouping by Compose project;
- search and focus;
- status coloring;
- a selected-node inspector;
- a selected-edge evidence panel;
- quick actions from nodes;
- a timeline toggle to show current or recorded-session state later.

The graph should link real entities such as:

```text
VS Code
  -> opened project
Project
  -> launched pnpm dev
pnpm
  -> spawned node
node
  -> listens on 0.0.0.0:5173
port 5173
  -> local URL
port 5173
  -> LAN URL
Project
  -> compose service api
service api
  -> container
container
  -> port 8000
container
  -> postgres network
```

When a relationship is inferred, expose the reason and allow the user to correct or dismiss the association.

---

## 8. FEATURE REQUIREMENTS

## 8.1 First-run onboarding

Create a short, transparent onboarding flow.

Steps:

1. Explain that Mr Manager is local-only and read-only by default.
2. Ask the user to choose one or more project roots rather than scanning the whole computer.
3. Offer a quick environment probe for developer tools.
4. Explain permissions and unsupported data.
5. Let the user choose metric refresh mode:
   - Normal — 2.5 second updates
   - Fast — 1 second updates
6. Keep external network checks off by default.
7. Finish on a populated Overview page.

The app must remain useful if the user skips project discovery.

---

## 8.2 Project registry and discovery

Allow users to:

- add one project folder;
- add a root folder and discover projects beneath it;
- remove a project from Mr Manager without deleting files;
- rescan manually;
- assign tags;
- pin favorites;
- archive projects locally;
- add notes and checklists.

### Discovery rules

Use bounded traversal:

- user-selected roots only;
- configurable maximum depth;
- skip common dependency/build/cache folders;
- do not follow symlinks or Windows reparse points by default;
- support cancellation and progress;
- avoid opening huge files;
- limit manifest size before parsing.

Recognize projects by combinations of:

- `.git`;
- `package.json`;
- `pyproject.toml`;
- `requirements.txt`;
- `uv.lock`;
- `poetry.lock`;
- `Cargo.toml`;
- `go.mod`;
- `.sln` / `.csproj`;
- `pom.xml` / `build.gradle`;
- `compose.yaml`, `compose.yml`, and legacy Docker Compose filenames;
- common framework markers.

### Package-manager detection

For Node projects, prefer:

1. `packageManager` metadata when valid;
2. lockfiles;
3. user override;
4. sensible fallback.

Detect conflicting lockfiles and report them.

Recognize npm, pnpm, yarn, and bun.

For Python, detect uv, pip, Poetry, Pipenv, and virtual environments without assuming one is active.

For Rust, detect Cargo workspaces and package scripts/aliases where practical.

### Script detection

Parse scripts and expose only known structured commands.

Examples:

- Node `package.json` scripts;
- Cargo run/build/test/check/clippy/fmt presets;
- uv/Python commands from project configuration;
- Docker Compose profiles and services;
- user-defined custom commands stored in Mr Manager.

A user-defined command must be stored as an executable plus argument array and working directory—not a concatenated shell string.

Before first execution, show:

- executable;
- arguments;
- working directory;
- environment profile;
- expected port if configured.

### Git summary

Use stable machine-readable Git output.

Show:

- repository state;
- current branch or detached HEAD;
- ahead/behind when known locally;
- staged, modified, deleted, renamed, conflicted, and untracked counts;
- last local commit;
- remote names without exposing credentials;
- worktree status;
- whether Git is unavailable.

Do not fetch remotes automatically.

Optional later integration may use an already-authenticated GitHub CLI after explicit opt-in to show pull-request and issue summaries. The base app must not require GitHub.

### Environment files

Detect common files such as `.env`, `.env.local`, `.env.development`, and `.env.example`.

Rules:

- Show file existence and metadata.
- Never display values by default.
- Compare key names between example and active profiles without storing values.
- Redact values from logs and diagnostics.
- Environment profiles should inject variables into launched processes from selected files.
- Prefer not to overwrite `.env`.
- If copying a profile to `.env` is offered, preview the operation, create a backup, and require confirmation.

### Broken setup detector

Implement deterministic checks such as:

- required package manager missing;
- runtime missing;
- conflicting lockfiles;
- missing dependencies directory when needed;
- missing keys compared with `.env.example`;
- occupied expected port;
- invalid or missing Compose file reference;
- missing bind-mount path;
- missing configured executable;
- stale project path;
- failed last start;
- build command unavailable;
- project path inaccessible.

Report actionable fixes. Do not execute fixes automatically.

### Dependency freshness

Dependency update checks are network-dependent and therefore disabled by default.

When explicitly enabled:

- use package-manager-native structured output where possible;
- never update automatically;
- cache results;
- clearly show that external registries were contacted.

---

## 8.3 Managed process supervisor and logs
 Mr Manager must safely launch and supervise project commands.

### Required behavior

- Launch an executable with an argument array and chosen working directory.
- Apply an environment profile only to that process tree.
- Capture stdout and stderr separately while preserving timestamps.
- Parse ANSI color safely.
- Display searchable, pausable, bounded logs.
- Save logs only when the user requests it or when local retention is enabled.
- Track process identity using PID plus start time.
- Associate child processes.
- Detect startup failure and exit code.
- Stop gracefully first.
- Offer force termination only after graceful stop fails and after a second confirmation.

On Windows, ensure stopping a Mr Manager-managed development server stops its child process tree. Use a safe native process-group or Job Object strategy where possible. Do not rely solely on killing the immediate parent PID.

### External processes

Processes not launched by Mr Manager are read-only by default.

The user may explicitly request termination from the process inspector. Show a stronger warning for:

- protected/system processes;
- processes running elevated;
- processes outside the current user;
- Docker/WSL/system services;
- processes with many dependent children.

Never provide a one-click “kill all unknown processes” feature.

### Log redaction

Redact likely secrets such as:

- bearer tokens;
- API keys;
- passwords in URLs;
- common secret environment-variable values known to Mr Manager's launched process;
- authorization headers;
- private keys.

Do not claim perfect secret detection. Explain the limitation.

---

## 8.4 Processes, ports, and relationship mapping

### Process table

Show:

- name;
- PID;
- parent PID;
- CPU;
- memory;
- start time;
- executable path when accessible;
- CWD when accessible;
- project association;
- number of listening ports;
- container association where known;
- Mr Manager-managed state;
- accessibility/permission state.

Provide filters for project, CPU, memory, listening state, managed state, user process, and system process.

### Port ownership

On Windows, use native ownership-aware TCP/UDP endpoint APIs where practical. Map endpoints to process identities.

Show:

- protocol;
- address;
- port;
- state;
- owning process;
- binding scope;
- guessed service/scheme with low-confidence label;
- local URL when appropriate;
- LAN URL candidates;
- project/container relationships.

### Local process-to-process links

Where both ends of a local connection are visible on the same machine, Mr Manager may create a connection edge. Treat this as evidence-based and account for short-lived connections.

Remote endpoints are privacy-sensitive. Keep the remote-connections view disabled by default. When enabled, provide redaction and export controls.

### Process-to-project association

Association evidence may include:

- process CWD lies within the project root;
- executable or command-line argument references the project root;
- process was launched by Mr Manager for the project;
- parent process is already associated;
- Docker Compose labels map the container to a project working directory.

Expose confidence and allow manual correction.

---

## 8.5 Localhost Bridge and preview hub

Detect probable local web services from listening ports and known process/container metadata.

For each endpoint, show:

```text
Local URL:     http://localhost:5173
Loopback URL:  http://127.0.0.1:5173
LAN URL:       http://192.168.x.x:5173
Binding:       0.0.0.0
Owner:         node.exe
Project:       Portfolio
```

### Reachability logic

- If bound to `127.0.0.1` or `::1`, state that it is not exposed to the LAN.
- If bound to all interfaces or a LAN adapter, generate candidate LAN URLs.
- Test the candidate from the same machine and label that only as a local self-test.
- Never claim another device can reach it until an optional companion check exists.
- Show possible Windows firewall blockers as diagnostics, not certainty.
- Do not change firewall rules.

### QR codes

Generate QR codes locally for LAN URLs.

### Preview

Implement preview safely.

Preferred design:

- open a separate isolated Tauri webview window for a selected loopback/LAN URL;
- grant that preview window no Mr Manager IPC capabilities;
- restrict navigation to the selected local origin and explicitly opened links;
- prevent the loaded local page from invoking privileged commands.

Add an optional screenshot-capture adapter later. Do not make a browser runtime download a requirement for the MVP.

### Responsive preview

Offer preset window sizes for desktop, laptop, tablet, and phone. Clearly state that this is viewport testing, not a full browser-device emulator.

---

## 8.6 Local services and tool inventory

Build an extensible detector framework.

### Detector evidence sources

A detector may use safe read-only evidence such as:

- process names;
- executable paths;
- PATH lookup;
- known installation directories;
- Windows installed-application registry entries;
- known localhost ports;
- local health endpoints;
- a safe allowlisted version command;
- Docker container labels/images;
- WSL distribution state.

Do not scan arbitrary ports across the LAN.

### Built-in detectors

Start with useful developer tools:

- Git;
- VS Code;
- Docker Desktop / Docker Engine / Docker Compose;
- Ollama;
- WSL;
- Node.js;
- npm;
- pnpm;
- yarn;
- bun;
- Python;
- `py` launcher;
- pip;
- uv;
- Poetry;
- Rust / Cargo;
- Go;
- Java where present;
- PostgreSQL;
- MySQL/MariaDB;
- Redis;
- MongoDB;
- common local AI applications or servers where reliably detectable;
- configurable VPN clients and adapters.

Build a declarative detector schema for read-only custom integrations. Custom detector definitions should not be able to execute arbitrary commands in the first version.

### Integration cards

Show:

- installed/not found;
- running/stopped/unknown;
- version;
- executable location;
- processes;
- endpoints;
- project/container relationships;
- safe actions;
- evidence and last probe time.

### Ollama integration

When a local Ollama endpoint is available, show:

- version;
- installed models;
- currently loaded models;
- size and available metadata;
- memory/VRAM metadata when provided;
- local endpoint health;
- associated Ollama process;
- links to the process and GPU view.

Use only the local Ollama API by default.

Do not automatically pull or delete models. Later, model pull may be added with explicit confirmation and streamed progress.

### WSL integration

Show read-only state such as:

- WSL availability;
- installed distributions;
- running state;
- version mode when available.

Do not automatically modify WSL configuration.

---

## 8.7 Docker command center

The Docker page should be useful whether Docker is absent, stopped, or running.

### Engine state

Show:

- CLI detected;
- daemon reachable;
- client/server version when accessible;
- engine context;
- Docker Desktop process state;
- errors with actionable explanations.

### Containers

Show:

- container name and ID;
- image;
- state and health;
- uptime;
- CPU/memory when available;
- ports;
- networks;
- volumes/mounts;
- labels;
- Compose project/service;
- associated Mr Manager project;
- logs.

Safe actions:

- start;
- stop;
- restart;
- open logs;
- inspect;
- open mapped localhost URL.

Do not provide deletion/prune actions in the MVP.

### Compose discovery

Discover Compose files within registered projects.

Prefer Docker's canonical resolved Compose model when Docker Compose is available. Fall back to a safe parser and clearly label unresolved interpolation.

### Compose visualizer

Graph:

- services;
- `depends_on` relationships;
- networks;
- volumes;
- host ports;
- health checks;
- profiles;
- container state.

### Compose Doctor

Build a deterministic rule engine with severities, evidence, explanation, and suggested remediation.

Initial rules:

- invalid Compose syntax;
- unresolved environment variables;
- missing referenced bind-mount paths;
- duplicate host ports;
- host-port conflict with a non-Docker process;
- service dependency absent;
- `depends_on` target without a health check when startup readiness matters;
- database port exposed to all interfaces;
- use of floating `latest` image tag;
- container configured to run as root when inspect data confirms it;
- missing health check for common long-running server/database images as an informational warning;
- missing restart policy as an informational suggestion where appropriate;
- network or volume referenced but undefined;
- container currently unhealthy;
- project service expected but not running.

Do not turn style preferences into critical warnings.

Support a future CLI output, but first expose results in the desktop UI and JSON export.

---

## 8.8 System Diagnostics

Create a polished, efficient real-time system monitor focused on development workflows.

### System metrics

Show when supported:

- total and per-core CPU;
- RAM used/available;
- swap/pagefile;
- disk usage and I/O;
- network throughput;
- GPU utilization;
- VRAM utilization;
- GPU temperature;
- battery percentage;
- AC/battery state;
- charge/discharge rate;
- battery health information when reliably available;
- top processes by CPU, memory, I/O, and GPU where supported.

Never fabricate fan speed, temperature, wattage, FPS, or battery-health data. Display “not available on this hardware/API” when necessary.

### GPU adapters

Implement a provider abstraction:

1. NVIDIA provider using NVML or a safe structured `nvidia-smi` invocation when available.
2. Windows generic GPU counters/provider where practical.
3. Unsupported state for unavailable vendors/data.

Do not hard-fail if no discrete GPU exists.

### Refresh modes

Implement configurable polling:

- Normal: default 2.5 second updates;
- Fast: 1 second updates with a warning about overhead.

Suspend or reduce hidden-window polling where appropriate.

### Recording mode

A user can:

1. name a session;
2. start recording;
3. use a game, IDE, build, model, or app;
4. stop recording;
5. inspect a timeline.

Record:

- system metric samples;
- top-process snapshots/deltas;
- process start/stop events;
- Mr Manager-managed command starts/exits;
- container lifecycle changes;
- AC/battery changes;
- VPN/default-route changes;
- user annotations.

### “What changed?” analysis

Create deterministic correlation analysis.

Example output:

```text
At 14:32:18 CPU exceeded 85%.
Within the preceding 20 seconds:
- node.exe PID 18420 increased from 8% to 57% CPU.
- a Vite build began for Portfolio.
- Docker container postgres-1 remained stable.

This is a time correlation, not proof of causation.
```

Allow the user to click directly into the process, project, command logs, or container.

### Retention

- Use an in-memory ring buffer for live display.
- Persist only explicit recording sessions or user-enabled history.
- Downsample old samples.
- Provide delete/export controls.

---

## 8.9 Network and VPN dashboard

### Network adapters

Show:

- adapter display name;
- type;
- up/down state;
- IPv4 and IPv6 addresses;
- gateway/default route evidence;
- DNS servers when available;
- link speed when available;
- Wi-Fi SSID only on screen and only when permitted;
- Wi-Fi signal quality;
- interface metric;
- local LAN address candidates.

Protect privacy in exports by redacting SSID, MAC addresses, public IP, and remote endpoints unless explicitly included.

### VPN detection

Create an evidence-based probable VPN state using combinations of:

- known VPN client process running;
- active tunnel/virtual adapter;
- adapter name/description patterns;
- default route through that adapter;
- DNS changes;
- user-defined detector.

Display:

```text
Likely active: Astrill
Evidence:
- Astrill process is running.
- A virtual adapter is up.
- Default route currently prefers that adapter.
Confidence: strong
```

Do not silently query a public “what is my IP” service. Public-IP and leak tests must be opt-in and list the external endpoints contacted before running.

### Diagnostics

Provide safe read-only diagnostics:

- DNS resolution test;
- local gateway reachability;
- configured endpoint ping/HTTP test;
- LAN URL self-test;
- port binding explanation;
- probable firewall issue guidance.

Do not modify adapter, DNS, route, VPN, or firewall settings.

---

## 8.10 Safe project cleaner

This feature must be designed as a safety-critical subsystem.

### User flow

1. User explicitly selects one or more folders.
2. Mr Manager scans only those roots.
3. It categorizes candidates.
4. It estimates recoverable space.
5. The user reviews and selects items.
6. Mr Manager creates an immutable cleanup plan summary.
7. The user confirms.
8. Mr Manager moves items to an app-managed quarantine.
9. The user can restore them.
10. Purge is a separate later action with a stronger confirmation.

### Initial candidate rules

#### Usually regenerable, still reviewable

- `node_modules`;
- `.next`;
- `.nuxt`;
- `.svelte-kit`;
- `.vite`;
- `.turbo`;
- `.parcel-cache`;
- `__pycache__`;
- `.pytest_cache`;
- `.mypy_cache`;
- `.ruff_cache`;
- `.tox`;
- `.nox`;
- Rust `target`;
- framework build caches.

#### Review required

- `dist`;
- `build`;
- `out`;
- `.venv` / `venv`;
- generated logs;
- downloaded artifacts;
- large files;
- old archives;
- duplicate project copies;
- language-specific build directories that may contain manually copied artifacts.

#### Never auto-select

- `.git`;
- source files;
- `.env*`;
- databases;
- user documents;
- images/videos used as project assets;
- SSH keys;
- credentials;
- operating-system directories;
- `Program Files`;
- Windows directories;
- user profile app-data roots unless explicitly selected and supported by a dedicated rule.

### Scanner safety

- Canonicalize every selected root.
- Reject system roots and dangerous paths by default.
- Do not follow symlinks, junctions, or reparse points.
- Do not cross filesystem boundaries unexpectedly.
- Support cancellation.
- Avoid loading file content.
- Bound memory usage.
- Detect locked/in-use paths where practical.
- Show errors per item rather than failing the entire scan.
- Never run a generated deletion command.

### Quarantine design

Each quarantine operation must create a manifest containing:

- plan ID;
- original canonical path;
- quarantine path;
- size and file count;
- category and reason;
- timestamps;
- project association;
- move/copy verification state;
- restore state;
- purge eligibility.

Prefer atomic move on the same volume.

If cross-volume copy is required:

1. check free space;
2. copy;
3. verify metadata/size sufficiently;
4. only then remove the original;
5. record partial-failure state safely.

Default retention: configurable, with no automatic purge until explicitly enabled.

### Restore

- Restore to the exact original path when possible.
- If the path is occupied, offer conflict-safe alternatives.
- Never overwrite newer content silently.
- Verify restoration and update the manifest.

### Duplicate repos

Treat duplicate-repository detection as a later opt-in rule.

Possible evidence:

- same normalized Git remote;
- same HEAD commit;
- similar root structure;
- last-modified date.

Never automatically quarantine a duplicate repo solely based on folder name.

---

## 8.11 README Forge

Build a deterministic, template-driven README generator with no AI dependency.

### Inputs

- project name;
- description entered by the user;
- detected languages/frameworks;
- package manager;
- scripts;
- install and run commands;
- environment key names;
- Docker/Compose usage;
- project structure;
- screenshots/assets;
- test/lint/build commands;
- license;
- contribution information;
- deployment notes entered by the user.

### Outputs

Generate a preview containing suitable sections such as:

- title and summary;
- features;
- screenshots;
- architecture;
- technology stack;
- prerequisites;
- installation;
- configuration;
- running locally;
- available scripts;
- Docker instructions;
- tests;
- project structure;
- roadmap;
- contributing;
- security;
- license.

### Safety

- Never overwrite `README.md` immediately.
- Show a rendered preview and text diff.
- Offer copy, export, or apply.
- Applying creates a timestamped backup.
- Do not include secret values.
- Do not claim detected commands are verified unless Mr Manager has successfully run them.
- Let users edit templates locally.

Optionally generate `CONTRIBUTING.md`, `SECURITY.md`, and a basic issue template from deterministic templates.

---

## 8.12 Portfolio Vault

Build a local project-evidence manager integrated with the Project entity.

### Per-project fields

- display title;
- one-line pitch;
- long description;
- problem;
- solution;
- personal contribution;
- architecture;
- technical challenges;
- results and metrics;
- awards;
- technology stack;
- repository URL;
- live URL;
- demo video;
- screenshots;
- diagrams;
- dates;
- team members;
- lessons learned;
- future work.

### Asset storage

- Store file references by default.
- Offer an explicit “copy into managed vault” option.
- Handle moved or missing assets gracefully.
- Generate thumbnails locally.
- Do not expose arbitrary filesystem paths to the webview outside narrow asset scopes.

### Deterministic exports

Generate editable outputs:

- portfolio JSON;
- Markdown/MDX case study;
- concise project card JSON;
- README project section;
- hackathon submission outline;
- LinkedIn-style project post draft;
- plain-text CV bullets.

These are template-driven. Avoid fabricated achievements or metrics.

### Backup

Allow export/import of portfolio metadata and optionally managed assets as a local archive.

---

## 8.13 OS Lab

Separate the **real live system view** from the **educational simulator**.

### Real System Map

This uses actual process, port, parent/child, project, container, and resource data.

Never show kernel scheduling queues, page tables, interrupts, or deadlock state as real data unless the OS genuinely exposes them and the implementation can prove accuracy.

### Educational simulator

Create clearly labelled simulations for:

- CPU scheduling:
  - FCFS;
  - SJF;
  - SRTF;
  - Round Robin;
  - Priority scheduling;
- memory allocation:
  - first fit;
  - best fit;
  - worst fit;
- page replacement:
  - FIFO;
  - LRU;
  - Optimal;
- deadlock/resource allocation:
  - resource-allocation graph;
  - Banker's algorithm;
- context-switch timeline;
- simple paging/virtual-memory visualization.

Users can define processes/resources, run the simulation, step through events, and compare metrics.

This module should share visual design primitives with the live topology, but must be marked **Simulation** everywhere.

Build this after the daily-use features are stable.

---

## 8.14 Settings, permissions, and diagnostics

### Settings areas

- appearance;
- refresh mode;
- project roots;
- custom project commands;
- detector settings;
- privacy controls;
- export redaction;
- metric retention;
- quarantine retention;
- notifications;
- tray behavior;
- optional autostart;
- optional external-network checks;
- update channel;
- diagnostics.

### Permissions center

Explain:

- which folders the user selected;
- which features access processes, ports, Docker, Wi-Fi, or app data;
- which features are read-only;
- which actions are destructive;
- which network checks contact external services;
- which capabilities are unavailable due to OS permissions.

### Diagnostics bundle

Generate a support bundle only after preview.

Default bundle should include:

- app version;
- OS version;
- feature availability;
- sanitized errors;
- recent Mr Manager logs;
- configuration without secrets;
- optional user-selected screenshots.

Redact:

- usernames in paths when practical;
- project names if user requests;
- SSID;
- MAC address;
- public IP;
- remote endpoints;
- environment values;
- tokens;
- command-line secrets.

---

## 9. WINDOWS PLATFORM IMPLEMENTATION

Build a Windows platform adapter with narrow, testable modules.

### Processes

Use a portable process library for baseline information and Windows APIs where needed.

Handle:

- access denied;
- protected processes;
- exited processes between snapshots;
- PID reuse;
- command-line access failure;
- process-tree changes.

### Ports

Use Windows ownership-aware TCP and UDP endpoint tables or an equivalent safe native mechanism to map endpoints to PIDs. Include IPv4 and IPv6 where possible.

### Network adapters

Use appropriate Windows adapter APIs for:

- addresses;
- interface state;
- metrics;
- gateways;
- DNS where available.

### Wi-Fi

Use the Native Wi-Fi API where practical for current connection and signal quality. Use command-line fallback only when robust and safely parsed.

### Battery

Use Windows power/battery APIs or CIM/WMI where appropriate. Treat missing manufacturer data as unsupported.

### GPU

Use provider adapters as described earlier. Parse external-command output defensively, enforce timeouts, and never assume NVIDIA exists.

### Installed apps and executable discovery

Use safe sources such as:

- PATH lookup;
- known directories;
- installed-application registry entries;
- running process executable paths.

Avoid slow full-disk executable scans.

### Process termination

- Graceful termination for Mr Manager-managed processes.
- Process-tree management using Windows Job Objects or another reliable technique.
- Force termination is a distinct action.
- Never terminate protected system processes silently.

### Privilege model

Run as a standard user.

If a feature is unavailable without elevation:

- show what is missing;
- explain why;
- keep the rest of the app working;
- do not automatically relaunch the whole application as administrator.

---

## 10. BACKEND ARCHITECTURE

### Platform traits

Define traits similar in intent to:

```rust
trait ProcessInspector { ... }
trait PortInspector { ... }
trait SystemMetricsProvider { ... }
trait NetworkInspector { ... }
trait BatteryProvider { ... }
trait GpuProvider { ... }
trait AppInventoryProvider { ... }
trait ProcessController { ... }
```

Product/domain services depend on traits, not Windows calls directly.

### Detector traits

```rust
trait IntegrationDetector {
    fn id(&self) -> &'static str;
    async fn probe(&self, context: &ProbeContext) -> Result<IntegrationStatus, ProbeError>;
}
```

Provide timeouts and cancellation.

### Project detectors

Separate manifest parsing, package-manager detection, script generation, Git inspection, Compose parsing, and doctor rules.

### Background collectors

Use one coordinated collector system rather than separate uncontrolled polling loops.

Possible default intervals:

- system metrics: 1–3 seconds based on refresh mode;
- processes: 2–5 seconds;
- listening ports: 3–5 seconds;
- Docker: 5 seconds while page visible, slower otherwise;
- integration inventory: manual and occasional refresh;
- project scans: manual/watch-driven, not continuous recursive polling.

Pause or reduce expensive work when the application is hidden and no recording is active.

### Event model

Send typed Tauri events for:

- metric samples;
- process lifecycle;
- managed-process logs;
- scan progress;
- cleaner progress;
- Docker changes;
- recording state;
- notification events.

Do not flood the frontend with full snapshots when a delta is sufficient.

### Command boundaries

Implement narrow Tauri commands. Examples:

```text
get_overview_snapshot
list_projects
add_project
add_project_root
scan_project
run_project_command
stop_managed_process
get_managed_process_logs
list_processes
request_external_process_termination
list_ports
get_topology_graph
list_integrations
probe_integration
get_ollama_status
get_docker_status
list_containers
container_action
parse_compose_project
run_compose_doctor
get_system_snapshot
start_metric_recording
stop_metric_recording
list_metric_sessions
get_network_snapshot
run_network_diagnostic
scan_cleanup_candidates
create_cleanup_plan
execute_quarantine_plan
restore_quarantine_item
purge_quarantine_item
generate_readme_preview
apply_readme
portfolio_create
portfolio_update
portfolio_export
```

Validate every argument in Rust.

### Error model

Return typed serializable errors containing:

- stable code;
- human message;
- optional remediation;
- safe technical details;
- whether retry is appropriate;
- whether permission/elevation is relevant.

Do not expose raw panic traces to the UI.

---

## 11. SECURITY MODEL

Treat the webview as untrusted relative to the Rust system layer.

### Tauri and IPC

- Use a strict Content Security Policy.
- Use Tauri capabilities with least privilege.
- Main window receives only required commands.
- Preview windows receive no privileged IPC.
- Do not enable broad filesystem globs.
- Do not expose generic shell execution to JavaScript.
- Do not interpolate user strings into shell commands.
- Use `Command::new(executable).args(args)` or equivalent.
- Canonicalize paths at trust boundaries.
- Validate project IDs and resolve paths server-side.

### Filesystem

- All scans must begin from an explicit user-selected root.
- Prevent path traversal.
- Prevent symlink/junction escape.
- Set maximum file sizes for parsing.
- Write app data atomically where practical.
- Use backups for user-facing file replacement.

### URLs and previews

- Validate scheme and host.
- Default to loopback/private-address previews.
- Isolate the preview webview.
- Block navigation to unexpected origins or open it externally after confirmation.
- Never give local pages access to Mr Manager commands.

### Secrets

- Do not persist raw env values.
- Avoid logging full command lines when they may contain secrets.
- Redact headers and URLs.
- Keep diagnostics previewable.
- Never include secret values in generated README or portfolio output.

### Destructive operations

Use a consistent confirmation system showing:

- action;
- target;
- scope;
- reversibility;
- expected consequence.

Quarantine is reversible. Purge is not.

### Threat model

Create `docs/THREAT_MODEL.md` covering:

- malicious local project files;
- path traversal;
- symlink/junction attacks;
- hostile local web page in preview;
- command injection;
- secret leakage;
- compromised custom detector configuration;
- race conditions between scan and cleanup;
- PID reuse;
- privilege confusion;
- malicious Compose/manifests;
- oversized parser inputs;
- untrusted ANSI/log content;
- updater compromise;
- diagnostics leakage.

---

## 12. VISUAL DESIGN BRIEF

Create a visually strong but credible desktop tool.

### Style

- Dark neutral foundation, optional light theme later.
- One restrained accent color plus semantic status colors.
- Dense but readable information hierarchy.
- Crisp borders, subtle depth, limited blur.
- Avoid excessive gradients, oversized rounded cards, neon overload, and fake “hacker” styling.
- The topology and timelines should provide the wow factor, not decorative noise.

### Layout

- Optimize for 1366×768 through 2560×1440.
- Minimum usable window size around a typical laptop display.
- Resizable panels.
- Persistent navigation.
- Virtualized tables.
- Empty states that explain how to enable real data.

### Interaction

- Keyboard navigation.
- `Ctrl+K` command palette.
- Context menus for safe quick actions.
- Tooltips for technical concepts.
- Confirmation dialogs with exact target details.
- Reduced-motion setting.
- High-contrast status states not based on color alone.
- Screen-reader labels for controls.

### Overview visual hierarchy

A good layout might include:

- compact top status strip;
- active-project cards;
- live services list;
- resource spark lines;
- warnings/activity column;
- quick actions;
- small topology preview.

### Topology visual hierarchy

Node styles should visually distinguish:

- projects;
- processes;
- ports;
- URLs;
- containers;
- Compose services;
- databases;
- local tools;
- models;
- network adapters.

Use animation only for meaningful lifecycle changes and active traffic indicators. Provide a reduced-motion mode.

---

## 13. PERFORMANCE TARGETS

Use these as engineering goals, not unsupported marketing claims.

- Application remains responsive with at least 1,000 process rows.
- Process and port views use virtualization.
- No unbounded in-memory logs.
- No unbounded metric history.
- Background polling is coordinated and cancellable.
- Expensive project scans run off the UI thread and expose progress.
- Cleaner scans are cancellable and bounded.
- Application idle CPU should remain low in Normal mode.
- Startup should not synchronously scan all projects or integrations.
- Cache expensive stable metadata and refresh it intentionally.
- Use debounced graph layout and incremental updates.

Add a developer-only diagnostics panel showing collector timings and dropped events so performance problems can be measured.

---

## 14. TESTING STRATEGY

Testing is mandatory.

### Rust unit tests

Cover:

- manifest parsing;
- package-manager detection;
- conflicting lockfiles;
- Git porcelain parsing;
- Docker JSON/JSONL parsing;
- Compose doctor rules;
- Ollama response parsing;
- URL inference;
- process association scoring;
- topology edge evidence;
- cleanup rules;
- path canonicalization;
- dangerous-root rejection;
- symlink/reparse-point handling where testable;
- cleanup plan immutability;
- quarantine manifest creation;
- restore conflict handling;
- README template generation;
- redaction;
- metric correlation logic;
- detector timeout behavior.

### Integration tests

Use fixture projects to verify:

- Node project scan;
- Python/uv project scan;
- Rust project scan;
- mixed monorepo scan;
- Compose parse and doctor;
- fake managed process launch and log capture;
- process-tree stop behavior;
- port mapping where possible;
- quarantine and exact restore round trip;
- database migrations;
- diagnostics export redaction.

Never use the developer's real project folders for destructive tests.

### Frontend tests

Cover:

- loading, unavailable, permission denied, empty, partial, and error states;
- project add flow;
- command confirmation;
- process filter/search;
- topology evidence drawer;
- cleanup review and confirmation;
- quarantine restore flow;
- README preview/diff;
- portfolio export;
- privacy settings;
- no-Docker/no-Ollama/no-GPU states.

### End-to-end acceptance fixture

Create a synthetic fixture workspace with:

- a small Vite-like project;
- a Python project;
- a Rust project;
- a Compose stack;
- expected cleaner artifacts;
- sample portfolio assets.

Use it to demonstrate the app without touching real user data. Clearly label fixture/demo mode.

### Security tests

Add regression tests for:

- command injection strings in project paths and script names;
- path traversal;
- malicious YAML/TOML/JSON sizes;
- ANSI escape sequences in logs;
- secret redaction;
- preview-window IPC isolation configuration;
- cleanup race conditions where feasible;
- restoring into an occupied path.

---

## 15. IMPLEMENTATION PHASES

Complete these sequentially. Keep the application runnable at the end of each phase.

### Phase 0 — Repository and engineering foundation

Deliver:

- Tauri 2 + React + TypeScript scaffold;
- strict lint/typecheck/format configuration;
- Rust formatting/lint/test setup;
- SQLite migration system;
- typed command/error pattern;
- logging with redaction;
- Tauri capability baseline;
- app shell and navigation;
- `AGENTS.md`;
- implementation, architecture, decisions, privacy, and threat-model docs;
- CI for frontend and Rust checks.

### Phase 1 — Real system foundation

Deliver:

- Windows system snapshot;
- process list;
- CPU/RAM/disk/battery baseline;
- port ownership mapping;
- process detail drawer;
- unavailable/permission states;
- coordinated polling;
- virtualized process table.

Acceptance:

- The app displays real current processes and maps at least common listening ports to owning PIDs on Windows.

### Phase 2 — Projects and managed commands

Deliver:

- add project and project-root discovery;
- Node/Python/Rust detection;
- package-manager and scripts;
- Git summary;
- environment-file key comparison;
- notes/checklists/tags;
- safe command preview;
- managed process supervisor;
- log viewer;
- graceful/force stop flow.

Acceptance:

- A fixture dev server can be started, logs viewed, port discovered, and full process tree stopped.

### Phase 3 — Topology and Localhost Bridge

Deliver:

- topology graph;
- evidence/confidence system;
- process/project/port/URL edges;
- LAN URL generation;
- binding explanation;
- QR code;
- isolated local preview window;
- relevant quick actions.

Acceptance:

- Starting a fixture server visibly creates a project → process → port → URL chain.

### Phase 4 — Docker and Compose Doctor

Deliver:

- Docker availability state;
- container list/state/logs;
- safe start/stop/restart;
- Compose file discovery;
- canonical Compose parsing where available;
- Compose topology;
- initial doctor rule set;
- project/container association.

Acceptance:

- A fixture Compose project renders as services/networks/volumes/ports and receives deterministic diagnostics.

### Phase 5 — Local services, Ollama, WSL, and VPN/network

Deliver:

- detector framework;
- developer runtime/tool detectors;
- Ollama version/models/running-models view;
- WSL summary;
- network adapters;
- Wi-Fi quality;
- probable VPN evidence;
- privacy controls;
- safe diagnostics.

Acceptance:

- Absence of any integration does not crash the app, and detected integrations show evidence and version/state.

### Phase 6 — System Diagnostics and recordings

Deliver:

- polished metrics dashboard;
- GPU provider abstraction;
- recording sessions;
- event annotations;
- deterministic “what changed?” correlation;
- retention/downsampling;
- export/delete.

Acceptance:

- A recorded fixture build produces a timeline and identifies the largest process-resource delta without claiming causation.

### Phase 7 — Cleaner and quarantine

Deliver:

- bounded scanner;
- categorized candidates;
- size calculation;
- cleanup-plan review;
- quarantine;
- restore;
- purge with strong confirmation;
- manifests;
- safety docs and comprehensive tests.

Acceptance:

- A fixture `node_modules` directory can be quarantined and restored byte-for-byte/path-for-path without affecting source files.

### Phase 8 — README Forge and Portfolio Vault

Deliver:

- deterministic README generation;
- preview/diff/backup/apply;
- project assets;
- case-study fields;
- local thumbnails;
- JSON/Markdown/MDX/text exports;
- backup/import.

Acceptance:

- A fixture project can produce a useful editable README preview and a portfolio case study without AI or fabricated content.

### Phase 9 — OS Lab

Deliver:

- scheduling simulator;
- memory allocation simulator;
- page replacement simulator;
- deadlock/resource simulator;
- step controls and comparison metrics;
- unambiguous Simulation labels.

### Phase 10 — Open-source release quality (Stop coding here and ask for approval. I will manually check the .exe application before adding the final touches to make it open source)

Deliver:

- system tray;
- optional autostart;
- notifications;
- updater configuration ready for signed releases;
- Windows installer build;
- GitHub Actions release workflow;
- complete README;
- contribution guide;
- detector contribution spec;
- cleaner rule contribution spec;
- screenshots generated from fixture/demo mode;
- demo script;
- changelog;
- Apache-2.0 license unless an existing repository license dictates otherwise.

Do not implement updater installation until signing and release integrity are properly configured.

---

## 16. ACCEPTANCE CRITERIA FOR THE COMPLETE APPLICATION

The complete application is done only when all of these are true:

1. It launches on Windows as a real Tauri desktop application.
2. It works without an account, API key, cloud backend, or internet connection.
3. A clean launch makes no external network request.
4. It shows real system processes and resource metrics.
5. It maps listening development ports to owning processes.
6. It can register and scan real project folders safely.
7. It detects at least Node, Python/uv, and Rust projects.
8. It shows Git branch and local working-tree state without fetching.
9. It starts a structured project command, streams logs, discovers its port, and stops its process tree.
10. It creates an evidence-based project/process/port/URL topology.
11. It generates usable local and LAN URL candidates and explains binding limitations.
12. Its preview window cannot call privileged Mr Manager IPC.
13. It handles Docker absent, stopped, and running states gracefully.
14. It displays containers and Compose topology when available.
15. Compose Doctor catches fixture port conflicts, missing env variables, and health/readiness concerns.
16. It detects Ollama when available and lists installed/running models through the local API.
17. It displays network adapters, Wi-Fi quality where supported, and evidence-based probable VPN state.
18. It records a performance session and correlates process/resource changes.
19. It never fabricates unavailable GPU, fan, battery, or network data.
20. Cleaner scans only explicit selected roots.
21. Cleaner never permanently deletes during quarantine.
22. A quarantined fixture can be restored safely.
23. README Forge previews and backs up before applying.
24. Portfolio Vault exports deterministic project data and case-study Markdown.
25. OS Lab is clearly separated from real system data.
26. Destructive actions require confirmation.
27. Secrets are redacted from logs, exports, and generated docs.
28. Core Rust and frontend test suites pass.
29. Lint, typecheck, formatting, and production builds pass.
30. The repository contains useful architecture, privacy, security, contribution, and release documentation.

---

## 17. OPEN-SOURCE DOCUMENTATION

Create high-quality documentation.

### `README.md`

Include:

- concise product pitch;
- screenshots from fixture/demo mode;
- feature matrix with implemented status;
- privacy statement;
- safety statement;
- Windows prerequisites;
- development setup;
- build commands;
- architecture overview;
- supported integrations;
- limitations;
- roadmap;
- contribution guide link;
- security reporting link;
- license.

### `docs/ARCHITECTURE.md`

Explain:

- frontend/backend boundary;
- collector system;
- platform adapters;
- domain model;
- SQLite;
- Tauri commands/events;
- topology construction;
- detector framework;
- process supervisor;
- cleaner transaction model;
- preview isolation;
- future CLI path.

### `docs/DETECTOR_SPEC.md`

Explain how contributors can add a safe detector, supported evidence types, timeouts, version parsing, tests, and why arbitrary commands are restricted.

### `docs/CLEANER_SAFETY.md`

Explain candidate categories, quarantine design, restoration, purge, symlink handling, dangerous roots, and failure recovery.

### `PRIVACY.md`

State exactly what is collected locally, what is never transmitted, optional external checks, retention, exports, and user controls.

### `SECURITY.md`

Include responsible disclosure guidance and supported versions. Do not publish a personal secret address directly in code; use a repository-configurable contact placeholder if needed.

### `docs/DEMO_SCRIPT.md`

Create a 90-second demo sequence showing:

1. Overview;
2. add fixture project;
3. start dev server;
4. graph link appears;
5. open LAN QR;
6. inspect Docker stack;
7. show Ollama if available or fixture state clearly labelled;
8. record a build spike;
9. quarantine and restore build artifacts;
10. generate README and portfolio case study.

---

## 18. CODING STANDARDS

### TypeScript

- Strict mode.
- No `any` except narrow justified interop boundaries.
- Schema validation for IPC payloads where useful.
- Feature-oriented modules.
- Accessible components.
- Avoid global mutable state.
- No business logic buried in visual components.

### Rust

- `cargo fmt` clean.
- `cargo clippy` clean with warnings treated seriously.
- Typed errors.
- No panics for expected runtime conditions.
- No `unwrap()` in production paths unless an invariant is proven and documented.
- Timeouts on subprocesses and local HTTP probes.
- Cancellation for long scans.
- Structured logs.
- Secure command construction.
- Unit-test pure parsers and rules.

### Database

- Versioned migrations.
- Transactions for multi-step state changes.
- Foreign keys enabled.
- Do not store ephemeral full process snapshots indefinitely.
- Store paths in a normalized but display-safe form.

### UX copy

- Explain technical issues plainly.
- Distinguish Error, Warning, Information, and Unsupported.
- Avoid alarmist wording.
- Avoid claiming certainty for heuristic findings.

---

## 19. DELIVERABLES FROM THIS CODEX TASK

Do not return only a design document.

Produce:

1. the working repository;
2. implementation plan and status documents;
3. functional vertical slices in milestone order;
4. real tests and fixtures;
5. build/run instructions;
6. a security/privacy architecture;
7. a polished UI using real local data;
8. an open-source-ready README and contributor structure.

At the end of each work session, update `docs/IMPLEMENTATION_STATUS.md` with:

- completed work;
- commands run;
- test/build status;
- known limitations;
- exact next milestone;
- unresolved risks.

Do not declare a feature complete when it is only mocked, visually stubbed, or untested.

---

## 20. BEGIN NOW

Begin with the following exact sequence:

1. Inspect the repository and local toolchain.
2. Create or update `AGENTS.md` with build, test, safety, and architecture rules from this prompt.
3. Create `docs/IMPLEMENTATION_PLAN.md`, `docs/IMPLEMENTATION_STATUS.md`, and `docs/DECISIONS.md`.
4. Scaffold or repair the Tauri 2 + React + TypeScript foundation.
5. Establish strict least-privilege Tauri capabilities and a typed Rust command boundary.
6. Add SQLite migrations and application settings.
7. Implement Phase 1 as a complete real-data vertical slice.
8. Run all checks and fix errors.
9. Continue sequentially through later phases while preserving a runnable application.

The first visible milestone must not be a fake dashboard. It must show real Windows process and system information with explicit unsupported/permission states.

Build Mr Manager as a trustworthy daily-use application and a serious open-source engineering portfolio project. Remember that the app should have all non destructive stuff
