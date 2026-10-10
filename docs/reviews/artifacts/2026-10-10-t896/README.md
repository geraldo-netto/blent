# T896 — authenticated presentation follows completed setup

The Android setup owner now publishes AUTHENTICATED exactly once, after TLS peer
proof, durable trust publication, window validation and retained I/O installation.
Native deadlines remain bounded during proof, storage and negotiation. A failed
or cancelled setup cannot publish successful authentication before its checks.

Three permanent regressions failed before the fix: wrong TLS proof, pending
storage/I/O, and rejected negotiated capacity. The same tests now pass and the
success case asserts exactly one authenticated event after readiness.

Offline connection suite: 524 tests passed, none skipped. All 41 inventoried
functions and all executable raw JVM methods in ConnectionSetupOwner meet the
80% individual line threshold; complexity remains at most 9. Counters, source
inventory, results and red/green logs are retained. Physical acceptance remains
T855/T846.
