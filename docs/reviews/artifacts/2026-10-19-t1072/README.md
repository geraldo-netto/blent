# T1072: warning-free Android unit fixtures

Removed an unnecessary lateinit and unused local. Required native shadow
signatures retain narrowly documented unused arguments. Intentional legacy
AudioRecord, Wi-Fi and Back test entry points retain scoped deprecation
annotations. The existing suppressed orientation helper now qualifies its
deprecated fixture type instead of importing it outside that scope. Native
signatures, assertions, resource ownership and production behavior are unchanged.
These are compiler-warning annotations, not metric or coverage suppressions.

The preceding clean collection had 32 warnings. Final clean offline compilation
of all unit sources has zero warnings/errors; all 118 affected tests and ordered
UI lifecycle regressions pass with zero failures/errors/skips. The first focused
run passed108 but failed eight pairing setup cases; T1076 independently fixes that
sandbox-order bug with permanent red/green coverage. Its complete original/final
JUnit evidence is retained in the sibling T1076 artifact. No full-suite rerun or
new production coverage result is claimed from this focused collection.

Source hashes match final frozen inputs. Kotlin PSI1.9.20 source audit covers369
fixture functions/callbacks, maximum estimate8; native SonarQube remains T1019.
No assertion was weakened and no new exclusion, resource owner, queue, callback
lifetime, native allocation or platform capability was introduced. Existing
fixture cleanup and normal regression ownership remain unchanged. No download,
network transfer or device interruption occurred.
