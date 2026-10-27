# T1161 — composable naming

The input-controls composable follows the required naming convention without changing behavior. Existing UI/lifecycle tests, full-file counters and lint passed.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Rename SessionCoordinator.inputControls to InputControls and update its single
production caller in MainActivity. This is the Unit-returning Compose naming
convention; the input control implementation, consent and retirement are unchanged.
No regression was invented for a name-only change. Existing DirectInput, session,
settings, lifecycle and ordered Activity/UI/connection tests pass unchanged.

All 135 production functions/callbacks in both complete source files meet 80%
individually, including every mapped native method variant. The initial selection
passed the changed functions but omitted three unrelated file functions; adding
the existing SettingsLayoutTest suite brings the complete file scope to 135/135.
Source inventory stayed frozen through collection and reporting. Native counters,
manifest, JUnit, exact outer totals and the nested original ordered oracles follow.
The cached Kotlin PSI1.9.20 source audit measures142 functions, maximum8; native
Sonar remains T1019. Offline lint has no errors and no ComposableNaming advisory;
APK assembly passes. Existing reviewed advisories remain unsuppressed.

Ownership is unchanged: Activity SessionCoordinator owns the view workflow,
InputControls borrows its TouchCapture during composition, and the existing
control/resource owners retain their cutoff, native retirement and cleanup duties.
The rename introduces no allocation, queue, identity or reuse boundary. No live
device, download or push occurred.

</details>
