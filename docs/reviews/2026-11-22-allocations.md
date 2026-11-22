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

## T1767: native request and receipt allocation

Keep fresh immutable native request/receipt objects. The hosted plaintext path's
entire driver and three I/O workers allocate about 458 bytes per payload through
the production local consumer, or about 410 bytes through the direct native
transaction control. Every payload includes an ACK read. Both modes use original
StreamConnectionIo workers; initial payload/connection/owner allocation and the
peer thread are excluded. The median total is constant across 64-byte, 4-KiB,
16-KiB and 64-KiB payloads, in burst and 60-fps modes. The difference is 48 bytes
of consumer bookkeeping per payload. At 60 payloads/s, the entire measured
production-consumer region allocates about 26.8 KiB/s, before removing fixed
counter-sampling overhead. This is an upper bound on request/receipt allocations
in that region; there is no per-class allocation attribution.

Each fresh request/loan identity is part of rejecting foreign, duplicated or late
completions. Pooling those identities would require a new independently checked
generation/settlement contract and bounded retained state. The small measured
upper bound does not justify that change now. This is a measured decision to
retain the existing policy, not an implemented memory optimization or proof that
all allocation rates are harmless. Producer arrays and actor waits remain separate
T1766/T1768 work, including workloads outside this one-outstanding-payload domain.

The same counters also measure actual JSSE TLS framing and native I/O:

| Payload | Allocation/frame at 60 fps, no gather | Allocation/frame at 60 fps, gather ≤16 KiB |
| --- | --- | --- |
| 64 bytes | 4623.7 bytes | 3365.5 bytes |
| 4096 bytes | 4828.8 bytes | 3572.5 bytes |
| 16384 bytes | 6764.0 bytes | 5352.0 bytes |
| 65536 bytes | 10114.9 bytes | 10114.3 bytes |

These totals include library/framing allocations, not just Blent receipts. Gathering
reduces small-frame allocation, but remains disabled by default under T1762. At
64 KiB, both modes bypass gathering and allocate essentially the same amount inside
the timed region. That does not rule out effects from the fixed scratch allocated
before measurement, JIT, collections or scheduling. This run's 64-KiB gather/no-gather
CPU and p95 ratios are 1.015/0.980 in burst mode and 1.118/1.189 at 60 fps, further
showing that the earlier slowdown is not a universal per-frame allocation increase.
T1762 retains the unresolved investigation and the conservative default.

MeasurementSnapshot now collects CPU and allocated bytes together from the original
four threads. Any missing counter fails rather than becoming zero. Its temporary
list and counter records are bounded by those four identities and do not escape
the sample. Connections, streams, original workers and peer cleanup retain their
existing exact ownership. The shared result schema requires allocation data; there
is no compatibility reader for historical result formats. Historical summaries and
source hashes remain unchanged. These are cumulative allocations, not retained or
peak memory, collection counts, leak evidence or Android ART acceptance.

All 18 hosted cases across five classes pass, including positive allocation counters
for actual local and TLS exchanges. Both 64-trial domains complete with unchanged
source hashes: 16 warmup and 48 measured trials each, three alternating mode pairs
per size/rate. The repository checker measures 237 Kotlin/Python functions, maximum
complexity 7; benchmark line coverage is exempt. No production behavior, installed
session or saved setting changed. Evidence:
`artifacts/2026-11-22-allocations/t1767/`.

## T1766: producer array allocation

Keep the current producer allocation policy for now: reuse the native scratch for
partial reads while copying only their used prefix into a right-sized transferred
array; transfer full original blocks and allocate the next native read separately.
This bounds queued partial payloads by their actual size and avoids borrowing an
array that has already moved to the endpoint. A whole-block pool would change
cross-domain queue ownership and retained capacity, especially for tiny reads.

A finite hosted fixture invokes the actual LocalChannelProducer with its current
64-KiB block. One synchronous synthetic native owner supplies each exact completed
read; the endpoint consumes immediately, after verifying that the next admitted
read cannot overwrite its transferred array. Both native filling and full endpoint
byte validation are included in CPU/wall measurements. Twenty-eight trials contain
eight warmups and twenty measured trials, five per completion size, each with
1,024 completions. Initial producer/native scratch allocation is outside timing.

