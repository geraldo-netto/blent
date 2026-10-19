# T1125 camera controller retirement fence

Four permanent regressions fail before the controller fix: premature replacement,
worker exit with an outstanding native proof, stale admission, and nested-scope
bypass of original cutoff. They pass after it; their original bodies remain
unchanged (a separate owned-future test module is added). A fifth test covers
success/error/panic future destruction before waiting on that future's own guard.
All nine controller tests pass; the fresh full host library passes 622 tests,
with three pre-existing opt-in timing benchmarks still ignored.

The Report owns a shared bounded observation table, not native resources. Its
separate structural budgets are one connection, four producer child/task owners
and one decoder. Reservation precedes native effects; native adapters must bind
the exact original proof synchronously before yielding/return. Original-controller
and child activity permissions both gate callbacks and new reservations. Cutoff
waits for synchronous admission/publication, then seals the original permission;
an already admitted creator keeps its reservation until failure or binding.

Every logical outcome and stop waits for actual independent proof completion
before terminal status, replacement or worker retirement. There is no elapsed
cleanup deadline or automatic replay. Unknown native work remains funded by its
native owner and observed by the original controller. Controller destruction
requests stop and returns; the existing thread-retirement sponsor retains the
actual worker until join. Sibling controllers keep independent bounds/progress.
Eventual replacement assumes actual native cleanup and fair observation; a hung
owner can keep its original slot indefinitely without admitting unsafe reuse.

Fresh source/compiled-object-attested LLVM coverage measures all 27 production
functions in camera_control.rs and camera_control/native.rs at >= 84.62%
individually, no exclusions. Raw counters/attestation remain under
`/backups/disk2/blent-work/2026-10-19-open-items/t1125/native`.
The pinned Rust source audit measures all 44 production/test/helper functions
at <=6; this is not actual SonarQube evidence (T1019).

T1126 must still wire real camera bridge/producer/decoder creations to these
reservations. Therefore this prerequisite does not close T1089 or establish
complete native camera retirement. T1090 ADB ownership remains explicitly blocked
by owner choice; no Windows/EVDI integration or live device action occurred.
