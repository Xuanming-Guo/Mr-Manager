# Contributing to Mr Manager

Thank you for helping improve Mr Manager. It is Windows-first, local-first, and safety-sensitive, so contributions must preserve user trust as well as functionality.

## Before starting

- Search existing issues before opening a new one.
- Discuss large features, dependency additions, new permissions, trust-boundary changes, persistence changes, or release behavior in an issue first.
- Keep pull requests focused and preserve unrelated work.
- Never include captured machine data, credentials, logs, databases, generated binaries, or personal paths.

## Development setup

Install Windows 11, Node.js 22, Rust 1.88, and the [Tauri 2 Windows prerequisites](https://v2.tauri.app/start/prerequisites/), then run:

```powershell
npm ci
npm run app
```

`npm run dev` opens a web-only preview. It cannot access real system data and must continue to explain that limitation honestly.

## Branches

`main` is the only permanent branch. Create a short-lived branch using one of these forms:

- `feature/<name>` for user-visible capabilities.
- `fix/<name>` for defects.
- `security/<name>` for coordinated security work.
- `refactor/<name>` for behavior-preserving restructuring.
- `test/<name>` for test-only work.
- `docs/<name>` for public documentation.
- `chore/<name>` for tooling and maintenance.

Use lowercase descriptive names, such as `fix/docker-timeout`. Dependabot branches are managed automatically. Do not create permanent `feature`, `fix`, or `develop` branches.

## Engineering expectations

- Keep OS, process, network, filesystem, database, Docker, and subprocess operations in Rust.
- Expose narrow typed Tauri commands and validate all frontend input again in Rust.
- Never interpolate user input into a shell string; use an exact executable and argument vector.
- Preserve offline operation, zero clean-launch egress, and telemetry-free defaults.
- Never invent metrics, reachability, relationships, or success. Label unavailable and partial evidence clearly.
- Keep inspection read-only by default. Mutations require exact previews, bounded targets, confirmation, and immediate revalidation.
- Treat process identity as PID plus start time.
- Redact secrets and control characters from logs, errors, diagnostics, exports, snapshots, and UI output.
- Bound file traversal, parser inputs, retained logs, metric history, probes, and subprocess duration.
- Use only temporary synthetic fixtures for destructive tests.
- Keep TypeScript strict and treat Rust Clippy warnings seriously.

## Verification

Run checks proportional to the change. The full gate is:

```powershell
npm run repo:check
npm run branch:check
npm run format:check
npm run lint
npm run typecheck
npm run test:run
npm run build
npm run test:e2e
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
npm run tauri build -- --debug --no-bundle
```

Never report a check as passing unless it completed in the submitted worktree. If a host policy or missing browser blocks a check, state that limitation explicitly.

## Pull requests

Each pull request should include:

- The user-visible outcome and reason for the change.
- Any effect on permissions, privacy, safety, data flow, or persistence.
- Tests run and their exact results.
- Known limitations and unsupported cases.
- Relevant public documentation changes.

All required checks must pass before squash-merging. Resolve review conversations and keep generated artifacts out of Git.

By contributing, you agree that your contribution is licensed under Apache-2.0 and that you will follow the [Code of Conduct](CODE_OF_CONDUCT.md).
