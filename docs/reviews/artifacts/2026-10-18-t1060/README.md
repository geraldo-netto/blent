# T1060: original Android foreground/native power ownership

The manifest Application strongly owns one power runtime for its process lifetime.
An Activity owns its opaque producer and policy observer; producer loss withdraws
that original claim without dropping an outstanding native handoff or quarantined
worker. The permanent Java-lifetime regression fails on SDK 27 and 34 with the
weak-registry control and passes with Application sponsorship. It checks retained
logical ownership, not native safety from garbage collection. The test Application
inherits this production owner and preserves its existing queue-drain teardown.

The runtime reuses the independent request channel, original WakeLease contract
and existing ResourceLane/AndroidWorkerRun process retirement boundary. One original
notice, one latest desired request, one retiring power worker plus one latest
replacement, and one original idle callback plus one latest target remain bounded.
A private :power service owns native CPU/Wi-Fi work on IO threads. StreamingService
owns notification/routing only. Its exact pending Binder capability is checked
before promotion; old producers, duplicate envelopes and late callbacks cannot
retire or acknowledge a replacement. Confirmed rejection and ambiguous submission
remain distinct; uncertain queue/cancellation outcomes retain their original slot.

Native leases are installed before potentially throwing acquisition. Cleanup
attempts both original handles, retains failures and requires normal original API
release or the containing original process fence before reuse. Actual matching
readiness enables the original absolute five-second Wi-Fi idle grace. USB/pause
bypass grace; repeated idle does not extend it. Original CPU duration remains four
hours; later current policy may renew through predecessor retirement. Worker loss
cuts off its original scope before UI reporting and cannot automatically reacquire.
The [contract](../../../display-wake-policy.md#foreground-native-service-ownership)
records allocation/admission, borrow/cutoff, settlement/reuse, conservation and
conditional eventual cleanup assumptions.

Permanent regressions were observed failing before fixes on SDK 27 and 34:
paused native CPU retention, grant-then-acquire failure losing cleanup ownership,
CPU release failure skipping Wi-Fi, stale envelope promotion revoking newer power,
ownerless stale replay retaining a notification, and weak application sponsorship.
The queued invalid-clock regression also fails before containment. Retained tests
cover strict task/event schemas and bounded malformed/truncated values, all native
resource combinations, old/foreign owners, actual readiness, late creators, held
original cleanup, known rejection, enqueue-then-throw, unknown cancellation, elapsed
and invalid/overflowing clocks, CPU renewal, default component creation and existing
Activity cleanup faults. Joined default IPC tests execute real local adapters and
withhold actual original retirement before simulated Binder death/replacement.
The separately committed T1065 bridge and T1066 audio-fixture regressions remain.

Final offline ordinary suite: **2,137 tests, zero failures/errors/skips**. Native
JaCoCo 0.8.15 measures all **141** maintained authored production methods separately
at **91.7% or higher**. Pinned Kotlin PSI 1.9.20 inventories changed functions,
including compiler-inlined bodies in their owner and independent callback counters;
no new exclusions. All source/vector hashes, 147 compiled class hashes, input/tool
identities, raw execution/XML counters and per-method reports are retained. Earlier
coverage gaps and the initial test-Application mismatch remain nonpassing evidence.
Cached source audit covers **681** production/test functions, maximum estimate 8;
this is not native SonarQube certification (T1019). Twelve architecture/contract/
portability checks pass.

Selected cached, instrumented SDK 27/34 original framework method bodies show local
held-state updates after synchronous native release calls; their JAR hashes and
bytecode are retained. This does not prove unrestricted system-server/kernel token
cleanup timing, physical process separation, device wake state or battery savings.
OS process/token cleanup and scheduler/delivery progress remain explicit assumptions;
unknown obligations keep their original Application/process sponsor. Physical
host/tablet evidence remains T914. Current Activity still publishes Waiting until
T1067/T1061 source/window integration; joined acceptance remains T1058/T1011.
No network transfer, installation, push or live device interruption occurred.
