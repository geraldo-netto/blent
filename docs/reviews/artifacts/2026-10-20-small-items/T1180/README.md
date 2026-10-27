# T1180: terminal display callback authority

Display callback authority uses checked grants and arithmetic-free revocation. Boundary, concurrency and bounded-history tests passed while original native workers remain owned until their own retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

RemoteDisplay now owns one GenerationFence. A replacement first revokes old
callback authority, stops the original lane and disconnects observations. Only
a usable wanted display acquires a fresh positive-Long grant. Exhaustion denies
the factory with visible status; no counter wraps. Completion revokes its
original grant without needing another number, including at MAX. Queued status
and render acknowledgement predicates consult the same original fence.

Three permanent regressions fail before the fix on API27/API34 (six cases).
The before-fix test seam seeds the actual existing Long field; terminal setup
wraps and admits work, terminal replacement starts after held retirement, and
failed completion permits another worker. The same assertions pass after the
fix. Additional permanent cases cover queued original status and invalid seeds.
All37 selected tests pass, including existing bounded/stalled retirement and
status-dispatch regressions. No regression was removed, skipped or weakened.

The pure independent BigInteger authority oracle exhausts all81 acquire/revoke/
observe schedules of length4 for each of34 seeds (0–16 and MAX−16–MAX):2,754
traces and11,016 transitions. Every step checks current and stale/invalid values.
It also checks33 negative constructor seeds, ordinary default admission and an
eight-thread terminal race with actual joins. Denied replacement invalidates the
last callback grant; this policy is intentionally separate from preserving the
original native cleanup obligation.

JaCoCo0.8.15 reports100% executable-line coverage independently for15 changed
measured JVM method variants, including default constructors and separately
compiled completion/status/render predicates. Cached pinned Kotlin PSI1.9.20
models151 production/test functions, maximum6. Native SonarQube measurement is
still unavailable under T1019. Original red/green logs, native counters, source
fingerprints and individual scores are retained here.

Offline command from the repository root:

```sh
./android/gradlew -p android --offline --no-daemon \
  -I /backups/disk2/blent-work/2026-10-19-open-items/t1150/offline.gradle \
  -I "$PWD/scripts/coverage/android.init.gradle" \
  testDebugUnitTest --tests 'com.blent.RemoteDisplayTest' \
  --tests 'com.blent.GenerationFenceTest' \
  --tests 'com.blent.BoundedRetirementTest' \
  --tests 'com.blent.StatusDispatchTest' profileCoverage
```

GenerationFence owns only its containing instance's revision and active flag,
under one monitor. Invariants:0<=revision<=MAX, active implies revision>0,
revisions never decrease, revoke clears authority without arithmetic, and no
acquisition succeeds after MAX. Successful acquisition hands one immutable
number to callbacks. Failure allocates no native resource. The containing
display and retained original callbacks keep the fence alive; no allocator,
queue, native handle or cleanup sponsor is introduced by the fence itself.

The unchanged ResourceLane remains authoritative for native workers, cutoff,
joined execution and actual retirement. Revocation does not free a worker or
authorize reuse. Held cleanup retains its original owner until its proof;
replacement denial clears pending admission while preserving that obligation.
Borrowed Surface ownership stays with its caller. Tests release Surface only
after original jobs settle, drain queued dispatches and join test threads/scopes.
Eventual native release still assumes the original backend returns its actual
retirement evidence and scheduled cleanup executes. Bounded hosted checks do
not prove unrestricted native safety or physical device behavior.
No APK, running service, physical device, Windows/EVDI backend or network
transfer was changed.

</details>
