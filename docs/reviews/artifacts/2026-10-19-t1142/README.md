# T1142 isolate collector response measurement

Collector responsiveness is measured after unrelated process setup finishes. A delayed-preparation regression passed while preserving the original response and cleanup assertions; production timeouts did not change.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The fixture now prepares/reaps its unrelated `--list` process before admitting
or starting the held collector. The measured control path retains the original
100 ms collection budget, 25 ms result timeout and 200 ms response bound.
The async case still creates no unrelated executable. No production code or
protocol timeout changes.

The new normal-suite regression deliberately delays preparation350 ms. It fails
before the phase correction, then passes with the identical test and cleanup/
response assertions (hashes retained). All seven output-related tests pass.
Exact original admission stays held while Read is blocked; release and worker
join still precede assertions, followed by actual original collector completion
and pool reuse. Preparation has no collector admission; its native process is
synchronously reaped before returning its ExitStatus. No borrowed view escapes.

This proves the fixture separates preparation from collector responsiveness.
The first T1130 historical failure did not retain startup timing, so this is not
attribution of every past failure or a new production liveness claim.
Bounded scheduler/native-execution progress remains a test-domain assumption.
Pinned source audit is <=9; actual SonarQube remains unavailable under T1019.
Production-function coverage is unchanged; this item modifies tests only.
No Windows/EVDI edit, live-device action, installation or network transfer.

Full normal common library:384 passed. Source audit:14 test/helper functions,
maximum6. Cargo workspace formatting passes.

</details>
