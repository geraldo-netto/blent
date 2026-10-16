# T1034: isolate native idle file effects

Idle lease creation and file effects now run on one admitted native thread.
The existing portable policy remains authoritative; Linux file access remains in
its adapter. Revocation sets one atomic stop and coalesced wake without touching
the file mutex. Dedicated native ownership avoids tying stuck file effects to
Tokio's blocking-pool shutdown. Shared work admission was extracted from the
concrete Tokio task owner into an independent boundary; both adapters retain the
same configured BLENT_TASK_LIMIT budget (default 64, valid 1..4096).

Encoder startup asynchronously awaits lease readiness after registering its
original native creator proof. The native owner retains its single lease before
exposing a queued view. Startup/handle field lifetimes drop views before original
worker retirement. Cancellation may precede later lease creation; the original
thread and retained dependency registry remain owned. Actual join/TLS teardown
and final lease destruction use the established isolated retirement helpers.
Unknown/stuck cleanup keeps original capacity and cannot grant replacement.
Failed, panicked or already-revoked startup remains explicit and fail-safe.

Six new permanent regressions cover held-file-lock destruction, control-runtime
progress during file effects, completed-but-unobserved queued startup, canceled
late creation, pre-revoked creation and native constructor panic. The two primary
regressions were added first and failed at about 400 ms caller/runtime blocking.
The same assertions now require and pass <100 ms control progress while retaining
original cleanup. Removing queued-lease retention separately reproduces a
400 ms startup destruction block; restoring it passes the retained oracle.

Existing cadence, invalid path/value, bounded history, viewer change, timestamp
and successor-token assertions remain unchanged. Native completion barriers now
replace assumptions that a Tokio yield also schedules a separate native thread.
Diagnostic pre-barrier failures are retained separately; they contribute no final
native counters and no assertion was removed, skipped or weakened.

Fresh locked offline suites pass 768 library and 478 CLI binary tests, with seven
pre-existing benchmark ignores unchanged. Installed SDK validation passes all 59
capture and nine isolated-encoder tests. Seven contract/portability checks pass.
Matching source/native object attestations and raw LCOV cover all 84 scoped
production functions individually at >=80%. The source audit covers 142
production/test functions, maximum complexity 7, using unchanged repository
Sonar-rule counting and cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2. Native
SonarQube remains unavailable under T1019; the source audit is not certification.

```sh
python3 /tmp/blent-t1034-evidence/scoped_rust.py coverage-cli-final
PATH=/tmp/blent-t1034-evidence/sdk-tools:$PATH \
  python3 /tmp/blent-t1034-evidence/scoped_sdk.py coverage-sdk-ready
python3 /tmp/blent-t1034-evidence/merge.py
```

Native objects remain in the corresponding `/tmp/blent-t1034-evidence` target
trees. No download, installation, upload, push or live device/display change
occurred. These native ownership tests do not qualify physical power/display
behavior; activity source/composition remains T1027/T1015 and physical T914.
