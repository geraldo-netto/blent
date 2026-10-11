# T928: independently retained display startup recovery

Linux composition retains exact startup task identity and the original native
completion observer before exposing new session work. Panic, cancellation and
ordinary error cannot release unfinished native work. Portable recovery verifies
the exact capacity ticket; stale proof cannot retire a replacement. Partial startup
records each worker immediately in retained ownership. Both listener adapters
share pure work construction.

The shared primary reserves only one independent binding-task owner, reusing the
existing daemon session. It still starts with `BLENT_SESSION_LIMIT=1`; additional
sessions reserve new native capacity before task exposure. Capacity failure drops
only an unpublished reservation, never the existing primary runtime.

Permanent startup-panic regression fails before recovery, then passes unchanged.
A second regression catches the draft's incorrect extra primary session reservation
and passes unchanged after correction. Actual held native work prevents capacity
reuse; independent peers progress. Additional tests cover joined cancellation,
partial listener rollback, actual legacy stream/WebSocket service, terminal worker
outcomes, exhausted extra/shared-task capacity and 32 repeated stale proof checks.
The original T908 unregistered-startup quarantine regression remains unchanged.

Final fresh locked/offline host/common library and binary collection passes:
704 host library tests, 441 host binary tests and 303 common tests. Seven pre-existing
opt-in benchmark ignores remain; none added. All 72 scoped production functions
pass individual native 80% executable-line coverage, none missing. All 144 reviewed
production/test functions have cyclomatic complexity at most 8. Sources and the
collector were frozen from snapshot through native checks. ELF object fingerprints,
exact command, build inputs, counters and report hash are retained in final/.

Diagnostic evidence belongs to earlier draft source hashes, not final counters.
The earlier full integration run passed the host unit suites and non-RPM integration
checks but failed retained RPM tests t129/t302 because `rpmbuild` is absent (T833,
explicitly deferred). Its manually exported counters/attestation record failure;
this is not an all-integration passing claim. Earlier unit-only counters identified
legacy service/Wake coverage gaps, subsequently covered by substantive normal-suite
regressions; final evidence comes from a new exclusive target tree.

The immutable inventories include pre-existing unrelated TLS fixture edits in
host/src/broker/linux/server/tests/tls.rs. Those bytes remain unchanged and unstaged.
No downloads, installs, real USB/ADB, host services or physical tablet tests were
performed. Physical acceptance remains postponed; lost broker-registration IPC
recovery remains T914. No EVDI/dependency source changes or performance gain claimed.
