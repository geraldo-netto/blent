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
