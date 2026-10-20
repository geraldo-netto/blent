# T1200: test fixture cutoff precedes native release

The new camera generation fixture released original held retirement gates before
shutting down. A queued replacement could then execute synchronously, append to
the workers list during iteration and throw ConcurrentModificationException,
skipping original containing-scope cancellation/join. This was discovered while
the analogous active audio draft exercised a deliberate failing control.

Permanent t1200FixtureCleanupCutsPendingAdmissionBeforeReleasingNativeGate first
fails with that exact exception. Its independent finally sponsor shuts down,
releases a stable snapshot, cancels and joins the original scope even when the
faulty close implementation fails. The repaired fixture shuts down first,
cutting off observer and pending resource admission before releasing the held
original worker; it then cancels/joins. The same regression now passes with all9
CameraGenerationTest cases. Red/green XML, red invocation log and source hash are
retained. Final invocation is the offline Android normal-suite run recorded with
T1183, including --tests com.blent.CameraGenerationTest; no device use.

Only test scaffolding changed. Native production counters are unaffected, and
no artificial production coverage requirement is added. Cached Kotlin PSI1.9.20
model included this source with the T1183 run: whole maintained test scope scores
<=6; native SonarQube remains T1019. The test is bounded to one held original and
one queued replacement. No native replacement may start during cutoff. Original
cleanup is joined before the test releases its scope. Eventual cleanup assumes
its controlled release gates and coroutine scheduler settle, which this test
explicitly supplies. Rejected/failing controls retain their emergency sponsor.
