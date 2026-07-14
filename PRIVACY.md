# Mr Manager privacy policy

Mr Manager is local-first. It requires no account, cloud backend, telemetry, analytics, AI key, or mandatory internet connection. A clean first launch makes no external network request.

## Data processed locally

With standard-user permissions, Mr Manager may inspect current system/resource information, processes, listening ports, selected project roots, local Git metadata, Docker state, installed developer tools, local Ollama/WSL state, network adapters, and user-created project notes. It stores durable application state in the local application-data directory.

Raw `.env` values and secrets are not stored in SQLite. Project environment inspection stores key names, file names, example status, and file sizes only. Managed command logs are kept in bounded memory in Phase 2 and pass through redaction before rendering, although automated secret detection cannot be perfect.

Topology uses existing local evidence from registered project metadata, managed-run state, process snapshots, and port ownership. It does not perform external reachability checks. Local preview windows are user-opened and limited to loopback/private HTTP(S) URLs generated or accepted by Mr Manager.

Docker inspection uses local Docker CLI output. Container labels, mount paths, Compose file metadata, and Docker logs are processed locally; logs and labels pass through redaction before display. Compose parsing is limited to files already discovered inside registered projects and does not read environment values.

The Network dashboard separates local adapter throughput from internet diagnostics. Local throughput reads operating-system counters and does not need external requests. Internet diagnostics are disabled by default and run only after the user clicks **Run Internet Test** and confirms a preflight explaining that `example.com` may be contacted and data may be consumed. Every result identifies whether it was local-only or contacted the internet; the bounded download probe is not presented as a full multi-server speed benchmark.

Cleaner inspects metadata only below folders explicitly selected by the user. Completed cleanup plans and quarantine manifests store canonical original paths, managed quarantine paths, sizes, counts, categories, verification state, and timestamps in local SQLite. Quarantined file content remains only in the local application-data quarantine directory. Cleaner does not transmit paths or content and never purges automatically.

Background-task records are retained only in memory for the current app session. They contain task labels, timestamps, safe summaries, typed results, and redacted errors; they do not add process history, remote endpoint history, or raw environment values to persistent storage.

Content Studio, README generation, portfolio management, AI agents, MCP, and Obsidian integration are not active product features. Schema-5 content records and previously created application-data files are retained only to avoid deleting user data; the application does not read, mutate, transmit, or expose them through IPC.

## User control

- Filesystem discovery scans only roots the user selects.
- Project commands run only after the user starts a detected script for a registered project; Mr Manager does not run commands during project scanning.
- External network checks are off by default and must identify endpoints before execution.
- Docker lifecycle actions require explicit confirmation and do not include delete, prune, image-removal, network-removal, or volume-removal operations in the MVP.
- Live metrics remain in bounded memory unless recording/history is explicitly enabled.
- Cleaner candidates require review; quarantine is reversible; purge is separate.
- Diagnostics bundles are previewed before export.

Exports redact SSID, MAC addresses, public IP addresses, remote endpoints, DNS servers, network history, environment values, tokens, and command-line secrets by default. Project names and usernames in paths can be additionally redacted.

Uninstalling the application does not silently delete projects or user files. Application data and quarantine require explicit user-managed removal according to the platform/uninstaller policy.
