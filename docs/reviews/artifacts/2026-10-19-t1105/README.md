# T1105: original broker link owns selected TLS setup budget

After T1104's independent TLS deadline contract, `--tls-timeout-ms` now exposes
1–60000 ms, default 10000, in the Linux broker. CLI and server defaults derive
from that same contract. Raw settings validate before admission/filesystem effects.
The backend maps one validated immutable value into the original link settings;
the link passes it through Candidate's READY/TLS workflow to the stock adapter.
There is no missing-value fallback or second TLS implementation.

Human approval/READY coordination finishes before the selected TLS/preface
operation budget starts. Discovery, window I/O, resource admission and retirement
observation retain their separate contracts. TLS expiry cannot settle original
native helper admission or release T1102 identity consumers. The default-policy
Candidate convenience method explicitly supplies the unchanged defaults.

## Permanent evidence

The unchanged CLI test fails before the option is added and passes afterwards.
A second real behavioral regression is staged with the new mapped value while the
link still incorrectly passes the default: selected 37 ms accepts a peer delayed
100 ms. It fails only after original resource/task cleanup. Forwarding the original
selected value makes the same test pass. A separate 100 ms READY approval wait
then succeeds under a fresh 37 ms TLS budget, proving these stages stay independent.
An initial draft test called unwrap on a unit-returning fixture; that compiler
failure is retained separately and is not behavioral red evidence.

Actual broker link tasks run stock mutual TLS over bounded duplex streams.
Their original native-memory admission and owned-task completion are captured
before activity, then observed after stop/drop and before outcome assertions,
including unexpected Verified results on faulty paths. The fixture resumes real
time before observing the independent real retirement reaper. A deliberate fault
inside Candidate that replaces its selected forwarding with the default is also
rejected by the normal broker test; original sources are restored in finally.

CLI mapping exercises 1/37/10000/60000 ms, malformed/signed/overflow values,
zero, above-maximum and maximal-u64 input. Real startup rejects invalid budgets
before runtime/state directories appear. Backend mapping preserves a retained
old snapshot and original identity while future settings change, rejects invalid
durations, and proves unrelated stage budgets retain their defaults.

Fresh offline native collection passes **1274 tests** (902 host, 370 common,
2 executable integration), with three existing host opt-in benchmarks ignored.
All **29 scoped production functions** pass the individual 80% gate, minimum
90.91%. All 102 source function estimates are at most 9. Exact sources/objects,
commands, LCOV and individual results accompany this report. Installed rustc 1.98.1 and
cargo-llvm-cov 0.8.7 use a fresh exclusively owned target. Twelve architecture
checks pass. Source estimates use the cached pinned parser; native SonarQube
measurement remains unavailable under T1019. No thresholds or tests are weakened.

## Ownership and bounds

One immutable timeout scalar is copied into each existing link task; no new
pool, thread, queue, native handle or callback is created by this setting.
Partial/invalid mapping drops already-owned fields without publishing a view.
The server retains its authoritative identity, and every published key/config/
signer keeps its original guard. Original streams, setup candidates, task leases
and native owners follow existing success/error/cancellation/timeout retirement.
Logical completion and reference counts do not stand in for native fences.

Fixtures contain 8192-byte duplex channels and bounded certificate records. All
successful/unexpected verified resources and original tasks are dropped/stopped
and joined/observed before assertions. They check hosted policy/task ownership;
they do not model an arbitrary blocked device syscall. Existing real Linux
store/native-thread tests remain complementary evidence. Conditional eventual
cleanup assumes original native work/destructors return and existing reapers and
joins progress. Physical USB/Android acceptance and unsupported backends remain
unresolved. Android's native timeout also covers other setup stages; the host
TLS knob is deliberately scoped to TLS/peer-proof/preface, with other budgets
explicitly separate. Joined software option acceptance remains T898.
