# T869: bounded identity crash fixtures

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Permanent stalled-checkpoint regression fails with the original unbounded output
wait. A separate five-second production timed owner bounds that red reproduction,
so the regression cannot hang the suite itself. The corrected fixture uses the
existing `SyncCommandExt.output_timeout`: ten seconds per expected crash boundary,
one second for the deliberately stalled child. The child writes its PID first;
the successful test verifies native `/proc/PID` disappearance after timeout,
rather than treating timeout as proof of death.

All 17 identity-store fixtures pass, including every original checkpoint's exit
code 86 and durable identity/pending-state assertion. All 19 functions have
complexity <=9. Only test fixtures changed; no production coverage exclusion or
new production child lifecycle implementation is introduced. No device tests.
