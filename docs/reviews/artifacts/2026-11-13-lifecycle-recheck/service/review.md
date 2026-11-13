# Service/process-exit recheck — 2026-11-13

Reviewed HEAD `9f8660aa3f0b9c01f78142268166947c2f50f6ff`. Review-only:
production/test/TODO files, installed configuration, running service and tablet
were not modified by this reviewer. Parent owns ledger entries. Three confirmed
findings were reported immediately and recorded as T1417, T1421 and T1423.

## Findings

**T1417 — stale service-deadline contract and failing existing regression**
(low severity, small effort). `common/src/commands.rs:34–43` still describes
`SERVICE_STOP_TIMEOUT=15s` as matching the units and derives the GUI response
budget from ExecStop plus service shutdown. Both source units now omit ExecStop
and use `TimeoutStopSec=infinity`. The actual 35-second client response budget
can remain bounded independently; the documentation/model must stop claiming
it equals server cleanup. The existing normal-suite regression
`common/src/commands/tests.rs:104–119` parses the unit value as `u64` and fails
on `infinity`. Executed exactly:

```
cargo test --offline --locked -p blent-config --lib commands::tests::t268_lifecycle_deadlines_cover_cli_and_shipped_service_budgets -- --exact --nocapture
```

`T1417-test.log`: 0/1 selected tests passed; `ParseIntError { kind: InvalidDigit }`
at line 117. Update current budget naming/comments and the permanent regression
to require bounded client responses plus retained service cleanup. Do not restore
a forced service deadline merely to satisfy the old assertion. This is one
cohesive current-contract correction.

**T1421 — original activity-source retirement is outside the process fence**
(medium severity, small effort). `host/src/daemon_activity.rs:12–16` normally
waits for actual `Following::retired`. `host/src/linux_main.rs:1873–1885` places
that wait inside a five-second join and drops it at expiry.
`host/src/activity_follow.rs:99–116,148–158` owns the source through a separate
pool/`SourceLifetime`; neither its original completion nor its native dependencies
enter `process_retirement::PROCESS`. `runtime_owner.rs:11–14` therefore sees a
settled session/route registry, destroys Tokio and returns even when this admitted
activity owner is pending. This is a missing independent owner, not a repeated
claim that T1411's session/route fence is ineffective.

The public-API `activity_exit_probe.rs` constructs a fake SourceFactory whose
source retains an original native thread completion, performs the same five-second
controller wait, drops the Following facade, then uses the real runtime owner:

- Held native source: runtime returned after 5006 ms with `native_complete=false`.
- Settled control: runtime returned after 72 ms with `native_complete=true`.

Both cases explicitly release and join their owned native thread afterward;
`activity-exit-result.txt` confirms cleanup. This reproduces the source/runtime
boundary, not a real D-Bus/DPMS failure or the complete daemon orchestration.
Complete by retaining the activity's original native fence through process exit,
including partial startup/error/cancellation, with permanent subprocess coverage
for cleanup extending beyond five seconds and a settled control. Native failure
must remain owned rather than being equated with timeout completion.

**T1423 — SIGTERM can terminate admitted startup work before handlers exist**
(medium severity, small effort). `host/src/linux_main.rs:1782` starts the primary
session and line 1854 starts the ADB monitor. Lines 1856–1865 print the startup
banner. Only line 1867 polls the shutdown future, installing SIGTERM at line 1918.
A normal stop arriving in this interval takes the kernel default action and
never enters the new process drain. The service's direct SIGTERM stop path does
not remove this startup window.

A contained probe ran the freshly built default-feature **actual daemon**, with
native device nodes masked by a private `/dev` tmpfs, all input disabled,
`/bin/true` as its capture helper, no graphical display, invalid private D-Bus
addresses, separate HOME/config/runtime/state, and a file-backed fake ADB. The
stdout pipe is filled before spawn, so the first startup print pauses main after
primary/monitor creation. Fake ADB confirms two original admitted routes. The
probe reads the owned process's SigCgt mask before signaling:

- Startup barrier: SIGTERM handler absent; SIGTERM exits `-15`; 2/2 fake routes
  remain in the private fixture.
- Settled control: drain stdout first and confirm handler installed; the same
  SIGTERM exits `0`; 0/2 fake routes remain.

`startup-signal-result.txt` records both final cases. Exact execution:

```
cargo build --offline --locked -p blent --bin blent
unshare --user --map-root-user --mount --pid --fork --mount-proc /usr/bin/python3 docs/reviews/artifacts/2026-11-13-lifecycle-recheck/service/startup_signal_probe.py /backups/disk2/projects/blent/target/debug/blent
```

`startup_signal_probe.py` requires namespace PID 1, mounts a private tmpfs over
`/dev`, and removes each owned temporary fixture after reaping its child.
`fake_adb.py` never connects to a real server; its effects are two JSON entries.
Failed initial fixture setup trials were contained and cleaned as well; the final
reported pair uses second-screen mode with all input disabled. No stylus lane was
resumed. Complete by registering/retaining shutdown signals before owned startup
effects, preserving early requests through initialization and partial-startup
cleanup, and adding a permanent startup-barrier regression plus post-handler
control. A buffered early request must not be lost when startup resumes.

## Verified boundaries and rejected candidates

The current session/display-route process registry has an explicit 8192-proof
bound, atomic cutoff versus admission, and live-runtime drain; no counterexample
to those corrected paths was found here. The shipped mixed/no-SIGKILL/infinite
service policy preserves pending owners, while client command deadlines remain
separate. Media CLI ordinary/error returns await their native reports; this
review did not establish an ordinary-return omission in those paths. Direct CLI
stop requests SIGTERM and reports a bounded-wait failure without its own SIGKILL.
Missing PID files alone do not admit a duplicate daemon: `ensure_single_daemon`
also scans same-user process identity. Two already-executing Starts are visible
in that inventory before either PID publication; a non-atomic PID write alone
is not evidence of duplicate native admission.

Primary paths examined: both service templates; `runtime_owner`,
`process_retirement`, `linux_main`, `session_supervision`, `daemon_activity`,
`activity_follow` and its recovery, common source lifetime, `owned_task`, Wi-Fi
attempt ownership, common command/daemon/process adapters, camera/audio CLI
normal/error cleanup, broker entry, monitor discovery/forwarding and existing
startup/stop regression contracts. This is a scoped process/service recheck,
not an exhaustive scan of every function in those files.

## Verification scope

Existing automated suites: **one exact existing test executed, 0/1 passing**
(T1417); no test-quality campaign or full-suite claim. Supporting programs:
**two isolated probe programs**, each with one failing-boundary case and one
settled control. Fake ADB is the startup probe's adapter, not another independent
behavior test. Builds: current host library with accessory support for the
activity probe, and default-feature daemon for startup verification.

Tools: rustc 1.100.0, project `scripts/complexity/check.py` with Python AST 3.12.3
and locked Rust source frontend/syn 2.0.119. `probe-complexity.json` records source
hashes and all 17/17 probe/helper functions at complexity <=9. Compiled probes,
Cargo build objects and transient namespace files are outside repository evidence.
No Windows runtime acceptance, installed-service update, physical tablet,
physical display, suspend/resume or actual native-device qualification is claimed.
