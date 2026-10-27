# T1178: checked process-wide worker revisions

Media and connection adapters share a checked worker-generation contract with distinct process-wide domains. Hosted boundary and concurrency tests passed; generated identities do not prove physical process retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

WorkerGenerations is a focused implementation-independent positive-Long
allocator, extracted from the already checked connection adapter. Both adapters
use that contract through separate process-wide instances for their distinct IPC
domains. All current connection callers use the shared type; no old alias or
second admission implementation remains. Media worker construction allocates its
generation before context lookup, mailboxes, deferreds, control scopes or Binder
objects. Maximum exhaustion throws before those allocations or native effects.

Two permanent media regressions fail before the repair on both API27 and API34
(four failing cases). They verify terminal rejection and final MAX admission,
old/negative/zero and late Binder event filtering, original STOP/EXIT, actual
control-job joins, unlink and unbind. The rejected-construction test cleans any
unexpected original run in finally, including during the failing control.

All85 selected tests pass:28 media cases,54 connection cases and3 pure allocator
tests. The independent BigInteger oracle checks34 seeds,20 admissions per seed
(680 transitions),33 negative seeds and32 joined concurrent calls across four
boundary seeds. Successful values are unique, contiguous and positive; exhausted
calls remain rejected. All test threads are joined, including partial creation;
no arbitrary elapsed deadline terminates progressing allocator work.

JaCoCo0.8.15 reports100% executable-line coverage independently for all9 measured
changed JVM method variants, including both connection constructor families and
their default-argument variants. The originally uncovered primary default
constructor is exercised by a retired-before-bind launch and terminal admission.
Cached pinned Kotlin PSI1.9.20 models417 production/test functions, maximum7.
That model is not native SonarQube certification; T1019 remains unresolved.
Native reports, source fingerprints and original red/green output are retained.

Reproduce offline from the repository root:

```sh
./android/gradlew -p android --offline --no-daemon \
  -I /backups/disk2/blent-work/2026-10-19-open-items/t1150/offline.gradle \
  -I "$PWD/scripts/coverage/android.init.gradle" \
  testDebugUnitTest --tests 'com.blent.AndroidWorkerRunTest' \
  --tests 'com.blent.WorkerGenerationsTest' \
  --tests 'com.blent.connection.AndroidConnectionWorkerRunTest' profileCoverage
```

The process instance owns one AtomicLong for its containing process lifetime;
the allocator retains no Context, Surface, Binder, scope or callback. A successful
CAS hands one immutable positive identity to one run. Failed CAS changes no
state; MAX is never reset or reused. A negative seed creates no external
resource. Each of the bounded concurrent calls can lose CAS only to another
successful admission; eventual execution still assumes scheduler fairness.

Original runs retain their existing Binder, borrowed Surface, jobs and cleanup
sponsors until actual retirement. Counter exhaustion neither releases those
resources nor certifies their death. Callback identities remain immutable;
default allocator ownership is not duplicated in each run. Rejected construction
retains no new run or native obligation. Existing failed/stuck-native-call tests
remain in the passing scope. Robolectric Binder transactions and joined hosted
jobs do not prove physical cross-process/device behavior; existing native
acceptance limits remain in the ledger. No APK, live service, physical device,
Windows/EVDI backend or network transfer was changed.

</details>
