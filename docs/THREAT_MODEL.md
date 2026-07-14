# Mr Manager threat model

## Assets and adversaries

Protected assets include source code, credentials, environment values, project history, local databases, process control, Docker state, network configuration, quarantine content, diagnostics, and application update integrity. Inputs may be malicious even when local: cloned repositories, manifests, Compose/YAML/TOML/JSON, filenames, symlinks/junctions, ANSI logs, localhost pages, detector definitions, and subprocess output.

## Primary threats and controls

| Threat                              | Required controls                                                                                                                                                                                   |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Command injection                   | Backend-selected executable plus argument vector; no frontend shell strings; preview exact execution; validate registered project and stored script IDs.                                            |
| Path traversal or junction escape   | Canonicalize at trust boundaries; explicit selected roots; reject dangerous roots; do not follow reparse points; revalidate immediately before mutation.                                            |
| Hostile localhost preview           | Separate webview label, zero Mr Manager IPC, private/loopback origin validation, same-origin navigation restriction, and external-open refusal.                                                     |
| Secret leakage                      | Redact at ingestion and before logs, diagnostics, recordings, exports, snapshots, generated documents, and screenshots; never persist raw env values.                                               |
| PID reuse/privilege confusion       | Identify by PID plus start time; revalidate before action; deny self/system/protected targets; run normally as standard user.                                                                       |
| Oversized or malicious parser input | File-size/depth/count limits, cancellation, timeouts, bounded allocations, deterministic parsers, and per-item errors.                                                                              |
| Cleaner race/data loss              | Immutable reviewed plans, canonical paths, same-root checks, no link traversal, locked-file handling, transactional manifests, verified copy-before-remove, safe restore conflicts, separate purge. |
| Route change loses operation state  | Rust-owned bounded tasks, typed results, safe cancellation points, global status polling, and durable quarantine manifests for mutation recovery.                                                   |
| Compromised detector configuration  | Declarative evidence only in v1; no arbitrary commands; allowlisted probes, loopback enforcement, timeouts, bounded output.                                                                         |
| Docker/remote context surprise      | Surface Docker context/daemon state; no deletion/prune in MVP; exact CLI arguments only; bounded structured output; preview and confirm lifecycle targets.                                          |
| Diagnostics privacy                 | Preview bundle contents; redact usernames/project names when selected, SSID, MAC/public IP, remote endpoints, tokens, and command secrets.                                                          |
| Updater compromise                  | No updater installation before Phase 10 signing, provenance, trusted metadata, rollback, and release-integrity review.                                                                              |

## Security invariants

- Clean first launch performs no external request.
- Scans begin only at explicit user-selected roots and are read-only.
- Managed commands launch only from detected scripts for registered projects, and managed logs are bounded and redacted before display.
- Managed project process trees are controlled through a Windows Job Object; external process termination remains a separate stronger-confirmation surface.
- Docker lifecycle controls are limited to exact start/stop/restart against listed containers and require typed confirmation; remove/delete/prune operations are absent.
- Compose parsing is bounded to registered project files and deterministic fixtures; unresolved or fallback parsing is labeled instead of treated as certainty.
- Topology edges must include evidence and confidence; localhost URL nodes must not claim companion-device reachability.
- Preview windows do not receive `main` window permissions and cannot invoke Mr Manager commands.
- Destructive or state-changing actions cannot be triggered by navigation, refresh, or untrusted page content.
- Unsupported evidence is never replaced by a guess.
- Tests that mutate files use synthetic temporary fixtures, never real projects.
- Cleanup scans cannot authorize mutation when cancelled or truncated. Immutable plans execute once, manifests are persisted before moves and after every item, and occupied restore destinations fail closed unless the user chooses a server-generated sibling path.
- Retired schema-5 Content Studio records and files are never deleted by migration and are no longer exposed through the webview command boundary.

Security regression coverage must include injection strings, path traversal, reparse points, malicious parser sizes, ANSI escapes, redaction, preview capability isolation, PID reuse, cleanup races, and occupied restore destinations.
