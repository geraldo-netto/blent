# T1260: retain cleanup admission through native worker exit

The permanent blocked-TLS regression fails against the original implementation:
completion and capacity reuse occur while the original cleanup worker is still
inside its TLS destructor. The corrected implementation preserves the original
admission through the actual native join. Sibling cleanup continues independently.

The existing independent Observe contract still requires a nonblocking poll and
actual native completion. Cleanup has one authoritative worker phase: Idle,
Running or Joined. Spawn failure leaves the original closure owned and retries
with the existing backoff. Work and error-reporting outcomes are separately owned
by the admitted Shared record. Only successful work/reporting plus a joined
native worker can return completion. Panic or unknown evidence retains capacity.

Linux transfers the join right of the protected cleanup worker and uses
pthread_tryjoin_np. A successful join consumes that exact right once; subsequent
state cannot address a reused thread identity. Native errors retain ownership.
The raw adapter has a detach boundary for an unsubmitted owner; its permanent
fixture contains that boundary inside a separately owned/reaped child process.
Windows retains its handle and checks WaitForSingleObject with zero wait before
joining. Unsupported application platforms retain their owner without claiming
native completion; existing platform support labels remain unchanged.

Arbitrary Rust JoinHandles stay in the ordinary Thread adapter. Its is_finished
check only admits isolated join work; it never releases capacity. The actual join
and any result/panic-payload destruction run inside Cleanup's protected worker.
That worker catches both work and reporting panics, stores their payloads in the
funded record, and returns only unit. No application panic payload is dropped by
transferring the protected worker's raw handle. This is a necessary restriction:
payload-red.log.gz records a contained failing control from the initial draft,
where transferring an arbitrary handle let its panic-payload destructor abort the
child. The retained permanent regression now passes without weakening its oracle.

The record owns the closure, native join right and at most two panic outcomes.
At most one monitor is admitted per record. Joined success settles those resources
before releasing the slot; unknown cleanup stays bounded and funded. Native TLS
teardown is included in the exit fence. Conditional eventual cleanup requires
successful spawning, eventual work/TLS/report completion and fair reaper polling.
Caught work/reporting failures deliberately quarantine the record. Process crashes
and Rust's fatal runtime aborts remain process-lifetime boundaries; these tests
are not an unrestricted safety proof for arbitrary native code.

## Verification

- All 408 common-library tests pass, including seven T1260 boundary cases and
  existing panic isolation. Cases cover blocked TLS, capacity reuse, sibling
  progress, nonblocking/repeated polling, spawn failure/backoff, worker panic,
  panicking result destruction, reporting failure and unsubmitted-owner cleanup.
- All 117 GUI tests pass, including the private native startup fixture.
- The selected all-feature host cohort passes 1462 cases and fails one T908 USB
  startup test; five pre-existing opt-in benchmarks remain ignored. An unchanged
  rerun of the entire main-binary cohort passes all 392 cases. The intermittent
  failure is preserved under new open item T1267; its cause is not established,
  and the failed cohort is not represented as green. All 1018 library tests,
  audio/broker binary tests, 23 mapping-retirement tests, both camera-probe targets,
  all three native-audio tests and Linux lifecycle tests pass in the recorded run.
- Each of 11 measured Linux/shared functions meets the 80% executable-line gate
  (minimum 88.89%); raw LLVM LCOV and per-function results are retained. The unused
  repeated-poll branch in the private native adapter remains visible as uncovered.
- Local cyclomatic measurements cover all changed Rust production/test sources,
  including platform adapters, and stay <=6. Locked syn 2.0.119, rustc 1.98.1,
  cargo-llvm-cov 0.8.7 and LLVM 22.1.8 were already installed/cached.
- Common library and tests cross-compile for x86_64-pc-windows-gnu. Native Windows
  execution and per-function counters remain unavailable and blocked under T1266;
  compilation is not acceptance. Portable common Clippy passes with warnings
  denied. The unrelated pre-existing EVDI lint is outside that portable scope.

Commands and SDK paths are retained in the logs. Everything ran offline/locked;
no real ADB/tablet, downloads, uploads or pushes were used. Production source hashes
bind the measurements. The existing original red log and the additional faulty
control are both retained; the final native implementation contains neither bug.
