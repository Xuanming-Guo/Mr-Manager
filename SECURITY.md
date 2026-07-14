# Security policy

## Supported versions

Mr Manager is pre-release software. Security fixes are applied to the current development line. No build should be treated as a signed public release until Phase 10 is explicitly approved and release integrity is configured.

## Reporting a vulnerability

Please use the repository host's private security-advisory feature. Until a public repository owner is configured, maintainers must replace this paragraph with the final private reporting channel before release. Do not include real credentials, private source code, or unnecessary machine data in a report.

Include the affected version/commit, Windows version, reproduction steps using synthetic data where possible, impact, and whether the issue involves command execution, path escape, secret disclosure, process control, preview IPC, quarantine/restore, or updater integrity.

## Security boundaries

- React has no generic shell or broad filesystem access.
- Rust validates typed command inputs and resolves targets server-side.
- The app normally runs without elevation.
- Background tasks are bounded, typed Rust operations; navigation cannot replace their targets or broaden their scope.
- Localhost preview content must receive no privileged IPC.
- Scans are bounded to selected roots and do not follow reparse points by default.
- Project commands can launch only from scanner-detected scripts for registered projects, using exact executable and argument vectors.
- Mr Manager-managed project runs are assigned to a Windows Job Object before they are exposed as running.
- Managed stdout/stderr logs are bounded and redacted before rendering.
- Topology edges carry source evidence and confidence; no edge is displayed without evidence.
- Local previews open only loopback/private HTTP(S) URLs in a separate webview label with no main-window command capability and same-origin navigation constraints.
- Docker inspection uses exact Docker CLI arguments with timeouts and structured output parsing; no generic Docker shell command is exposed.
- Docker start/stop/restart requires an exact confirmation phrase for the listed container. Delete, remove, prune, image deletion, network deletion, and volume deletion are not exposed in the MVP.
- Compose parsing is bounded to registered project files; fallback parser results are labeled when canonical Docker Compose config is unavailable.
- State-changing operations show target, scope, consequence, and reversibility before confirmation.
- Cleaner plans are derived from bounded server-held scan evidence, persisted once, and revalidated immediately before mutation. Quarantine prefers atomic moves; cross-volume copies require free-space checks and verification before source removal. Restore never overwrites. Item and whole-manifest purge are separately confirmed, constrained to canonical managed quarantine storage, and revalidated per item.
- Captured CLI probes use exact executable/argument vectors, bounded output and timeouts, and the shared Windows no-console child-process policy.

Please see [the threat model](docs/THREAT_MODEL.md) for the detailed security design. Do not publish an exploit before maintainers have had a reasonable opportunity to investigate and coordinate a fix.
