# Shared native idle advice and bounded FIFO checks

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

T1119 extracts Linux advice validation into `host/evdi/idle_control.{h,c}`.
The independent header defines the five-field, original-source, short-lived
500/1000 ms contract and 0 result for missing/invalid advice. The writer retains
its 200 ms compatibility default. Configuration still cannot enable 1000 ms;
joined software qualification and explicit settings remain T1115/T1111.

A cached writer checks advice at min(frame deadline, now + 200 ms). This targets
a bounded condition-wait deadline, not a hard real-time wall-clock guarantee
across scheduler or filesystem delays. Intermediate checks release the exact
frame lease without sending NV12; fresh publication wakes the condition and
bypasses sparse delay. First frame and original stop/retirement remain unchanged.

Validator owns one newly opened nonfollowing CLOEXEC descriptor until close on
every path; the caller owns the original source descriptor and optional path
borrow through the synchronous call. Five fields and at most 159 accepted bytes
use stack storage; no allocation, global state, callback or retained view. A
result is advisory observation, never native retirement. The writer holds each
original generation-tagged frame across pacing/partial write and releases it
before storage retirement; timeout never authorizes freeing held storage. Native
joins and existing retirement regressions remain the actual release boundaries.

[Evidence](artifacts/2026-10-19-t1119/README.md): red/green regressions, faulty
controls, 65 normal native tests, per-function native counters and source audits.
T1120 now pins and rejects preexisting advice before the native writer first
accesses its source; see [original-reader admission](2026-10-19-native-idle-admission.md). T1121 now rejects
embedded NUL and hidden trailing bytes using an exact consumed-cursor comparison;
its [native evidence](artifacts/2026-10-19-t1121/README.md) retains red/green and
8,192 bounded hidden-suffix cases plus prefix mutations. No unrestricted lifecycle or hardware claim.
