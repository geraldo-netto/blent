# T1148 — output operation progress

An owned native configuration or whole-frame write now publishes one bounded
progress observation. Its controller rejects an operation still pending after
2s measured from actual operation start. Idle time has no deadline. A completed
or replaced operation invalidates the earlier observation; closed observation
channels await the original native join rather than spinning.

This timeout is a logical failure, never evidence that native work stopped.
Controller retirement closes its stop authority. The original Task, Report
completion, file lease and frame borrow remain funded until actual native return
and join. A native call that never returns remains owned; eventual cleanup assumes
native return and observer progress. Span borrows cannot nest on the same sender;
the observation owns no native data and clears on return, error or unwind.

Two permanent regressions failed before the fix using a real blocked pipe write
and kernel-lock-held configuration. Both perform independently sponsored release
and actual cleanup before asserting the missing timeout. The unchanged sealed
source passes after the fix and confirms original completion/lease remain pending
at timeout. Four additional tests cover idle/closed progress, deadline replacement,
completion, native panic and stale expiration. Existing bounded invalid frame and
profile corpora remain in the normal suite.

Offline attested LLVM collection:20 worker tests pass; all14 worker/progress
production functions pass individually>=80% executable-line coverage. A broader
camera run passes89 tests. Scoped Clippy passes. Pinned parser source audit:
55 functions, maximum4; this is not SonarQube evidence (T1019 remains blocked).
The snapshot includes pending T1137 application work; this commit qualifies only
the worker/progress sources. It does not claim application integration, physical
camera acceptance or legacy ADB safety. No network transfer or host device/service
change occurred. Raw manifests, counters and attestations remain locally under
`/backups/disk2/blent-work/2026-10-19-open-items/t1148`.

```sh
cargo test --offline --locked -p blent --lib camera::output_worker
cargo clippy --offline --locked -p blent --lib --features accessory-transport --no-deps -- -D warnings
```
