# T849 explicit channel activation

Fresh mux owners enable Management only. Resource channels require an explicit
current-generation activation after the authenticated management commit barrier.
Replacement grants no credit until activation; terminal drain and actual native
consumer/writer retirement remain required. Caller ordering is documented in the
[connection design](../../2026-10-10-t846-connection-contract.md).

Permanent initial-credit and replacement-credit regressions failed before the
change in Rust and Kotlin. Initial red logs used T846 names; new activation tests
were renamed T849 when the parent work was decomposed. Existing T846 regression
assertions remain, with explicit all-channel activation in synthetic fixtures.

Eight Rust mux tests and eleven Kotlin mux/echo tests pass. Native LLVM/JaCoCo
counters pass the 80% per-function gate for all 12 inventoried Rust and 12 Kotlin
functions. Supplemental native Kotlin counters include constructor/accessors and
frame methods: 18 methods, each 100%. Scoped complexity maximum is 8. Synthetic
host example and profile Kotlin compile offline.

This is an activation prerequisite, not the production management broker. T850
and T851 still own typed records and lease/transaction ordering. Historical native
T846 echo trials preceded this gate; they do not qualify production integration.
