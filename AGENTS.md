# Mr Manager repository instructions

These instructions apply to every file in this repository. More specific instructions in a nested `AGENTS.md` may add constraints for that subtree, but must not weaken the safety rules here.

## Product contract

Mr Manager is a local-first, Windows-first developer command center built with Tauri 2, Rust, React, and strict TypeScript. It connects real evidence about projects, processes, ports, localhost services, Docker, local tools, system resources, networks, safe cleanup, documentation, portfolio assets, and an explicitly educational OS lab.

The product name is **Mr Manager** in code, UI, documentation, artifacts, and package metadata. The legacy attachment filename is not product branding.

The application must remain useful without an account, internet connection, cloud service, API key, or LLM. A clean first launch must make no external network requests and must not enable telemetry.

## Hard delivery gate

Implement Phases 0 through 9 in order. At the end of Phase 9:

1. Run all required checks.
2. Build an **unsigned internal Windows executable** for manual review. Prefer `npm run tauri build -- --no-bundle` so the review artifact is the raw `mr-manager.exe`, not a release installer.
3. Record the exact artifact path, checksum, commands, and results in `docs/IMPLEMENTATION_STATUS.md`.
4. Stop and ask the user for approval.

Do not begin Phase 10 without that explicit approval. Phase 10 includes release packaging, installer polish, signing, updater/release workflow, public-release collateral, and other open-source release finishing work. The unsigned executable is an internal verification artifact, must not be published as a trusted release, and may trigger Windows reputation warnings.

## Work sequence

- Inspect before editing and preserve unrelated user changes.
- Keep `docs/IMPLEMENTATION_PLAN.md` and `docs/IMPLEMENTATION_STATUS.md` current.
- Work in vertical slices and keep the application runnable after every phase.
- Implement only real data paths. Never display fabricated metrics, integrations, relationships, reachability, or success states.
- Hide unavailable features or label them clearly as unavailable, unsupported, partial, or simulated.
- Record durable or ambiguous engineering choices in `docs/DECISIONS.md`.
- Update architecture, privacy, security, and threat-model documentation when a change affects a trust boundary or data flow.

## Architecture rules

- Rust owns operating-system, process, network, Docker, filesystem, database, and other privileged operations.
- Treat every webview as untrusted relative to Rust.
- Expose narrow, typed Tauri commands; validate all arguments again in Rust.
- Do not expose generic shell execution or broad filesystem scopes to JavaScript.
- Keep domain logic independent of Tauri command wrappers so it can be unit tested and reused by a future CLI.
- Use typed Rust domain models and matching TypeScript DTOs. Do not pass unstructured JSON blobs across IPC.
- Return stable, serializable application errors. Expected failures must not panic.
- Put platform-specific behavior behind traits. Windows is implemented first; unsupported platforms return typed `Unsupported` results.
- Use coordinated, cancellable background collectors. Pause or reduce expensive collection while hidden unless recording is active.
- Every graph relationship needs evidence and a confidence of `certain`, `strong`, or `inferred`.
- A preview webview must have no Mr Manager IPC capability and must not inherit main-window permissions.

## Safety rules

- Inspection and scanning are read-only by default.
- Scan only roots explicitly selected by the user. Never recursively scan an entire drive by default.
- Never delete user data during development or tests.
- Cleaner operations must use immutable reviewed plans, quarantine first, manifests, restore support, and a separate irreversible purge confirmation.
- Reject dangerous roots and prevent traversal and symlink/junction escape.
- Revalidate paths, identities, sizes, and eligibility immediately before mutation to reduce time-of-check/time-of-use risk.
- Never interpolate frontend input into a shell string. Use an exact executable and an argument vector.
- Show the exact command, target, effect, scope, and reversibility before any state-changing action.
- Treat process identity as PID plus start time. Revalidate it before termination or association.
- Never silently modify firewall, VPN, environment, Git, Docker volume, router, startup, or system settings.
- Normal operation must not require administrator privileges. Surface protected/elevated states explicitly.
- Never store or display raw environment values. Redact secrets from command lines, URLs, headers, logs, diagnostics, exports, screenshots, snapshots, generated documents, and error details.
- Bound parser input size, traversal depth, file count, log buffers, metric retention, subprocess duration, and local HTTP probes.
- Strip or neutralize untrusted ANSI/control sequences before rendering logs.
- Synthetic fixtures are the only permitted targets for destructive integration tests.

## Code standards

### TypeScript and React

- Keep TypeScript strict and avoid `any` except at a narrow documented interop boundary.
- Keep business logic out of visual components.
- Prefer feature-oriented modules, accessible primitives, explicit loading/error/empty/unsupported states, and schema validation at IPC boundaries where useful.
- Use TanStack Query for backend state. Restrict local UI state to transient presentation concerns.
- Virtualize large process/file lists and debounce expensive graph layout.

### Rust

- Keep `cargo fmt` clean and treat Clippy warnings seriously.
- Use typed errors for expected runtime failures; reserve `anyhow` for top-level boundaries if introduced.
- Do not use `unwrap()` or `expect()` in production paths unless an invariant is local, proven, and documented.
- Use structured, redacted logging.
- Give subprocesses and probes explicit timeouts and cancellation.
- Use transactions for multi-step persistence and mutation workflows.

### Database

- Use versioned forward migrations and enable foreign keys.
- Store persistent product state, not indefinite raw process snapshots.
- Never store raw secrets or raw `.env` values.
- Normalize paths for comparison while retaining a safe display form.

## Required checks

Run checks proportional to the changed surface. Before marking a phase complete, run the full applicable set:

```powershell
npm run format:check
npm run lint
npm run typecheck
npm run test:run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
npm run tauri build -- --debug --no-bundle
```

Use `npm run tauri dev` for a manual desktop smoke test when the environment supports a GUI. Use fixture/demo mode for screenshots and acceptance flows, and label it clearly.

Never claim a check passed unless the command completed successfully in the current worktree. Record commands, results, limitations, and the next milestone in `docs/IMPLEMENTATION_STATUS.md`.

## Public repository hygiene

- Do not commit generated binaries, local databases, captured machine data, logs, caches, secrets, or machine-specific absolute paths.
- Pin compatible direct dependencies and commit lockfiles.
- Keep fixtures synthetic and free of real identifiers.
- Do not add updater installation or release-signing behavior until Phase 10 is explicitly authorized and integrity is configured.
