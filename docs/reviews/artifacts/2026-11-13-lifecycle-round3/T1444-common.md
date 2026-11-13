# T1444 common command execution boundary

This is the common-command part of the shared T1444 outcome. The host mutation
owner, remote-outcome policy, OSK integration and private D-Bus acceptance belong
to the companion host change; this artifact alone does not complete T1444.

`AsyncCommandExt::output_input_retained_observed(input, observer)` adds explicit
execution without a deadline. The shared execution helper and collector receive
`None` for this path; existing bounded APIs receive their original checked
`Some(deadline)`. Sync/async bounded callers keep their execution deadlines.
No sentinel duration or compatibility path is introduced.

The retained future remains owned by its caller. Dropping it still invokes the
existing native retirement behavior. Output/admission errors can also initiate
retirement. Those outcomes do not certify that a delegated service mutation has
settled: the host mutation owner must keep this future alive after its own caller
wait expires and retain/quarantine unknown remote outcomes as specified by T1444.

## Permanent regression and controls

Added `common/src/commands/retained_tests.rs` before implementation. Its test-local
caller adapter initially selected the existing bounded API; the retained-execution
assertion failed after the five-second command deadline
(`T1444-common-red.log`: retained command expired before original execution
completed). The adapter was then changed to the new explicit retained API, keeping
the original assertions. The existing bounded API was correct for its own
contract; it was unsuitable for the required retained mutation execution. The
companion native delayed-server regression independently demonstrates the actual
production caller defect.

All 34/34 selected `commands::` Rust tests pass after the change, both normally and
with instrumentation (`T1444-common-green.log`, `T1444-common-coverage.log`). The
new outer regression runs six private child scenarios within that count:

- Hold the original command beyond the existing five-second deadline; both
  original proofs remain pending until deliberate release, then output succeeds.
- Deliver exact stdin bytes through the pipe, with two original proofs.
- Reject a missing executable while retaining its already-exposed collector proof.
- Explicitly cancel the execution owner and await both original native proofs.
- Reject output beyond the unchanged byte grant, retaining exact cleanup proofs.
- Reject a second command with collector capacity one before any second native
  exposure; the first command finishes after release.

The child fixture re-executes the native test binary. It opens only its private
files, uses bounded output chunks, and exits if its private root disappears. No
shell, user process, service, device or D-Bus server is used by these common tests.
The held-input interval is a finite test stimulus beyond the existing command
protocol deadline; it does not terminate or truncate work. Original completion is
observed after release before the defect assertion. Child test wrappers wait for
actual exit without an arbitrary elapsed-time kill.

Existing deadline, cancellation, invalid/unrepresentable deadline, sensitive
stdin, byte-budget, collector failure/panic and original-proof cohorts remain in
the 34-test selection. The optional-deadline constructor updates in
`commands/output/tests.rs` preserve their original finite deadlines and assertions.
This is not a whole-crate result; 400 other Rust tests were filtered out.

## Measurement and compilation

All seven changed production functions meet 80% executable-line coverage
individually: six are 100%; `execute_observed` is 30/32 (93.75%). Measured functions
are sync `output_timeout_observed`, async `output_input_timeout_observed`, new
`output_input_retained_observed`, shared `execute_observed`, and collector
`new`, `start`, `collect`. Exact line counters/source/object SHA-256 identities are
in `T1444-common-coverage.json` and `T1444-common-coverage.lcov`.

`T1444-common-complexity.json` measures 62/62 authored functions across the four
changed production/test files, maximum complexity 8. Tool:
`scripts/complexity/check.py`, blent-rust-source 0.1.0/syn 2.0.119.

Normal command:

```text
cargo test --offline --locked -p blent-config --lib commands:: -- --nocapture
```

Instrumented build under `cargo llvm-cov show-env --sh`:

```text
cargo test --offline --locked -p blent-config --lib --no-run --message-format=json
```

The exact emitted test executable was frozen and run with `commands::`,
`RUST_TEST_THREADS=1`, and fresh private profiles. Object/source snapshots, raw
profiles and merged data remain under `/tmp/blent-t1444-common-coverage` outside
the repository. Tools: rustc 1.100.0, cargo-llvm-cov 0.8.7, LLVM
23.1.5-rust-1.100.0-stable.

The default-feature GNU Windows shared-library/test compile passed
(`T1444-common-windows-check.log`):

```text
cargo check --offline --locked -p blent-config --target x86_64-pc-windows-gnu --lib --tests
```

Compiler evidence is not native Windows retirement acceptance; the existing
Windows validation blocker remains.

## Ownership boundaries

The collector reserves its existing bounded pool admission before pipe/thread
creation and reports its original completion before native command spawn. Successful
spawn reports the original child completion synchronously before further awaits.
Both bounded and retained modes use the same owners and handoffs; there is no
second mutable execution state. Borrowed input remains valid for the execution
future's lifetime, and output storage retains the existing per-stream/total grants.

The retained mode removes only execution/collection time expiry. EOF/child exit,
output overflow, I/O failure, panic and explicit owner cancellation retain their
existing cleanup paths and exact proofs. A result remains separate from original
thread/collector completion. Reuse is governed by those original completion proofs,
not a timer or caller result. Conditional eventual cleanup requires original
execution/native retirement to finish; an indefinitely pending operation remains
owned and bounded. Remote side-effect settlement is a separate host-layer contract.
