# T1201: bounded camera-probe timestamp admission

The existing unbounded timestamp collection was first extracted into Samples,
and the actual probe caller changed to use that single implementation before any
behavior fix. Two permanent normal-suite regressions then fail: reservation does
not prevent a third observation after capacity two, and zero capacity is accepted.
The same methods now reject invalid capacities and observations beyond the bound.

The user controls --max-samples: default 8192, range 1–1048576. Construction
validates before fallible exact reservation and before the caller's native setup.
Exhaustion returns an error and invalidates measurement; it never truncates
history while claiming a complete result. Counts/first timestamp derive from the
single retained vector. Timestamps use representable u64 milliseconds; invalid
Duration conversion leaves observations unchanged. Scalar JSON values remain
lossless for the accepted numeric domain.

All five normal integration tests pass. The independent count oracle covers
capacities 1–64, each through capacity+9 attempts (2,656 observations), rejection
preserves the exact stored prefix, and boundary tests exercise zero/oversize/
usize::MAX capacity, Duration::MAX, empty state and u64::MAX milliseconds.
The minimum/default/maximum reservations and JSON representation are also tested.
Command: cargo test --offline --locked -p blent --test camera_probe_samples.
Formatting/whitespace checks pass. This benchmark module is explicitly exempt
from production per-function coverage; no artificial coverage tests were added.
Manual source estimates for changed methods/functions are <=9, including the
existing caller's loops and control arms. Native SonarQube/grammar evidence is
unavailable under T1019. The separate pre-existing decoder integration failure
remains T1189; this helper pass is not a native probe or full-workspace pass.

Samples exclusively owns its vector; capacity is immutable admission policy,
never a second mutable count. Initial state has zero elements. Each success
adds exactly one representable timestamp while len < capacity; each rejection
leaves state unchanged. Exact reservation funds at least capacity elements, and
record cannot trigger growth beyond it. Retained timestamps cost at most 8 MiB,
plus allocator/JSON overhead; compact array output is bounded by 21*n+2 bytes.
Borrowed timestamp views are synchronous, read-only and cannot outlive or overlap
mutable access to their owner. No frame/native buffer, callback, thread, queue or
native pointer is retained. Scope/future destruction releases the vector after
its observer borrow ends; partial allocation failure owns no external resource.
No allocator or native lifecycle is invented for this pure local data owner.
No physical camera/ADB/device work, EVDI/Windows change or network transfer.
