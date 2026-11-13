# Android lifecycle review

Reviewed source revision `56250b92`. Two confirmed findings are tracked by the
parent in `TODO.md`: **T1414** and **T1416**. This review changed no production or
normal-suite test source. No device command was executed: the installed release
and the user's tablet were not stopped, restarted, reconfigured or exercised.

## Confirmed findings

**T1414 — Activity replacement loses the USB-to-ADB cutoff.**
`SessionCoordinator.kt:94` keeps external selection and delivery cutoff in each
Activity coordinator. `ConnectionDisplayBinding.kt:15` treats every initial
inactive view as untouched ADB, including a retained explicitly disconnected USB
attempt. `MainActivity.onCreate/onStart` therefore creates/starts a replacement
coordinator with the pre-cutoff cached token. The composition probe reproduces
one ADB control socket admission on port 8891 after USB disconnect, without any
new protected token delivery. This proves admission despite the cutoff, not
successful host authentication, TLS bypass or video rendering. Preserve the
selection/cutoff in an owner surviving Activity replacement, retaining ordinary
initial ADB startup. Permanent regression coverage must recreate and start after
disconnect/denial/revocation, prohibit control and video admission until a later
protected delivery, and retain positive fresh-delivery controls. T1396 checks
USB-worker count without starting the recreated Activity; T1404 checks the same
coordinator, so neither disproves this cross-lifetime trace.

**T1416 — Replacement display owner collides with its retiring worker.**
`MainActivity.kt:64` creates a new `RemoteDisplay`; `RemoteDisplay.kt:42` creates
its own retirement lane. Stopping the old Activity does not await its worker's
retirement. The replacement can bind the same still-live display service and
send a different generation. `MediaWorkerService.kt:49` rejects that START and
terminates the worker process; `RemoteDisplay` reports failure and clears its
wanted state. The probe executes actual `RemoteDisplay`, `AndroidWorkerRun` and
`MediaWorkerService` logic with a cached-Binder context and hosted native-run
fixture. It observes two binds before original retirement, one termination
request, and replacement error `Worker command rejected`. Android documents that
additional clients receive the cached Binder without a new `onBind` call:
[Bound services overview](https://developer.android.com/develop/background-work/services/bound-services#the-basics).
Retain a process-owned display-worker admission/retirement fence across Activity
instances. A permanent regression must hold original retirement across
recreation, prohibit replacement binding until original process-death and
cleanup proof, then verify successful replacement; retain the same-owner control.

## Executed evidence

- Gradle 8.5 exported the current hosted debug-test classpath and checked its
  compile prerequisites. This was not a full Android test-suite run.
- Kotlin 1.9.20 compiled the two external review probes against current production
  and existing test-fixture classes. These artifact probes are not permanent
  normal-suite regressions.
- JUnit 4.13.2/Robolectric 4.17 executed **10 cases across API 27 and API 34**:
  T1414 had 2 expected failing reproductions and 4 passing controls; T1416 had
  2 expected failing reproductions and 2 passing same-owner controls. Java exited
  1 because the four requested safety assertions fail on the reviewed source.
  See `probe.log`; `compile.log` is empty on successful final compilation.
- The scoped project cyclomatic checker measured **43 authored owners across all
  4 retained probe/script source files**, maximum complexity **7**. It used Kotlin
  PSI 1.9.20 and the Python AST path; per-owner scores, tool information and input
  hashes are in `probe-complexity.json`. No full-project metric claim is made.

The T1416 probe uses the exposed 60000 ms EXIT-grace parameter to make the
pre-exit ordering deterministic, and releases its barrier immediately during
cleanup. Production uses 500 ms; the source permits the same overlapping ordering
inside that interval. The probe does not establish its frequency on the tablet.
Termination is simulated by the existing service fixture plus Binder-death
notifications; no process or native decoder is killed. Both probes retain and
settle their fixture workers, scopes and surfaces in cleanup. Missing native
process-death/recreation evidence is a limitation, not a separate defect.

## Reviewed scope and limits

Source review covered the Activity/session composition, protected token delivery,
attachment epoch admission, remembered USB connection binding, remote display
and worker retirement, media Activity attachment, streaming power lifecycle,
application composition and release manifest/R8 rules. `source-manifest.json`
lists the exact files and whether examination was complete or scoped. Existing
T1396/T1404/T1405-related paths and targeted fixture/regression sources were used
to challenge candidates. No third finding was inferred from source availability,
unexecuted tests or absent native evidence. There was no physical process-death,
suspend, background/foreground, Surface destruction or R8 release lifecycle trial.
The production manifest isolates media workers and R8 rules retain the application
classes; source inspection alone does not prove native release acceptance.

## Reproduce without a device

From the repository root, using the already populated Android/Gradle dependency
cache and JDK 21:

```sh
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
export ANDROID_HOME=/backups/disk1/apps/android-sdk
export BLENT_REVIEW_WORK="$HOME/.cache/blent-lifecycle-review-android"
mkdir -p "$BLENT_REVIEW_WORK"
./android/gradlew -p android --offline \
  -I ../docs/reviews/artifacts/2026-11-13-lifecycle-review/android/export-classpath.gradle.kts \
  -PblentLifecycleReviewClasspath="$BLENT_REVIEW_WORK/classpath.txt" \
  :app:exportLifecycleReviewClasspath
python3 docs/reviews/artifacts/2026-11-13-lifecycle-review/android/run-probe.py
```

The runner writes textual compilation/JUnit logs beside this document. Generated
classes and the machine-specific classpath stay in `BLENT_REVIEW_WORK`, outside
Git. It prints the Java result (`probe exit: 1` for this reviewed revision); its
wrapper return code does not turn expected JUnit failures into passing tests.