| Completed bytes | Allocated bytes/completion | CPU ns/completion | Wall ns/completion | Bytes transferred in a different array |
| --- | --- | --- | --- | --- |
| 64 | 168.16 | 97.96 | 90.90 | 64 |
| 4096 | 4200.16 | 1884.14 | 1875.08 | 4096 |
| 16384 | 16488.16 | 7511.00 | 7481.73 | 16384 |
| 65536 | 65640.16 | 27554.81 | 27517.59 | 0 |

Allocation is payload size plus approximately 104 bytes per completion in this
JVM, plus fixed per-trial counter overhead. Full blocks transfer without a payload
copy. The largest measured CPU cost is about 27.6 microseconds/completion, including
fixture filling/verification. Multiplying by 60 completions/s gives about 1.65 ms
of driver CPU per second; this is a workload estimate from the hosted microbenchmark,
not measured tablet throughput. At that same hypothetical rate, 64-KiB completions
allocate about 3.76 MiB/s, so native allocation/GC evidence still matters.

The hosted CPU cost and current compact partial-payload ownership do not justify
introducing a pool or a new offset/length queue contract now. This resolves the
measurement/policy outcome under T1766's existing-policy alternative; it does not
prove allocation harmless under every rate or native collector. T1527 explicitly
retains actual connection-process allocation, GC and retained/peak evidence, and
must reopen a focused buffer optimization if that evidence identifies material
cost. No zero-copy, pooling, native improvement or leak-freedom claim is made.

The synthetic fixture retains at most one producer scratch/pending native read
and one transferred payload, each bounded by 64 KiB. The queue endpoint drops each
payload only after validation, while the producer owns the next distinct native
loan. On normal exit or failure, cancellation completes the exact original pending
synthetic read before owned endpoint storage is discarded. There is no native
thread, socket or external lease whose retirement is inferred from that callback;
production native retirement remains covered by its existing permanent regressions.

All 20 hosted cases across six classes pass, including new invalid-domain checks
and original-array/non-overwrite checks for partial/full boundary sizes. The
repository checker measures 265 Kotlin/Python functions, maximum complexity 7;
Groovy 3.0.17 compiler AST measures four task closures, maximum 3. Benchmark line
coverage is exempt. All 28 trials preserve source hashes. Evidence:
`artifacts/2026-11-22-allocations/t1766/`. No production code, installed session or
saved setting changed. The measurement and policy decision are one cohesive outcome;
no pool implementation is claimed.

## T1769: shared benchmark output paths

Both idle benchmark entrypoints now resolve and validate their destination before
constructing measurement owners. A bare filename writes its complete report in the
working directory; a filesystem root is rejected as a filename. The existing
correct tunnel helper and its three T1761/T1764 regressions moved to the shared
android-hosted source set, with one implementation and current callers updated.
The shared test class is MeasurementOutputTest; no old alias or wrapper remains.

Two permanent entrypoint regressions failed before the fix: the complete seven-trial
actor run and four-case allocation run each ended in a null-parent exception for
a bare filename. The same tests now pass and check their complete report counts.
A third entrypoint test rejects root paths for both programs. Shared bare, absolute
and root helper cases also pass, giving six hosted output cases across two classes.
The normal Python idle-tool fixture invokes these regressions; the tunnel fixture
uses the moved shared tests. Benchmark coverage remains exempt; these correctness
regressions remain in the normal tool suite.

The helper retains only its local File/path values and creates the requested
parent before any actor or probe admission. Existing actor and probe lifetimes
are unchanged; each original actor executor is still stopped and joined before
report publication. Regression filenames have unique owned identities and are
removed in finally on success or failure; absolute-path test directories remain
scoped to their owned temporary directory. Invalid roots admit no measurement
owners. Host filename/path limits remain filesystem constraints, not new invented
capacity or compatibility rules. No installed session is involved.

