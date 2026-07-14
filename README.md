# Mr Manager

Mr Manager is a local-first, Windows-first developer command center built with Tauri 2, Rust, React, and strict TypeScript. Its goal is to connect real evidence about projects, processes, ports, local services, Docker, and system resources without requiring an account, cloud service, API key, telemetry, or an internet connection.

## Current status

Mr Manager has a verified **Phase 8 vertical slice plus the pre-Phase-9 usability and reliability pass** and is now ready for Phase 9 OS Lab work. Long operations continue across in-app navigation through a Rust-owned task center, Topology defaults to automatic live development services, Docker returns specific partial states, Network and System Diagnostics use labelled responsive charts, refresh is explicitly Normal or Fast, and Cleaner separates reversible quarantine from confirmed permanent purge. The educational OS Lab is not yet complete.

There are no trusted installers or signed releases. After Phases 0-9, the project must produce an unsigned internal Windows executable for manual review and stop for explicit approval before any Phase 10 release work.

See [the implementation plan](docs/IMPLEMENTATION_PLAN.md), [current implementation status](docs/IMPLEMENTATION_STATUS.md), and [decision log](docs/DECISIONS.md) for the authoritative details.

## Principles

- Local-first and useful offline, with no clean-launch network requests or telemetry.
- Read-only inspection by default; state-changing actions require explicit previews, confirmation, and revalidation.
- Rust owns OS, process, network, filesystem, database, Docker, and subprocess access.
- Narrow typed Tauri commands cross the untrusted webview boundary.
- Unavailable or partial evidence is labeled honestly; metrics and relationships are never fabricated.
- Destructive tests target synthetic fixtures only.

## Development

Prerequisites are Windows 11, Node.js 22, Rust 1.88, and the [Tauri 2 Windows prerequisites](https://v2.tauri.app/start/prerequisites/).

```powershell
npm ci
npm run tauri dev
```

Run the frontend directly with `npm run dev`; this is only a web preview and intentionally cannot access real system data.

The main verification commands are:

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

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before participating. Security-sensitive findings should not be posted in a public issue; use the repository host's private security-reporting channel.

## License

Licensed under the [Apache License 2.0](LICENSE).
