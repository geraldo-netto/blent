# T1248 joined admission waits for its exact prepared generation

`ConnectionJoinedAudioFixture.admitPrepared` waited for any historical Prepared
record with the requested resource. After an old speaker had retired, that old
record satisfied the next generation's wait. The fixture could send Commit
before the new original lease was prepared, causing its actor to fail with
`Check failed.` and eventually ending the test with an admission wait failure.
The actor-state trace, rather than timeout alone, identifies the counterexample.

The permanent normal-suite regression performs complete original speaker
retirement, replaces its channel generation and admits a fresh speaker four
times on each SDK. Both API27/API34 cases fail before repair; `red.xml.gz` and
`red.log.gz` retain the original failure and native/management diagnostics.

The helper now captures the selected original lease before waiting, requires
`Prepared(lease.key)` exactly, then waits for a route matching that original
lease. Both regression cases and all eight repeated workflows pass afterward.
T1245's rendered fresh-generation workflow also reaches real playback after this
correction. This is a fixture bug; no production transport change is made.

The wait borrows the original lease, without native allocation or new mutable
state. No unrelated readiness record can sponsor admission or release. Existing
fixture cleanup retains original bindings/endpoints and controlled native workers
until their separate completion boundaries, releases gates in `finally`, and
closes each shared connection after its audio owner. Tests use private Binder,
real loopback sockets and synthetic SDK/devices/process-death notifications;
they provide no physical or kernel process-death proof.

Kotlin PSI source estimates (maximum3) and exact source/class hashes are
retained. Native SonarQube remains unavailable under T1244. Production coverage is
unchanged by this test-only fix. All commands run offline with cached tools in
isolated network namespaces. No physical checks, installation, transfer, push or
EVDI work occur; the owner's live-check pause remains intact.
