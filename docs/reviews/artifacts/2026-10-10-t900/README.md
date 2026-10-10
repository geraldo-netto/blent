# T900 — portable host pairing presentation model

`control_view` assembles only complete, matching-revision broker snapshots.
Device/link/saved-peer maps are ordered and bounded at configurable limits up to
512/64/1024, matching broker capacities. Partial, duplicate, invalid-counter,
contradictory identified-link, nested and mixed-revision input is rejected;
recovery requires a fresh Begin. Snapshot completion grants no remote authority.

UI actions retain only bounded command metadata, the displayed computer identity
and an opaque reference to their broker lifetime. Submission rechecks current
revision/device or connection/ticket/state. A reconnect invalidates old actions
even when all persisted identities and numeric counters are equal. Approval and
revocation do not optimistically change saved trust or authenticated state.
Backward/conflicting snapshots invalidate all current actions. Public view and
control types contain no GUI, filesystem, process or native-backend internals.

Eleven permanent tests pass offline, including all capacity maxima, 4,096 seeded
malformed records, exact-session replacement, boundary counters, stale/foreign
choices, duplicate records and durable-state preservation. A fresh isolated LLVM
coverage target avoids the reused-object issue tracked under T880. All 23 scoped
production functions meet 80% individual executable-line coverage; all 43 reviewed
production/test functions have cyclomatic complexity at most 9.

This is the independently testable presentation layer. Linux transport remains
T901, host GUI widgets T894 and joined native pairing acceptance T855. No native
USB access, installation, download or production support claim accompanies it.
