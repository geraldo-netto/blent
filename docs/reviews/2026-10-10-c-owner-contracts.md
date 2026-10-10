# T823: private native owners, portable C contracts

`conversion.h` exposes pixel jobs and an opaque pool; `frame_exchange.h` exposes
opaque ownership, borrowed capture targets and immutable writer leases. Pthread
layout, worker dispatch, buffer histories and mutexes live in private headers.
Only implementation files and the composition root use those owner definitions;
native fixtures retain private access to test invariants.

Capture damage uses one exchange-owned transaction per batch. Readiness, wakeup
and latency operations own their synchronization. Capture-only target metadata
is borrowed until the same owner publishes/resizes. Dimensions retain the existing
mode-owner validation precondition. No hot-path allocation, pixel copy, extra lock
or new indirection in the conversion kernels was introduced. Worker storage stays
statically allocated by the native composition root. EVDI is unchanged.

Permanent public-header isolation tests failed before the refactor, then passed.
`host/tests/tooling.rs::t823_c_pixel_contracts_hide_native_owner_layout` runs those
checks in the normal host suite. Existing lifetime, lease, conversion-oracle and
sanitizer tests remain; `t823_exchange_contract` adds extreme rectangles/scales,
bounded latency draining and null-callback handling. Four native test targets pass
(12 + 58 + 1 + 1 tests). All 222 native production functions meet 80% line coverage;
all native function complexity scores are ≤9. The coverage manifest precedes only
a clarifying dimensions-precondition comment in the public header; executable C
bytes are unchanged from the measurement.

## Replay

Five alternating before/after pairs, one 1920×1080 session, scale 1, four-worker
capacity, 128 samples, `cc -O3`, no EVDI/device access. All checksums match.
Median per-run conversion p50: sparse 8.236 / 8.467 µs, full 131.758 / 131.086 µs.
CPU per 128-frame run: sparse 1.136 / 1.161 ms, full 64.517 / 63.808 ms.
Conversion p99: sparse 12.725 / 12.424 µs, full 198.749 / 220.281 µs; scheduling
and worker wakeups make these short-run tails noisy, so no latency gain is claimed.
After uses the public atomic transaction; before used direct mutex ownership.
Sparse damage median 130 / 161 ns and full 491 / 481 ns have different timing
boundaries: after includes lock/unlock, before reports acquisition separately.
They must not be presented as a damage-only speed comparison. Structural lock
count remains one per batch, with no waits or allocations added to callbacks.

The benchmark harness detects private headers so historical sources still build.
Evidence: [paired runs, coverage and logs](artifacts/2026-10-10-t823/).
