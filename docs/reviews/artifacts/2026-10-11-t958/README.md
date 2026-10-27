# T958: worker factory and operation ownership

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Factory selects bounded mode and delegates actual metadata to production adapter.
A separate AcceptanceRun owns unchanged execution/cancel/retirement behavior:
blocked execute, clean early return, delayed process death, heartbeat and blocked
native retirement. Preserve validation before admission and original callbacks.
All four functions now score <=9; factory previously scored 15.

Complete disposable worker APK builds offline against current production sources;
provenance retains exact source and APK hashes. T959 permanently compiles these
same fixtures in normal tests. Benchmark coverage exemption remains unchanged;
no installation, device trial or stronger native isolation claim is made.
