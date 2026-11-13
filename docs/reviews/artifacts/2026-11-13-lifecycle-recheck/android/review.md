# Android lifecycle recheck

Reviewed commit `9f8660aa`. Two confirmed ordering defects are recorded in
`TODO.md`: **T1419** and **T1420**. Both are adjacent to T1414's persisted cutoff;
this review found no additional confirmed T1416 defect. No production source,
normal-suite test, device state, installed package or external service was changed.

## T1419 — background observation consumes a fresh delivery

`MainActivity.kt:171` cancels USB-display observation when the Activity stops.
After USB disconnect and a protected ADB delivery while backgrounded,
`MainActivity.kt:158` first replays the late disconnected snapshot at resume.
`SessionCoordinator.kt:348` then records the current delivery counter as a new
cutoff, making that already post-disconnect delivery appear stale.

The actual Activity/connection/session composition probe executes:

1. Start with an admitted USB display and protected delivery counter 1.
2. Pause/stop the Activity, disconnect the USB binding and settle its fixture
   worker, then accept protected delivery counter 2.
3. Confirm `adbDeliveryPermitted` is true before resume.
4. Start/resume the same Activity. Permission becomes false, control/video routes
   remain absent, and no ADB control socket is admitted.

This reproduced on **2/2 SDK variants (27 and 34)**. Foreground
`disconnect -> delivery` and `resume -> delivery` controls passed **4/4
executions**, each selecting ADB control port 8891. The impact is lost valid
reconnect authorization until another protected delivery occurs, not a host
authentication bypass. Severity **medium**, effort **small**.

Completion requires recording cutoff at the authoritative USB transition,
independent of Activity visibility; merely replaying presentation on resume must
not advance it. Permanent regression coverage must retain this background
ordering, repeated stop/start controls, pre-cutoff rejection, foreground delivery
and fresh post-resume delivery. Preserve untouched ADB startup and original native
cleanup ownership. This is independently verifiable from T1420's missing cutoff.

## T1420 — null-choice conflation skips explicit disconnect

`ConnectionDisplayBinding.kt:16` returns when the admitted nullable display choice
has not changed. An opening USB connection and a later explicit disconnect both
have `choice == null`, although the latter is a new cutoff. The adapter therefore
never invokes `selectDisplay(null)` for that disconnect.

The actual Activity/connection/session composition probe executes:

1. Start a USB opening attempt without an admitted display; its null selection
   cuts off cached ADB delivery 1.
2. Accept a protected delivery 2 while that attempt remains open. The current
   supported return-to-ADB path admits control port 8891.
3. Explicitly disconnect the USB binding and settle its original fixture worker.
4. Observe `active=false` and `reconnectAllowed=false`, but ADB permission and the
   port 8891 route remain admitted from delivery 2, which preceded disconnect.

This reproduced on **2/2 SDK variants (27 and 34)**. The observation proves that
local ADB work remains admitted across the explicit cutoff; it does not prove
successful host authentication, a TLS bypass or actual video rendering. Severity
**medium**, effort **small**.

Completion requires distinguishing actual selection/cutoff epochs from nullable
route equality. Permanent regressions must cover explicit disconnect, denial and
revocation when no USB display has yet been published, reject pre-cutoff delivery,
and allow only a later protected delivery to restore ADB. Keep ordinary repeated
presentation snapshots coalesced and preserve unrelated connections. T1419's
background ordering must remain correct when this transition handling changes.

## Final recorded verification

`ReviewAdbOrderingProbe.kt` is external review evidence, not a permanent
normal-suite regression. It uses the existing `ConnectionUiFixture`,
`SessionDisplayRig`, `WorkerTestBridge` and actual MainActivity lifecycle methods.
Kotlin 1.9.20 compiled it against the current production and fixture classes.

- Final probe run: **8 executions**, comprising **4 expected assertion failures**
  (2 per finding) and **4 passing controls**. JUnit exited 1; see `probe.log`.
- Existing T1414/T1416 regressions: **42/42 executions passed** across the six
  classes listed in `run-probe.py`; JUnit exited 0. See `existing-tests.log`.
- Retained probe/script complexity: **15 authored owners across 3 files**,
  maximum **4**, using the project Kotlin PSI 1.9.20/Python AST checker.
  Scores and input hashes are in `complexity.json`.

The Gradle classpath export checks current compile prerequisites. Neither these
runs nor source inspection is a complete Android test-suite or native acceptance
claim. They preserve accepted failed-retirement quarantine semantics.

## Scope, ownership and limits

`source-manifest.json` distinguishes complete-file reads from focused sections.
Review covered Activity route observation, protected delivery/cutoff storage,
USB presentation transitions, current display-owner identity, latest pending
request, callback detachment and original worker retirement. Existing tests also
exercise the cached-Binder/death boundary through its isolated shared scheduler;
no hosted main-looper stall occurred in these recorded runs.

Activity controllers are stopped/destroyed in finally blocks; fixture connection
workers are released and their owning scope joined; media owners are reset and
the main looper is drained. The new ordering probe starts no native video decoder
or physical socket. Existing native-boundary fixtures settle their simulated
Binder/service scopes and release owned Surface/SurfaceTexture objects. Generated
classes and the machine-specific classpath remain outside Git.

No physical process-death, rendering, suspension, USB transition or release/R8
trial was performed. These are source traces and finite hosted executions, not
proof of unrestricted native safety or additional platform support. Missing
native observations were not treated as defects.

## Reproduce

From the repository root with the existing dependency cache:

```sh
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
export ANDROID_HOME=/backups/disk1/apps/android-sdk
export BLENT_REVIEW_WORK="$HOME/.cache/blent-lifecycle-recheck-android"
mkdir -p "$BLENT_REVIEW_WORK"
./android/gradlew -p android --offline \
  -I ../docs/reviews/artifacts/2026-11-13-lifecycle-recheck/android/export-classpath.gradle.kts \
  -PblentLifecycleReviewClasspath="$BLENT_REVIEW_WORK/classpath.txt" \
  :app:exportLifecycleReviewClasspath
python3 docs/reviews/artifacts/2026-11-13-lifecycle-recheck/android/run-probe.py
```

The wrapper prints the separate JUnit exit statuses. Its own completion does not
convert the expected failing probe assertions into passing tests.
