# T1139 successful startup versus timeout observations

Success-path test fixtures allow realistic executable startup while explicit timeout controls keep their short budgets. A controlled 350 ms startup regression passed; it does not diagnose every earlier timing failure.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The native input-success fixtures now admit a two-second startup/completion
allowance. The intentional wait/missing-command cases keep100 ms; the existing
cancellation observer still uses its original100 ms outer/10 s inner budgets.
No production command timeout or native ownership code changes. Diagnostic
assertions now retain the exact result when a success expectation fails.

A permanent normal-suite test gives the input helper a deliberate350 ms startup
delay. Before the fixture allowance correction it fails with TimedOut. The same
unchanged oracle now verifies successful byte delivery with the success allowance
and TimedOut with an explicit100 ms negative control. Both paths retain exactly
two original child/collector proofs and wait for their actual cleanup before
asserting; temporary paths stay owned through settlement. Input is a fixed22-byte
test payload, and helper lifetime/delay is bounded by this test domain.

This corrects a fixture assumption about successful executable startup, without
claiming a100 ms host startup guarantee or weakening timeout/proof checks.
The first historical T1130 failure retained only is_ok=false; its exact error
was not captured, so the new controlled reproduction is not attribution of every
historical event. A retry is not treated as a production fix.
Pinned source audit is <=9; actual SonarQube remains unavailable under T1019.
Only test code changes, so production-function coverage is unchanged.
No Windows/EVDI edit, live-device action, installation or network transfer.

All7 observation tests and all385 normal common-library tests pass.
Pinned source audit checks33 test/helper functions, maximum6. Cargo formatting
passes. No permanent regression is skipped or removed.

</details>
