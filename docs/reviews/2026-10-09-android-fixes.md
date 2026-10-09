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
