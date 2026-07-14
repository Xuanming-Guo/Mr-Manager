# Contributing to Mr Manager

Thank you for helping build Mr Manager. The project is early-stage, Windows-first, local-first, and safety-sensitive. Contributions should preserve user trust before adding convenience.

## Before you start

- Read `AGENTS.md`, `docs/IMPLEMENTATION_PLAN.md`, `docs/IMPLEMENTATION_STATUS.md`, and `docs/DECISIONS.md`.
- Discuss large features, dependency additions, trust-boundary changes, or changes to the phase order before implementation.
- Work only in the active phase. Phase 10 release packaging, signing, updater behavior, and public-release work require explicit approval after the Phase 9 internal review gate.
- Keep changes focused and preserve unrelated worktree changes.

## Local setup

Use Windows 11 with Node.js 22, Rust 1.88, and the Tauri 2 Windows prerequisites.

```powershell
npm ci
npm run tauri dev
```

The web-only preview (`npm run dev`) cannot access system data and must continue to show that limitation honestly.

## Engineering expectations

- Keep privileged OS, process, network, filesystem, database, Docker, and subprocess behavior in Rust.
- Expose narrow typed commands and validate inputs again in Rust; never construct shell strings from frontend input.
- Preserve offline operation, zero clean-launch egress, and telemetry-free defaults.
- Do not display invented data or implied success. Represent unsupported, partial, permission-denied, and error states explicitly.
- Inspection is read-only by default. Mutations need exact previews, confirmation, bounded scope, identity/path revalidation, and documented reversibility.
- Never use real user data for destructive tests. Use temporary synthetic fixtures only.
- Redact secrets and control characters from logs, errors, snapshots, exports, and UI output.
- Add tests for behavior and regression risk, including negative and permission-limited paths.

## Verification

Run checks proportional to the change. Before a phase is marked complete, run the full applicable gate:

```powershell
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

Record exact results and limitations in `docs/IMPLEMENTATION_STATUS.md`; never report a check as passing unless it completed successfully in the current worktree.

## Pull requests

A pull request should explain the user-visible outcome, phase and acceptance criterion, trust-boundary or data-flow impact, tests run and their results, known limitations, and any documentation updates. Keep generated binaries, machine data, databases, logs, caches, secrets, and machine-specific paths out of Git.

By contributing, you agree that your contribution is licensed under the Apache License 2.0 and that you will follow `CODE_OF_CONDUCT.md`.
