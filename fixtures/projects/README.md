# Synthetic project fixtures

Everything below this directory is intentionally synthetic. The fixtures exercise project discovery, manifest parsing, package-manager detection, environment-key comparison, monorepo traversal, and conflicting-lockfile diagnostics without reading a developer's real projects.

## Fixture catalog

| Directory               | Expected detections                                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------------------------------- |
| `vite-app`              | Node, npm, Vite-like layout, runnable scripts, public environment keys                                        |
| `python-uv`             | Python, `pyproject.toml`, uv lockfile, src layout                                                             |
| `rust-cli`              | Rust, Cargo lockfile, binary target                                                                           |
| `mixed-monorepo`        | pnpm workspace containing Node, Rust, and Python members                                                      |
| `conflicting-lockfiles` | Node project with npm, pnpm, and Yarn lockfiles; should report a deterministic conflict                       |
| `compose-stack`         | Compose project with deterministic doctor issues for ports, env keys, bind mounts, latest tags, and readiness |

These files contain no credentials, personal paths, live endpoints, or captured machine data. Values in `.env.example` are public fixture configuration only; secret-shaped keys are left blank.

Tests that mutate fixture content must first copy the relevant directory into a temporary location. Never install dependencies, build artifacts, quarantine files, or rewrite manifests in the checked-in fixture tree.
