# T1242 rendered media controls, reconnect and original cleanup

Rendered controls preserve independent camera/audio consent and cleanup across reconnect. Joined hosted tests passed, but synthetic workers and transport do not establish physical media retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Three permanent normal-suite tests compose the production camera/audio controls,
connection binding, consent observations, media selection, consumer wrappers and
resource lanes. The transport and native workers are explicit synthetic fixtures;
no tablet, Android native camera/audio device or process death is represented.

- Rendered Start sends original consent without acquiring native media before
  admission. Stops independently cancel camera, microphone and speakers while
  the authenticated connection and remaining consumers survive. Original route
  closure cannot emit consumer-retired commands before original worker cleanup.
- Reconnect initially waits for original connection retirement. Equal replacement
  lease values require fresh rendered Start. Queued old Start/Stop actions cannot
  control either an offered or an actively running replacement. Each replacement
  native factory waits for its own previous consumer, independently of siblings;
  old completion/status callbacks cannot alter the replacement.
- Microphone SDK denial acquires no microphone worker and leaves existing camera
  and speaker consumers running. Its controls return to an explicit local Start.

All three tests pass on Robolectric API27 and34 (six cases). The broader offline
cohort passes1593 cases;11 existing status-dispatch/service-failure cases supply
additional fault counters. No failures, errors or skips occur. The final six UI
cases also pass after adding the active-replacement/stale-status assertions.
No new production defect was confirmed in this bounded scope; no production code
changes or physical root-cause claim accompany these test additions.

## Coverage and complexity

JaCoCo0.8.15 execution data from the1593-case cohort and11-case fault cohort is
combined by the native report task. Production classes are unchanged between
collections. The repository's authored-method attribution measures all211 scoped
production functions/callbacks individually at80–100% executable-line coverage.
`coverage-functions.json.gz` retains every function, method variant and source
hash. This covers twelve camera/audio/consent/consumer/connection source files;
it is not whole-project, native Android or Windows coverage. Raw execution data
remains outside the repository in the local evidence directory below.

Cached Kotlin PSI1.9.20 counts every new method, initializer and callback at<=3
cyclomatic complexity. These are local source counts, without a native SonarQube claim. Owner
clarification2026-10-21 accepts these measurements; T1244 verifies and closes
the former tool prerequisite. No metric suppression or coverage
exclusion is added.

## Resource ownership and bounds

Each fixture owns one coroutine scope, one original connection plus one latest
replacement, and at most two admitted consumers per camera/microphone/speaker
lane. Production lanes and claims retain original ownership through cancellation;
synthetic completion gates explicitly supply retirement, never elapsed time or
UI state. Unknown native completion remains owned. Current admission is revoked
before disposal; observations/prompts are retired, bindings shut down, every
original gate released and the scope cancelled/joined. Teardown asserts every
created consumer actually traversed its fixture retirement boundary, including
assertion-failure paths through `use`.

There are no native allocations in the synthetic workers. Thread-safe command/run
lists, atomic cancellation counts and volatile observed flags preserve cross-thread
test observations. Render callbacks borrow their original offer only within the
test lifetime. Leak freedom here assumes eventual explicit gate release and fair
coroutine execution; this does not establish Android device cleanup or unrestricted
native safety. Existing blocked/failed-native retirement tests remain unchanged.

## Offline execution

Gradle runs with `--offline --no-daemon`, cached Robolectric providers and explicit
`GRADLE_USER_HOME=/home/netto/.gradle` inside a separate user/network namespace
with loopback only. The initial root-mapped namespace lacked the wrapper cache;
the explicit cache path corrects runner setup. The final native coverage report
unions immutable `cohort.exec` with the11-case fault run. The existing namespace
JVM preference warning is not treated as a test or native-support result.

Local logs, raw execution data and merge configuration remain under
`/backups/disk2/blent-work/2026-10-21-offline-integration/T1242/`.
Compressed final records and source/class hashes accompany this document.
No download, upload, APK install, USB/ADB operation, installed-service change or
physical acceptance was performed. T855/T858/T859/T860/T1238 remain unfinished;
live checks stay paused.

</details>
