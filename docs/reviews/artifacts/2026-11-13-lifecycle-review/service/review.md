# Service shutdown review — 2026-11-13

HEAD reviewed: `56250b92`. Review only; no production, test, configuration,
service, tablet or TODO changes by this reviewer. Parent records T1411/T1412.

## Confirmed findings

- **T1411, medium severity, medium effort:** `host/src/linux_main.rs:1873–1885`
  abandons the joined primary/monitor retirement after five seconds and aborts
  the monitor; the `primary.stop()` native completion result is discarded.
  `host/src/monitor.rs:593–620` normally drains retiring sessions, and
  `host/src/monitor/retirement.rs:29–43` waits for route and native session
  completion. Aborting drops those asynchronous views. The original route
  worker remains retained by `host/src/usb/routes.rs:93–120,169–185`, but
  `host/src/runtime_owner.rs:4–12` fences only Wi-Fi before returning from the
  process entry. A static retirement pool does not prevent process exit.
  Ordinary stop with removal failure lasting beyond the five-second join can
  therefore exit while original route/session obligations remain unsettled;
  external mappings can survive while process-held leases are lost. The
  isolated probe below directly confirms the route/runtime-owner boundary.
  Complete by retaining and draining admitted original route/session/native
  obligations through normal/error/cancelled process-entry teardown, closing
  further admission, and adding permanent failing-before/passing-after
  subprocess and daemon-orchestration regressions. Merely increasing the
  five-second timeout does not establish settlement. One cohesive process-exit
  ownership outcome; Windows T1352 remains separately blocked.

- **T1412, medium severity, small effort:** `scripts/blent.service:22` and
  `packaging/blent.service:14` set `TimeoutStopSec=15`;
  `scripts/write-systemd-service.sh:19–25` preserves that policy. The installed
  unit independently confirms `TimeoutStopUSec=15s`, `KillMode=control-group`,
  `KillSignal=15`, `FinalKillSignal=9`, `SendSIGKILL=yes` (full observation in
  `installed-unit.txt`). Even the functioning Wi-Fi exit fence at
  `host/src/wifi/attempt.rs:63–78` can therefore be killed during ordinary
  managed stop/restart. `host/src/linux_main.rs:2598–2621` requests SIGTERM and
  errors after the ten-second CLI wait; it does not itself escalate to SIGKILL.
  After ExecStop exits, systemd applies its group signal/stop timeout, so the
  observed policy is not a claim of exactly fifteen seconds from the GUI click.
  The installed `systemd.service(5)` TimeoutStopSec and `systemd.kill(5)`
  KillMode/SendSIGKILL sections specify these semantics. Complete by making both
  published units and their generated form preserve pending cleanup owners,
  keeping stop/restart results honest and preventing premature restart. Add a
  permanent safe isolated service/controller regression with delayed cleanup,
  recovery and failed cleanup. This unit policy remains independently relevant
  to Wi-Fi even before T1411; integrate both outcomes for display routes.

## Evidence and safe paths

`route_exit_probe.rs` uses the actual public `blent::usb::Routes` and
`blent::runtime_owner::run` with an independent in-memory command adapter.
There is no real ADB executable, mapping, device, input injection or service
operation. Ownership is local to the disposable process, with exactly two fake
route entries; process exit ends its retained fake worker. The failure case
rejects removal while preserving both entries. The successful control
acknowledges removal and clears both entries. `route-exit-result.txt` records:

- Failure: runtime owner returned; native completion false; 2/2 fake routes
  remained; process exit 0.
- Settled control: runtime owner returned; completion true; 0/2 fake routes
  remained; process exit 0.

Built library once with
`cargo build --offline --locked -p blent --lib --features accessory-transport --message-format=json`;
compiled the probe with rustc 1.100.0 using that library and dependency search
paths from Cargo compiler-artifact filenames. Old cached 1.98.1 rlibs were
incompatible with this compiler; those unsuccessful link attempts provided no
runtime evidence. The compiled probe was stored outside the repository.

The direct CLI preserves an error when its ten-second stop wait expires.
The GUI direct restart uses `blent_config::lifecycle::restart` so failed stop
prevents starting its replacement (`gui/src/platform/linux.rs:210–220`).
The monitor retains route ownership and session slots while its retirement
future remains alive. Wi-Fi already closes admission and drains its original
owners at process exit. No additional confirmed findings were identified.

## Scope and limits

Relevant paths examined in **19 implementation/service files**:
`host/src/main.rs`, `host/src/linux_main.rs`, `host/src/runtime_owner.rs`,
`host/src/monitor.rs`, `host/src/monitor/retirement.rs`,
`host/src/monitor/routes.rs`, `host/src/session_supervision.rs`,
`host/src/usb.rs`, `host/src/usb/routes.rs`, `host/src/usb/routes/worker.rs`,
`host/src/wifi/attempt.rs`, `common/src/retirement_work.rs`,
`common/src/linux/processes.rs`, `common/src/linux/daemon.rs`,
`common/src/commands.rs`, `gui/src/platform/linux.rs`,
`scripts/blent.service`, `packaging/blent.service`,
`scripts/write-systemd-service.sh`.

Five existing regression/support files were consulted to check caller contracts
and avoid duplicating already-covered behavior: `host/tests/runtime_exit.rs`,
`host/tests/wifi_exit.rs`, `host/src/usb/tests.rs`,
`host/src/usb/routes/retirement_tests.rs`, and
`host/src/monitor/retirement/tests.rs`. Existing automated suites executed: **0**.
Review programs executed: **one custom probe in two final cases**, plus its
initial failure-case run before the successful control was added, followed by
a rerun of both cases after helper extraction. The library
build is compilation evidence, not a test run or platform acceptance.

The probe executes the runtime-owner/route boundary, not the full daemon's
five-second shutdown path, an installed service stop, or real ADB failure.
T1412 is supported by shipped/installed configuration and local systemd manual
semantics; no service was created, stopped, restarted or modified. Native
Windows validation and declined lanes were not resumed. This scoped review
makes no whole-project correctness or native cleanup completeness claim.

## Probe draft complexity verification

The initial fake `execute` function scored 10. Reverse-command handling was
extracted into one cohesive `reverse` helper; both probe cases were rerun with
the same outcomes. Project `scripts/complexity/check.py`, using
`blent-rust-source` 0.1.0 / syn 2.0.119 (source-record schema 1), now reports:
`output` 1, `execute` 4, `reverse` 7, `main` 1. All 4/4 probe functions are <=9;
`probe-complexity.json` retains line locations, source hash and tool versions.
Measured with `/tmp/blent-native-review-metrics/bin/python` importing the
project checker directly. This is scoped probe-source measurement, not a new
production-code or whole-repository complexity claim.
