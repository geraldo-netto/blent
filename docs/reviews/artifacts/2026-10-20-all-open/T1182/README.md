# T1182: terminal camera callback authority

CameraBinding now composes the existing instance-local GenerationFence rather
than incrementing a signed counter on stop. Terminal capacity is checked before
permission prompts, service startup, target claims and native construction.
After permission and existing cleanup checks, reserve the exact grant before
background-service effects; recheck it before claim/admission. Stopping or ending
execution revokes publication without requiring a new number. No compatibility
constructor, alternate worker or legacy generation path is added.

The primitive initial seed directly sets the original pre-fix counter for hosted
boundary tests. Six original T1182 cases fail before repair: terminal permission
admission, replacement, stale completion/status, permission reuse, bounded starts
and invalid seeds. A further causal regression fails when synchronous background
withdrawal revokes the request but the outer workflow resumes and starts a worker.
The fix reserves before that effect and rejects its revoked identity afterward.
Original failing logs/XML and final passing cases are retained.

All104 cases across15 portable/Robolectric classes pass, including eight new
normal-suite camera cases and existing target, permission, Activity/service,
status dispatch, failure and bounded native-retirement regressions. The camera
oracle covers34 seeds ×40 start/stop pairs with independent BigInteger admission
counts; immediate retirement proves every started run settles. Held-retirement
cases require the original claim to remain unproved until its actual release;
late original status cannot publish before or after that fence. Permission
identity, exact external targets, manual stop and independent resource ownership
remain covered. GenerationFence's independent34 seeds ×81 traces ×4 operations
(11,016 transitions) now also checks available capacity at each state. Its eight
native JVM threads are joined. Negative construction tests own/cancel/join their
containing test scope even after partial construction fails.

Native JaCoCo0.8.15 reports100% executable-line coverage individually for all46
CameraBinding/GenerationFence JVM variants with authored line counters, including
separate callbacks and lined default bridges. Kotlin PSI1.9.20 hashes were verified
before the offline local source model ran:116 function/closure scores, maximum7.
This model is not native SonarQube certification; T1019 retains that gap. Source
hashes, full native XML, test counts and red/green output are retained here.

Validation: ./android/gradlew -p android --offline --no-daemon
-I /backups/disk2/blent-work/2026-10-19-open-items/t1150/offline.gradle
-I scripts/coverage/android.init.gradle testDebugUnitTest, with --tests for the15
classes in test-counts.json, followed by profileCoverage. Current source hashes
match the reported counters. Whitespace checks pass. No downloaded tools.

The serialized binding workflow remains authoritative owner of settings, source,
permission request and background UUID; GenerationFence owns only callback grants.
LatestDispatch marshals producer publications to that workflow. Available is a
preflight, never a grant or native-death proof. Permission can wait without
spending the last grant. Admission spends identity before service effects;
partial failure revokes without reusing it. ResourceLane retains original lazy
MediaConsumerRun, claim and NonCancellable cleanup until native retirement or an
exact no-start proof. Target/service failure retains original guard/service
obligations and cannot admit replacement. Numeric exhaustion never releases a
claim, cancels another binding or substitutes for original native evidence.

Conservation: every retained claim receives only its original RETIRED/NOT_STARTED
proof; while cleanup is held, no replacement opens and no proof is published.
Forbidden publication after stop, execution end or terminal denial is tested.
Conditional eventual cleanup requires the original worker's retire operation to
finish and the scheduler to run settlement; deliberate held/failing controls do
not pretend this assumption holds. Test rigs release gates, shut down bindings,
cancel and join their containing scope. No new queue, allocator or native reuse
path is introduced. Bounded JVM models/counters do not certify unrestricted
interleavings or physical camera safety. No device, live service, Windows or EVDI
backend was changed; unavailable device acceptance remains separately recorded.
