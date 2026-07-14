# Mr Manager demo script

## Phase 2 fixture flow

Use only the synthetic fixtures under `fixtures/projects`. Do not select a real personal project for destructive or mutation-oriented demos.

1. Start Mr Manager with `npm run tauri dev`.
2. Open **Projects**.
3. Choose **Add project** and select `fixtures/projects/vite-app`.
4. Confirm the scan shows:
   - Node stack;
   - npm package manager;
   - `package.json` scripts;
   - Git unavailable/not-a-repository as applicable;
   - no environment values.
5. In **Structured commands**, verify the displayed command vector for `dev`.
6. Click **Run** for `dev`.
7. Confirm **Managed runs** shows a running PID and redacted log lines.
8. Wait for the port collector refresh and confirm the run detail shows the local URL owned by that PID.
9. Open **Topology**.
10. Confirm the graph shows a project -> managed run -> process -> port -> URL chain with edge evidence.
11. Select the URL node and confirm the QR code is generated locally.
12. Click **Preview** and confirm the service opens in a separate preview window.
13. Return to **Projects** and click **Stop**. If the process does not exit, click **Force**.
14. Confirm the run reaches `exited` or `failed` and no fixture files were modified.

Additional scanner checks:

- Add `fixtures/projects/python-uv` and confirm `.env.example` key names appear without values.
- Add `fixtures/projects/rust-cli` and confirm structured Cargo actions are shown.
- Discover root `fixtures/projects/mixed-monorepo` with the default depth and confirm Node, Python, and Rust members appear.
- Add `fixtures/projects/conflicting-lockfiles` and confirm the lockfile conflict warning appears.

## Phase 4 Docker and Compose Doctor fixture flow

This flow remains safe when Docker is absent or stopped; Mr Manager should show an honest unavailable state instead of fake containers.

1. Add or rescan `fixtures/projects/compose-stack`.
2. Open **Docker**.
3. Confirm the engine cards show one of the real Docker states: CLI missing, installed/stopped, inaccessible, running, or error.
4. If Docker is running, confirm containers, networks, volumes, labels, ports, health, and resource fields are populated from Docker output. Do not run a lifecycle action unless intentionally testing Docker start/stop/restart.
5. Select a container and confirm logs are bounded and redacted.
6. In **Compose visualizer and doctor**, confirm `compose-stack` appears with services `api`, `db`, and `web`.
7. Confirm services render their dependencies, ports, networks, volumes, and healthcheck status.
8. Confirm Compose Doctor reports deterministic fixture issues, including:
   - duplicate host port `8080`;
   - unresolved `API_KEY`;
   - missing bind mount `./missing-nginx.conf`;
   - floating `latest` image tags;
   - `depends_on` target without a healthcheck;
   - database port exposed on all interfaces.
9. Confirm the Compose source label says either canonical Docker config or fallback parser, depending on whether Docker Compose is available.

## Phase 5 Integrations and Network flow

This flow should make no external internet request unless the operator explicitly enables external checks and clicks **Run Internet Test**.

1. Open **Integrations**.
2. Confirm detector cards show installed/not-found/running/unknown states with evidence and timestamps, not fabricated certainty.
3. Confirm Ollama uses only local executable/process evidence and the `127.0.0.1:11434` loopback API; installed and running models appear only when the local API is available.
4. Confirm WSL distro state is read-only and unavailable states are stable if WSL is absent.
5. Open **Network**.
6. Confirm current download/upload, session totals, peaks, per-adapter cards, and the combined timeline update from local OS counters.
7. Confirm gateway, DNS, VPN evidence, LAN IP candidates, loopback-only server warnings, and LAN QR links are shown without exposing DNS server addresses, MAC addresses, SSIDs, public IPs, or remote endpoint history.
8. Confirm **Run Internet Test** is disabled until external checks are enabled in Settings, and every result says whether it was local-only or contacted the internet.

## Phase 6 System Diagnostics and recording flow

This flow records only explicit local sessions. It should not run external internet diagnostics unless the operator separately enables and starts them.

1. Open **System Diagnostics**.
2. Confirm CPU, RAM, disk I/O, network throughput, battery, GPU provider state, collector diagnostics, and virtualized ranked processes are visible.
3. Confirm GPU fields show NVIDIA telemetry only if `nvidia-smi.exe` is available; otherwise they show an honest unsupported state.
4. Click **Start recording** with a fixture-oriented name such as `Fixture build`.
5. Run a synthetic fixture action, for example `npm run build` in `fixtures/projects/vite-app`, or simply let the recorder collect idle samples.
6. Add an annotation such as `Started fixture build`.
7. Stop the recording and confirm it appears in **Recording sessions** with bounded samples and annotations.
8. Open the saved recording and confirm the labelled CPU/RAM timeline, annotation list, and deterministic **Change analysis** finding are shown with non-causal wording.
9. Click export and confirm the JSON says it is redacted by default and does not contain SSIDs, MAC addresses, public IPs, DNS server addresses, remote endpoints, or network history.
10. Delete the recording and confirm it disappears from the session list.

## Phase 7 Cleaner fixture flow

Use only a disposable synthetic fixture containing a `node_modules` directory and a sibling source file.

1. Open **Cleaner** and choose the fixture project root explicitly.
2. Scan and confirm `node_modules` appears with category, reason, confidence, risk, size, file count, lock state, and regeneration guidance. Confirm no candidate is preselected.
3. Select only `node_modules`, create the immutable plan, and type the exact quarantine confirmation.
4. Confirm the source file is unchanged, `node_modules` moved into managed quarantine, and the durable manifest reports its verification method.
5. Click **Restore exact** and confirm the directory returns byte-for-byte and path-for-path.
6. Repeat with an occupied original path and confirm exact restore refuses to overwrite; use **Restore beside existing** only to demonstrate the server-generated conflict-safe path.
7. Show the separate item-specific purge dialog, but do not purge unless intentionally exercising a disposable fixture.
