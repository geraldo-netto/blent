# T1165 — primitive state with preserved equality

Integer FPS state uses primitive Compose storage while float state preserves its existing equality semantics. Settings tests and counters passed; no performance or battery improvement was measured.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Use installed Compose1.6.3 mutableIntStateOf for the FPS choice. Primitive integer
comparison matches boxed Int structural equality for the full Int domain, so
existing selection, keyed remember replacement and Apply semantics are preserved.

Retain boxed Float state for stream FPS/bandwidth and bitrate draft. Installed
runtime bytecode shows primitive float state compares with JVM floating equality
on supported SDKs: NaN is unequal and signed zeros equal. StructuralEqualityPolicy
uses boxed Float equality: NaNs equal and signed zeros distinct. The observation
contract and callback types accept Float; replacing them would change observable
state semantics without a narrower independently enforced input contract. Comments
record why the three advisory float recommendations remain unsuppressed. This
review does not silently redefine invalid observations or callback inputs.

The existing settings, accessibility, layout, Activity and display-settings suites
pass offline; exact totals follow. They retain invalid/range preference bounds,
selected FPS/bitrate Apply, settings replacement, rejection restoration and view/
receiver ownership. All40 production functions/callbacks in SettingsSheet meet80%
individually, including every native method variant. Sources stayed frozen through
native collection/reporting. The source audit remains at most9; exact per-function
PSI1.9.20 rows follow, native Sonar still T1019. Lint has no errors and removes the
integer AutoboxingStateCreation advisory. APK assembly passes.

The draft remains composition-owned, keyed by the authoritative settings FPS and
collected with the composition; event handoff remains immutable SettingsEvent.
No native resources, external callbacks or retirement obligations are added.
No measured CPU/battery improvement, device validation, download or push is claimed.

</details>
