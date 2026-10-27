# T1065: original local worker settlement

The local Binder fixture reports worker death only after original execution and cleanup settle. Tests cover held cleanup and late creators; this fixture cannot emulate forced OS death of an uncooperative native thread.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The shared test Binder previously published death directly after scope cancellation.
The held-retirement regression fails on SDK 27 and 34 before this fix. EXIT now
waits for the actual original execute job, its independent cancel call and normal
native cleanup before notifying the original death recipients. Cleanup failure
retains the original execution and denies replacement. Late creators remain owned
and retire without executing. Surface ownership settles even on immediate exit.

A second retained faulty control proves malformed Surface markers were accepted;
the permanent boundary test rejects them and nonpositive/reused epochs. Six
regressions on each SDK cover held/failed retirement, stale/overlapping generations,
late creators, invalid inputs and 32 immediate exits. Six fixture-consumer classes
pass together. Existing five worker components receive independent local bridges.
The fixture cannot emulate forced OS death of an uncooperative native thread;
unknown cleanup never becomes a fictitious death fence. Physical evidence remains
T863/T914. No production function or native platform-support claim changes.

Pinned cached Kotlin PSI 1.9.20 audits 97 functions/callbacks, maximum source
complexity estimate 5. Actual SonarQube evidence remains unavailable under T1019.
Logs, faulty controls and source hashes are retained. Full-suite context includes
uncommitted T1060 integration; the collection exposed an independent audio-fixture
ArrayList race recorded as T1066. No network or live-device operation occurred.

</details>
