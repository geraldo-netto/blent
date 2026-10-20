# T1171 — fixture-owned Compose platform sandboxes

Broad offline Gradle runs reproduced existing pairing/preference/saved-browser
cases failing in Compose's initial runUntilIdle before their bodies. Repeated JVM
stacks and bounded read-only queue probes found empty current main queues. The
cached 1.6.3 test bytecode places that wait before test registry/body execution.
An isolated fresh compiler output reproduced the same failure; the recorded
incremental/fresh bytecode differences do not establish its cause. Direct ordered
JUnitCore controls initially passed; their success did not dismiss the broad
normal-run counterexample. No private Compose field is claimed as the cause.

ComposeFixtureRunner derives the existing Robolectric 4.17 instrumentation
configuration and includes the exact fixture class identity in its sandbox key,
with Compose platform instrumentation. Each fixture/SDK gets its own platform
context; Activity teardown cannot lend cached dispatchers to another fixture.
The stock instrumentation policy excludes @RunWith classes, so that identity key
does not transform production application classes or change their native counter
identity. The override and Builder API were verified in installed library
bytecode; no tool or dependency was downloaded.

The runner owns only its immutable fixture Class reference while JUnit owns the
runner. Robolectric remains the owner of cached SDK sandboxes and original SDK
executors; the adapter neither resets framework fields nor invents a native
release fence. Existing fixture Activity/worker/job cleanup remains responsible
for original settlement. Class keys prevent cross-fixture retargeting. Selected
SDKs and fixture classes form a finite local test domain; the broad validation
used an explicitly funded 2 GiB test heap. Framework caches and hosted tests do
not establish unrestricted native memory safety or device acceptance.

Existing UI/Activity fixtures change runner metadata/imports only; assertions,
method bodies and deadlines remain intact. Permanent ordered SDK27/34 checks run
actual Activity/UI preludes and require execution of the original pairing oracle.
The later T1169 integration extends these with the retained exact 69-class prefix.
Its replay exposed separate retired display-history retention, fixed by T1172;
that failure was not an idling failure and was not suppressed. Focused current
ordered/controls runs pass, and the broad current setup executes all changed UI
fixtures without the prior initial-idling failures. Final joined native counters
and the saved-browser cancellation regressions remain T1169 acceptance.

Three obsolete pre-fix JVMs were stopped after repeated setup starvation and
replacement by the passing scoped configuration. Their logs are explicitly
incomplete failed runs, not qualification. Exact original PID/parent/start identity
was checked, child absence verified, stop sent through pidfds, actual process
exit observed, handles closed and Gradle parents joined. They were superseded by
source/configuration changes, not cut off for an elapsed-time budget. Read-only
agents retained no callback and mutated no queue; writers/attachments returned
before original JVM exit. Probe sources and uncertainty remain in the record.

Cached Kotlin 1.9.20 PSI measures the new runner/order helpers with maximum 5;
native SonarQube remains T1019. No production function changed, no coverage
exclusion was added, and no network/device/install/push occurred. Raw evidence:
`/backups/disk2/blent-work/2026-10-20-open-items/t1171`.
