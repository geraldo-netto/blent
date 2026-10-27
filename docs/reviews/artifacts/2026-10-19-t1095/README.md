# T1095: local and runtime deadline aggregation

Local endpoints, media groups and runtimes expose existing deadlines while preserving unknown providers and outstanding consumer cleanup. All 130 focused cases passed; these metadata getters alone changed neither timers nor resource authority.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The optional independent deadline capability now reaches local endpoint owners,
producer/consumer frame clocks and pumps, native resource groups, shared consent-
controlled media ownership, typed camera/audio/speaker facades, fixed local slots,
local runs and runtime composition. Getters project original authoritative state;
no timer, budget transition, consent check, native cleanup predicate or native
admission/release behavior changes. Actor integration remains T1092, including
required asynchronous budget/completion wakeups. T918 retains matched performance
and allocation evaluation; no CPU/polling reduction is claimed by getters alone.

Local bind and admitted accept budgets remain distinct from idle accept/read.
Producer backpressure and borrowed consumer write/flush clocks retain their original
absolute budgets. Stopped owners report their existing retirement budgets until
already observed expiration, terminal quarantine or actual retirement. Group
aggregation preserves known deadlines alongside missing/uncertain providers;
actual retired participants need no more native budget. Shared media retains a
published consumer's separate retirement deadline after native group retirement.
Pending permission opens no endpoint and adds no local native budget; protocol
admission time remains separately visible through the transport. Runtime includes
only installed owners, preserving explicit factory uncertainty and unsupported
capabilities rather than inventing idle evidence.

Eleven permanent normal-suite cases cover these lifecycle/composition boundaries,
including native bind/accept/close workers, sticky close failure, partial producer/
consumer progress and flush, known/unknown native budgets, partial factory failure,
camera/speaker consumer completion, all three pending media directions, uncertain
native-retirement publication and runtime factory uncertainty. Initial missing
projections fail all ten initial cases. The close-failure fixture originally
waited on a sticky quarantine predicate during cleanup; it was corrected before
projection implementation to retain and independently join its two original
threads. Its corrected case still fails on unknown metadata, then passes with
implementation, while asserting that the sticky owner never becomes reusable.
The final eleven cases pass. A metadata-only fault dropping outstanding consumer
budgets fails the permanent camera/speaker case. Original source is restored and
verified before the final clean native collection. Raw evidence remains.

Authoritative ownership stays in existing native owners and consent/resource
controllers. Getters synchronously borrow state, under the existing endpoint lock
where native threads mutate it, and return immutable scalar snapshots. Group scans
are bounded to two channels and local scans to four slots. No new native handle,
worker, queue, admission, retained provider, mutable view or release/reuse path is
added. Diagnostic idle, expiration or known status never substitutes for original
native/consumer fences. Unknown publication/factory obligations remain quarantined;
conditional cleanup still requires the original workers, I/O and consumer evidence.
No unrestricted native leak-freedom claim follows from managed references or tests.

Tests reuse original bounded ownership fixtures; camera/speaker rigs become
internal test helpers rather than duplicate mutable implementations. Actual JVM
native-worker fixtures release original gates and join/observe their exact workers.
The sticky-failure case independently joins both original threads and settles its
fake backend while preserving production quarantine. Other uncertain factories and
publication failures are bounded simulations with no unreturned real native handle.
Existing selected loopback cases exercise actual owned JVM sockets; physical USB,
capture/playback hardware, process death and device lifecycle remain outside scope.

Fresh clean offline JaCoCo collection passes all 130 selected new/related resource,
runtime, framing and loopback cases, with no failures/errors/skips. All 23 changed
production functions/constructors measure 100% native line coverage individually;
the synchronized inline body remains within its enclosing getter. Raw XML/exec,
source/class hashes and strict current-checker results are retained. All 12
architecture checks pass. Cached pinned Kotlin PSI 1.9.20 audits 401 functions/
callbacks across changed production/tests, maximum repository Sonar-style estimate 8.
Native SonarQube remains unavailable under T1019. No broader suite repeat is claimed.

No network transfer, download, installation, restart, physical handling or push.

</details>
