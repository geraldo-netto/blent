# T1172 — return retired display-fixture history capacity

A second execution of the original joined display fixture timed out waiting for
its new socket's authentication. Its first execution passed. Two released codec
shadows remained in the SDK's class-level history after fixture close; their old
readiness could satisfy codecReady(0), while the bounded two-entry history refused
a new configured codec. The custom shadow's Resetter annotation did not clear
this history across the repeated JUnitCore lifetimes in the retained reproduction.

Permanent normal-suite JoinedDisplayReplayTest executes both original SDK27/34
oracles twice. It failed on the second lifetime before the fix. A separate
JoinedDisplayHistoryTest executes the original native workflow, verifies actual
codec release, then requires returned history capacity; both SDK cases failed
before the fix. Both complete new test files are sealed and pass unchanged after
fixture cleanup returns that metadata capacity. Original assertions and the
4000 ms predicate deadline were preserved. The full 69-class prelude replay also
passes after the correction; no failure, assertion or exception is suppressed.

The sequential SDK fixture owns at most two codec-history records. Close first
settles the original receiver/worker scopes and joins their original jobs, awaits
decoder retirement, verifies original socket/job and codec release evidence, and
releases the original Surface and SurfaceTexture. Only then does it clear the
retired test history. Earlier failure retains the history; cleanup never uses a
logical timeout or a reference count as native proof. No native release is added
or retried. This fixes test-owned metadata retention and admission, not production
streaming or physical decode/presentation. Framework SDK/process lifetime remains
the sponsor for unavailable original cleanup; this bounded hosted trace does not
prove unrestricted native safety.

The focused run passes all eight outer JUnit cases (two history cases, one replay,
two existing ordered checks and three full/UI order checks), including their
verified nested original oracles. Cached Kotlin 1.9.20 PSI measures 40 entries,
maximum 3; native SonarQube remains T1019. No production function changes, so no
artificial production coverage test or exclusion is introduced. Raw evidence:
`/backups/disk2/blent-work/2026-10-20-open-items/t1172`.
