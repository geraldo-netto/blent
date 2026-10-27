# T1183: terminal independent audio generations

Microphone and speaker use independent nonwrapping callback grants. Boundary and held-cleanup tests preserve direction isolation; exhausting one direction cannot revive or release another's work.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Each microphone/speaker AudioBinding now owns its own GenerationFence. Stop,
configure and execution-end revoke original callback authority without numeric
increments. Terminal invitations are rejected before permission prompts or pending
consent. External targets check capacity before admission, and launch acquires the
exact grant before retain/service/run effects. Service callbacks can revoke it;
rechecking afterward prevents the outer request from resuming native admission.
Completion revokes before original target/background cleanup. Current local
consent, save preferences and direction separation remain intact. No compatibility
counter, alternate constructor or fallback worker is retained.

The primitive seed sets the actual original counter before the fix. Seven
permanent normal-suite regressions fail first, then pass: terminal permission,
replacement/configure, late execution-ended status, stale original consent,
reentrant service withdrawal, wider-integer admission oracle and invalid seeds.
An independent speaker-isolation control stays passing. The initial draft also
exposed fixture close order failure; retain that original log/XML separately.
After fixing draft teardown, the red run has only the seven real audio failures
plus T1200's independently reproduced camera fixture failure. T1200 is resolved
in its own commit. Final scope adds terminal external-target rejection before any
claim or service and retains all original assertions.

All109 cases across16 portable/Robolectric classes pass, including nine audio
boundary cases, nine camera fixture/generation cases, existing permission,
Activity/service, target/lazy-run, dispatch, failure and retirement tests. The
independent BigInteger oracle checks34 seeds ×20 start/stop pairs ×2 directions
(1,360 admission attempts). Every immediate-cleanup run must retire. Held original
claims remain unproved until actual native cleanup; configuration saves cannot
release them or revive callbacks. Exhausting the microphone preserves the
speaker's original worker/service and publication authority. The shared fence's
permanent bounded/concurrent tests also pass.

JaCoCo0.8.15 reports100% individual executable-line coverage for all41 AudioBinding
JVM variants with authored line counters, including callbacks and lined default
bridges. Cached, hash-verified Kotlin PSI1.9.20 reports160 source function/closure
scores across changed production/new tests, maximum6. That local model is not
native SonarQube certification; T1019 retains the missing analyzer evidence.
Raw native XML, test counts, source fingerprints and red/green logs are retained.

Validation: ./android/gradlew -p android --offline --no-daemon
-I /backups/disk2/blent-work/2026-10-19-open-items/t1150/offline.gradle
-I scripts/coverage/android.init.gradle testDebugUnitTest, using --tests for the16
classes in test-counts.json, followed by profileCoverage. Source hashes match final
counters. Whitespace checks pass; no tool transfer, APK install or physical audio.

The serialized binding owns preferences, exact local permission request, target,
service UUID and ResourceLane. Fence grants are only publication/admission
identities, never native-death evidence. A grant is spent before effects and never
reused after partial failure. Each original claim is handed to one lazy
MediaConsumerRun and its retirement callback. Before run construction failure
returns its exact NOT_STARTED proof. A created worker remains owned by the lane
and NonCancellable guard through real retirement, including stop, owner loss,
service failure and exhaustion. Failed/stuck cleanup retains its original native
obligation; denied replacements cannot borrow another direction's resources.

Conservation and forbidden-state checks: each admitted original claim has only
its original no-start/retirement settlement; held cleanup publishes no release
proof or fresh native worker. Late original status/completion cannot publish after
revocation. Conditional eventual cleanup assumes controlled gates/native retire
and scheduler progress; deliberately held/failed controls do not assume these
have occurred. Rigs cut pending admission before releasing held workers, then
shut down, cancel and join original scopes. No new retaining cycle, queue,
allocator or native reuse mechanism is introduced. Bounded JVM evidence does not
certify unrestricted interleavings or physical acoustic behavior. Existing
platform/device blocks remain; Windows and EVDI backends were not changed.

</details>
