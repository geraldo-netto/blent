# T1129 camera/audio CLI signal cleanup

Both CLI entry points use the shared portable `control_outcome::with_signals`
workflow. A failed Ctrl-C registration requests stop and awaits the same original
work before returning its signal diagnostic. Normal Ctrl-C/SIGTERM preserves the
original work result after cleanup. Naturally completed work requests no extra
stop. OS signal adapters stay in the entry points.

Five permanent normal-suite tests exercise all three signal outcomes crossed
with successful/failed original work, natural completion and invalid-profile
entry-point rejection without devices. An original admission remains held until
an explicit cleanup gate opens. The identical oracle fails before the shared
ordering fix and passes afterward; logs are compressed alongside this note.
The earlier camera-only collection at `t1129/native` predates shared extraction
and is historical, not final evidence.

The caller owns the pinned original future and stop sender. The helper borrows
both until the selected outcome settles, creates no native owner or spawned
work, and retains no view beyond its return. Stop is a cutoff request, not proof;
only original completion allows return. Cleanup failures cannot bypass the wait.
Eventual return assumes that original work is polled and actually settles; no
elapsed timeout is substituted. External caller/process loss still depends on
the underlying owners: camera process-death safety is separately unresolved in
T1137 and identity-safe legacy ADB cleanup remains blocked under T1090.

Native coverage results and final source-matched evidence are recorded below.
The pinned source audit covers 42 production/test functions in the affected
files, maximum9 (the unchanged T812 panic test); changed/new functions <=5.
This audit is not actual SonarQube measurement; T1019 remains blocked.
No Windows/EVDI code, live device/signal action, install or network transfer.

Fresh source/compiled-object-attested offline LLVM collection passes 633 host
library tests; three existing opt-in benchmarks remain ignored. Every production
function in camera.rs, audio.rs and control_outcome.rs passes individually:
32/32 >=80% executable lines, zero unmeasured, no new exclusions. All new shared
signal functions and the changed CLI entry points have measured native counters.
Final raw manifest/counters/objects:
`/backups/disk2/blent-work/2026-10-19-open-items/t1129/native-shared`.
Cargo workspace formatting passes; excluded Linux include drift remains T1128.
