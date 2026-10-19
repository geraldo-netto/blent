# T1101: configured broker retirement observation

Parent revision `a00786d1`. The broker CLI previously always observed shutdown
for five seconds. `--retirement-timeout-ms` now selects 1–60000 ms, default 5000.
The portable `retirement_limits::observation_budget` validates positive durations
at most sixty seconds; CLI conversion preserves whole milliseconds. Backend
settings validate before admission and filesystem/socket/native effects. The
same immutable server settings reach its stop workflow and configured shutdown.

Startup and the admitted-server stop workflow have separate cohesive helpers.
The existing explicit-budget shutdown API delegates to the same independent
validator. This introduces no second lifecycle owner, storage, queue, background
thread or native stop implementation. The original server still owns routes,
tasks, authority and quarantine while being observed. Expiry reports failure;
it cannot settle a held native completion or reclaim its admission. Existing
retirement sponsors and T1102 identity consumers retain their original resources.

## Permanent regressions and controls

`t1101_cli_exposes_cleanup_observation_budget` fails on the old CLI and passes
with unchanged assertions. Mapping checks preserve the 5000 ms default and
exercise 1/37/5000/60000 ms. Invalid zero, above-maximum and maximal-u64 values
reach real broker startup and fail before runtime/state directories appear;
malformed/signed/overflow values fail parsing. The pure contract preserves
valid nanosecond precision, rejects exact boundaries and checks 4096 seeded
values against an independent inclusive-range oracle.

The production admitted-server workflow is tested with its original retirement
completion deliberately held and pool capacity one. At 1, 37 and 60000 ms, it
remains pending before the selected timer and reports elapsed at that timer,
while the exact held completion stays incomplete and admission exhausted.
Only the original release allows actual observation/settlement and reuse.
The fixture drops its original server, releases its held observation, and waits
for that completion and the original authority thread's joined completion before
removing private storage, including assertion failure/unwinding.

The test uses an independently requested Tokio timer as its expiry oracle. The
first draft incorrectly expected arbitrary native instants to align exactly
with millisecond timer ticks; installed Tokio 1.52.3 rounds timer ticks upward.
Both initial driver failures are retained. The final test never awaits production
work beyond the independently selected timer. A deliberate hardcoded-five-second
fault fails immediately with `T1101 entry ignored selected cleanup observation
deadline`, proving the corrected oracle rejects the original propagation bug.
No production deadline was expanded to accommodate that draft-test mismatch.

Fresh offline native counters include host/common library tests and the actual
broker executable's local election, signal and restart integration fixture.
**1256 top-level tests pass** (890 host, 364 common, 2 executable integration);
three existing host opt-in benchmarks remain ignored. All **17 scoped production
functions** pass their individual 80% gate, minimum 83.33%.
All 48 source estimates are at most 6. Exact source/object hashes and counters
are retained alongside the report. No physical USB devices or live services are
used. Installed rustc 1.98.1/cargo-llvm-cov 0.8.7 collect an exclusively owned fresh
target. Twelve architecture checks pass. Source estimates use cached
tree-sitter-rust 0.24.2/tree-sitter 0.25.2, not native SonarQube (T1019).

## Bounds and lifetime scope

The pure validator retains no input/view or resource: one immutable Duration is
returned or an owned InvalidInput error is returned. Settings belong to one server
from validated admission through destruction. A longer observation budget keeps
the caller waiting longer; a shorter budget reports unresolved cleanup sooner.
Neither changes device stop guarantees, per-I/O budgets or original admission.
Existing quarantines remain funded until exact native evidence permits reuse.

The held-observation test checks the portable completion/admission contract,
not an arbitrary blocked native syscall. Real Linux store/thread cancellation
and TLS consumer lifetime evidence remain T1100/T1102 and existing native shutdown
regressions. Eventual cleanup assumes original work/destructors return and the
reaper is scheduled; a configured timer is never proof of those assumptions.
Unrestricted native safety, physical tablet acceptance and Windows support are
not claimed. Host resource-admission settings and joined parity remain T1103/T898.
