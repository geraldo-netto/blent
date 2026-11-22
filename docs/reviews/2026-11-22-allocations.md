# Android connection allocation review

## T1765: reusable scheduling credit snapshot

ConnectionMux now keeps one six-element credit snapshot and refreshes every entry
from its authoritative flows before calling ConnectionQueue.next. The queue borrows
that array only during the synchronous call and retains no reference. The existing
receive-credit accounting array remains a different obligation; it is not a second
owner of send-credit state. The snapshot adds 24 bytes of fixed payload storage per
mux, plus runtime array headers, in exchange for removing an allocation per eligible
scheduling pass. No wire format, queue capacity, frame ordering or user default changes.

One serialized mux owner allocates and retains the snapshot for its lifetime.
No callback, thread, native loan or endpoint obtains it. Refresh happens only after
the prior frame reservation has finished; queue selection does not mutate or retain
the view. Exceptions leave storage with the mux, and owner reclamation releases it
with the rest of the managed mux state. There is no pool, recycled identity or native
settlement added here. The conservation obligation is one snapshot per mux and no
escaping mutable reference; only its current flow-derived values affect selection.

A hosted current-source benchmark measures 14 trials before and 14 after: four
warmup and ten measured trials per run, five each for idle scheduling and 64-byte
data transfers, 200,000 passes per trial. The finite fixture checks idle quiescence,
exact data-array identity/length and release of the original frame before reoffering
its payload. Each data pass includes a fresh peer-credit header; both runs use the
same fixture. Initialization is outside timing. Thread allocation/CPU counters cover
the driver; elapsed time also includes scheduling and collection pauses.

| Case | Before allocation/pass | After allocation/pass | CPU ns/pass before → after | Wall ns/pass before → after |
| --- | --- | --- | --- | --- |
| Idle | 40.00084 bytes | 0.00084 bytes | 26.43 → 21.89 | 26.37 → 21.83 |
| 64-byte data | 184.00084 bytes | 144.00084 bytes | 54.13 → 52.49 | 53.93 → 54.36 |

The measured difference is exactly 40 bytes per pass in this JVM. The 0.00084-byte
residue is fixed per-trial measurement overhead, not an assertion of zero allocations
in the entire connection process. Data-case allocation falls about 21.7% in this
isolated fixture; this is not a whole-application memory reduction. CPU is slightly
lower for data while wall time is essentially unchanged. Medians from separate runs
are local point estimates, not causal confidence intervals or native ART evidence.

The existing 42 queue, flow and tablet-session cases across eight classes pass;
all 24 measured production functions across ConnectionMux and ConnectionQueue have
100% executable-line coverage. Profile Kotlin compilation passes. A deliberately
stale snapshot control makes the permanent metadata/credit regression fail; restoring
the real refresh passes that same regression. This optimization fixes no previously
incorrect flow behavior: existing permanent safety oracles remain unchanged.

All 19 restored-regression/benchmark cases pass, including the existing 16 tool
cases and two new finite allocation-fixture cases. The repository checker measures
273 Kotlin/Python functions (maximum complexity 7); Groovy 3.0.17 compiler AST
measures four task DSL closures (maximum 3). Benchmark line coverage is exempt.

Evidence: `artifacts/2026-11-22-allocations/t1765/`. Native Android allocation,
retained/peak process memory and end-to-end acceptance remain in T1527, alongside
producer-buffer, native-receipt and actor-wait work. No installed session was changed.
