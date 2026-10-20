# T1181: checked receiver start, arithmetic-free stop

VideoReceiver retains its existing authoritative counter and running flag.
Start checks the nonnegative positive-Long admission range under the existing
receiver monitor before changing state or allocating jobs, statistics, sockets
or decoder work. Exhaustion throws explicitly. Stop revokes through the running
flag and keeps every existing cleanup action, without needing another number.
One successful start consumes one fresh identity; repeated running starts do
not consume identities. No new counter, compatibility path or native state owner
is added. The internal constructor seed defaults to0 and rejects negatives.

Three permanent boundary tests fail before the fix on API27/API34 (six cases):
terminal start allocates work instead of rejecting; last-generation stop wraps
MAX to MIN; and actual original socket/codec cleanup also wraps the counter.
The same tests pass after the fix. The retained red run uses the normal codec
inventory fixture and H.264, reaches actual hosted socket/codec ownership, and
fails at the intended counter assertions after original cleanup settles.

All122 selected tests pass, including existing transport, native-startup,
Surface wake, configuration, late codec callback, decoder retirement and display
worker regressions. New tests join original jobs, retain blocked codec creation
until its actual return/release, verify original socket close and codec release,
preserve the caller's borrowed Surface, reject stale timer authority and retain
an independent old statistics accumulator. A BigInteger oracle checks34 seeds,
20 start/stop sequences each (680 per SDK). Two eight-thread phases cover final
admission and terminal rejection; all threads and owned jobs are joined.

JaCoCo0.8.15 reports100% executable-line coverage independently for all6 changed
measured JVM method variants, including default construction and the separately
compiled receive/timer workers. Cached pinned Kotlin PSI1.9.20 models124
production/test functions, maximum9. These source-model scores are not native
SonarQube certification; T1019 remains unresolved. Source fingerprints, native
counters, selected test counts and red/green output are retained.

Offline command from the repository root:

```sh
./android/gradlew -p android --offline --no-daemon \
  -I /backups/disk2/blent-work/2026-10-19-open-items/t1150/offline.gradle \
  -I "$PWD/scripts/coverage/android.init.gradle" \
  testDebugUnitTest --tests 'com.blent.VideoGenerationTest' \
  --tests 'com.blent.VideoConnectionBoundaryTest' \
  --tests 'com.blent.SurfaceWakeupTest' --tests 'com.blent.DisplayRoutesTest' \
  --tests 'com.blent.FramedDecoderTest' --tests 'com.blent.DecoderStartupTest' \
  --tests 'com.blent.DecoderSetupTest' --tests 'com.blent.RemoteDisplayTest' \
  profileCoverage
```

The receiver monitor still serializes start/stop; its private AtomicLong never
wraps or decreases. The running flag cuts off authority before cleanup, and no
later start can reuse MAX. Each admitted start owns one SupervisorJob and its two
workers, one statistics accumulator and the original transport/decoder owners.
Denial allocates none of those. Original jobs, startup attempts, codec borrow
counts, native release threads and socket creators keep their existing cleanup
sponsors; numeric invalidation does not certify their completion.

The blocked-creator case keeps the original job incomplete until its creation
call returns, then joins it and awaits actual hosted codec release. Native
Surface ownership stays with its caller and is released by tests only after
original work settles. All test admission loops retire/join before reuse;
failure finally blocks sponsor cancellation, socket close, gate release and
original job/codec settlement. Eventual cleanup still assumes the original
creator/native calls return and scheduled cleanup runs. Robolectric shadows and
bounded hosted fixtures do not prove unrestricted safety or physical Android
behavior. No APK, running service, physical device, Windows/EVDI backend or
network transfer was changed. Separate NativeDeadline wrap is recorded as
T1190; this receiver change does not claim to fix that notification owner.