The repository checker measures 90 Kotlin/Python functions, maximum complexity 5.
The temporary Gradle progress observer has two closures, both complexity 1 under
Groovy 3.0.17 compiler AST. Evidence:
`artifacts/2026-11-22-allocations/t1769/`.

## T1768: shared actor wake/deadline wait

The worker and admission actors now share awaitConnectionWake, a coroutine adapter
that selects an original channel hint or the existing timeout. ConnectionWait
still owns the pure scheduling policy, including its 0..1000 ms result. A zero
wait returns without consuming a retained hint or observing channel closure,
preserving the previous behavior. Positive waits preserve closed-channel failures,
external cancellation and the original fallback/deadline cadence. Other cleanup
and retirement timeouts remain separate and unchanged.

The adapter borrows each actor's own conflated ReceiveChannel until that wait
settles; it adds no shared mutable state, worker thread, native I/O, resource lease
or queue. Coroutine channel/timer registrations belong to that wait and are disposed
on selection/cancellation under the library contract. Permanent tests verify that
post-timeout and post-cancellation hints remain available, including 64 cancellation
cycles. This is not a native I/O fence: original worker, socket and lease retirement
still belongs to the existing resource owners. Eventual coroutine progress assumes
fair dispatcher/timer execution; no timeout is treated as native retirement.

A finite hosted benchmark uses the actual worker actor, one original executor thread
and an identified cue/receipt pair. Every cue must be acknowledged before another
is admitted. Two runs each have twelve trials, alternating burst and 60-events/s
pacing: one whole warmup trial and five measured trials per rate. Each burst trial
also warms 2,000 cues before 10,000 measured cues; each paced trial warms 32 before
120 measured cues. CPU/allocation counters cover the original actor and driver
separately, so shifting work onto the sender is visible.

| Case | Actor allocated bytes/event, before → after | Driver allocated bytes/event, before → after | Total allocation/event, before → after | Total CPU ratio, after / before | p95 wake latency ratio |
| --- | --- | --- | --- | --- | --- |
| burst | 773.0 → 670.4 | 144.0 → 176.0 | 917.2 → 846.4 | 0.947 | 0.915 |
| 60 events/s | 880.2 → 784.2 | 146.9 → 178.9 | 1027.1 → 963.1 | 0.853 | 0.864 |

The paced case saves 64 total bytes/event; the driver allocates 32 bytes/event more,
while the actor saves 96. Burst totals save about 71 bytes/event. Both measured
CPU and p95 latency improve in these hosted runs. These are medians from separate
runs, not confidence intervals or a native Android/media/battery claim. Snapshots
are not atomic across threads, so a surrounding actor iteration and sampler overhead
can cross a boundary. Raw data and current source hashes remain in the artifacts.
The result supports the small shared adapter change; it adds no pooling or identity
reuse and changes no user default.

Six new permanent wait regressions pass with the original timeout behavior and
the selected implementation. An intentionally omitted zero-delay guard fails the
retained-hint regression; restoring the guard passes. The broader 777 hosted cases
across 104 classes pass. All 58 measured production functions meet the 80% line
coverage requirement (minimum 80%); Profile compilation passes. Five normal Python
tool tests pass, covering shared benchmark compilation, the active fixture, the
complete idle/allocation output regressions and all tunnel fixture cases after
moving pacing to its shared owner. The checker measures 216 Kotlin/Python functions,
maximum complexity 8. Benchmark coverage remains exempt.

Active and idle experiments now use one shared actor/probe owner; their exact original
thread is cancelled, awaited, retired and joined before replacement. The probe
retains one cue and one receipt, bounded to 20,000 admitted identities per actor;
selected trials need at most 12,000. Raw latency storage is bounded by 10,000 values
per trial and twelve reports. No actor/native reference enters the returned results.
Existing setup/permission owners are synthetic, so no physical tablet or installed
session participates. Native process memory/GC acceptance remains in T1527.
Evidence: `artifacts/2026-11-22-allocations/t1768/`.
