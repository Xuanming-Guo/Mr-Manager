# Security policy

## Supported versions

Mr Manager is pre-release source software. Security fixes are applied to the current `main` branch. There is no signed installer or supported public binary release yet.

## Reporting a vulnerability

Do not disclose a vulnerability in a public issue, discussion, pull request, screenshot, or log.

Use [GitHub private vulnerability reporting](https://github.com/Xuanming-Guo/Mr-Manager/security/advisories/new). Include the affected commit, Windows version, impact, and minimal reproduction steps using synthetic data where possible. Remove real credentials, private source code, usernames, project paths, IP addresses, and unnecessary machine data.

Particularly sensitive areas include command execution, path traversal or reparse-point escape, secret disclosure, process control, preview-window IPC, network diagnostics, Docker actions, and Cleaner quarantine, restore, or purge behavior.

Maintainers will acknowledge a complete report, investigate privately, coordinate a fix, and credit the reporter if requested. Please allow reasonable time for remediation before public disclosure.

## Security boundaries

- The React webview has no generic shell command or broad filesystem scope.
- Rust validates typed inputs, resolves targets server-side, and owns privileged operations.
- Normal operation does not require administrator privileges.
- Project commands use scanner-detected scripts, exact executables, and argument vectors without shell interpolation.
- Captured subprocesses have timeouts, bounded output, redaction, and the shared Windows hidden-process policy.
- Filesystem scans are bounded to user-selected roots and reject dangerous roots and reparse-point escape.
- Local preview windows accept only constrained loopback/private URLs and receive no main-window IPC capability.
- Internet diagnostics are disabled by default and disclose external contact before execution.
- State-changing operations show their target and consequence and revalidate identity immediately before mutation.
- Cleaner uses reviewed immutable plans, quarantine-first behavior, restore support, and separately confirmed permanent purge.
- Generated binaries, databases, logs, and captured machine data are excluded from the repository.

No updater or release-signing trust path is currently active.
