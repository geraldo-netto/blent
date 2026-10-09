# Android review implementation

## T829: decoder replay generator prerequisite

The current-source offline decoder benchmark failed to compile because its
source list omitted `DecoderDiagnostics.kt`, while DecoderSession already used
its types. `test_decoder_diagnostics_dependency.py` failed before the source-list
fix and passes afterward. Generated `compileDebugKotlin` passes offline. This
repairs the benchmark only; production behavior is unchanged.

## T822: independent wire contracts

Protocol limits, formats, endpoint validation, selection values and standard
profile vocabulary now live independently of receiver/codec/Intent owners.
JSON, Intent and MediaCodec mappings are explicit adapters. Packet diagnostics
are injected by the receiver; packet readers contain no Android calls. Native
capability checks and existing wire behavior remain intact. The standalone
benchmark copies the extracted contracts and selects the matching JSON adapter.

Permanent architecture regression failed against the original dependencies and
passes after extraction. Plain-JVM contract tests cover payload borrowing,
selection receipts and invalid identifier/profile/endpoint boundaries. The wire
contracts/readers also compile with only Kotlin/JDK, without Android or stubs.
Offline Android suite: 731 tests, zero failures; JaCoCo/PSI gate: 786/786
maintained methods reach at least 80% individually. Production Kotlin cyclomatic
complexity remains at most 9. Generated decoder Kotlin compilation and benchmark
source-dependency regressions pass. No native installation or download.

## T810: portable resource lifecycle policy

Audio/camera bindings now depend on independent endpoint/preferences/state values
and a LifecycleRun interface. Android construction adapters own Context,
permission checks, native capture/resource types, services and preference storage.
Compose observes immutable StateFlow snapshots. Cancellation remains distinct
from confirmed retirement; generation checks and borrowed native lifetime gates
remain. Camera rotation is sampled at actual execution, after earlier retirement.

The architecture regression fails on the previous Android/Compose dependencies
and passes now. Plain-JVM lifecycle tests exercise resource delegation, state,
late status rejection and Stop; policies also compile against Kotlin/coroutines
without Android or Compose. The full offline Android suite passes, including
existing ownership/permission/service/recreation tests. All 813 maintained
Android methods pass individual 80% coverage; production complexity stays <=9.
T807/T806 retain the known cleanup-error and unbounded-waiter behavior to fix next.

## T807: contained cleanup failures and uncertain retirement

Independent Activity teardown actions continue after ordinary failures. Audio
and camera service-stop errors remain visible in their owner state without
escaping a root coroutine; unconfirmed background-service identity is retained.
Native camera close attempts retain reverse order, report incomplete cleanup,
retain uncertain handles and do not automatically retry their destruction.
A portable retirement guard keeps a failed native owner reachable and rejects
replacement. Atomic state updates preserve concurrent cleanup observations.

Decoder retirement had the same admission flaw: swallowed release errors removed
the process-wide gate. Failed MediaCodec release now retains that owner/gate;
confirmed release after a failed stop still permits replacement. Fatal errors
are not consumed by the new cleanup boundaries.

Eight new owner/Activity/native-close regression cases failed before the fix.
The added codec test failed on both SDKs when release failure reopened admission;
normal release controls already passed. All now pass. Existing reverse-order,
late-startup and T707 tests retain their cleanup assertions and additionally
require visible errors and contained exceptions. Plain-JVM tests cover admission,
late/interrupt failures, independent cleanup and fatal-error propagation.
Full offline Android suite: 749 passing tests; 834/834 maintained methods pass
individual 80% coverage; production complexity remains <=9. No physical native
failure was induced. T806 still owns bounded requests/deadline reporting and
T798 still owns the UI retirement wait.

## T806: bounded admission and observable native waits

Audio and camera share a portable ResourceLane: one active/retiring owner and one
latest pending request. Superseded requests allocate neither native owners nor
predecessor jobs. Stop discards the pending request; only confirmed retirement
admits a successor. Failed cleanup retains the existing quarantine. A five-second
independent deadline reports stalled cleanup without freeing native memory.

Decoder startup/retirement now publish process-wide pending/stalled observations,
visible across presentation recreation. Camera callback leases share a startup
deadline across recovery attempts; missing callbacks remain visible even before
Stop. The last genuine callback return clears the startup wait. Camera destruction
stays on the Android adapter's I/O dispatcher. Cleanup deadlines are notifications,
not guarantees that an in-process native hang or crash can be recovered.

The two original storm regressions failed before the change. They now exercise
128 replacements, latest-request selection, Stop/shutdown, bounded child jobs
and no queued resource construction. Separate regressions failed for unobservable
decoder waits on both SDKs, missing camera callback deadlines, and Stop hiding an
already-stalled owner. Permanent tests also cover pre-dispatch cancellation,
failed construction/destruction, timeout bounds, repeated lease returns and
progress of an independent lane while another remains stalled. Existing T707,
T732 and cleanup-failure regressions remain enabled.

Offline validation: 767 tests pass; all 864 maintained Android methods reach
80% individually; production cyclomatic complexity stays <=9. Generated decoder
Kotlin compilation and source-boundary/dependency tests pass. No downloads or
physical native hangs were induced. T798 still owns the UI retirement wait.
