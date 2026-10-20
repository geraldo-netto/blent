# T1204: scoped camera cutoff without replacement invitations

The original probe stop operation was extracted unchanged before repair. Its
permanent real-command regression fails on the actual lensless invitation with
fresh token and port1. Five Android receiver/held-native/permission regressions
also fail before repair (10 API variants). An initial fixture compile error from
a missing background-service constructor argument is retained separately; the
corrected regression run fails behavior assertions, not compilation.

The current portable Request validates and borrows the original credential/port.
The native probe sends CAMERA_STOP with expected_token/expected_port over stdin,
requires exact acknowledgement and command success, and retains the same native
command owners. Android validates the independent key and atomically compares/
withdraws the one authoritative invitation. Valid stale stops acknowledge a safe
no-op. Unknown/malformed actions cannot fall through to start, and lensless ADB
wire invitations reject. Internal nullable local-lens offers remain supported.
Current callers, shared JSON fixture, docs and old positive fixtures are updated;
there is no legacy stop invitation or compatibility fallback.

121 targeted Rust cases pass, including the actual probe compilation and original
retirement regression. The instrumented camera/key/stop selection passes too.
68 Android cases pass across API27/34, including existing native-generation and
explicit local-lens regressions. The independent finite key model checks all
2401 four-operation traces (9604 transitions per API) over three distinct keys,
publication, clear and stale/current/replayed stop. Boundary tests cover port0,
1,65535,65536/integer extrema, credential lengths0..128, ASCII0..127 and Unicode,
malformed/native command failures and exact acknowledgements. No test is skipped,
removed or weakened. Formatting/whitespace checks pass.

All five changed/new maintained Rust functions have100% individual native LLVM
line coverage; all12 Android method/callback variants have100% JaCoCo0.8.15
coverage. Exact hashes, raw LCOV/JaCoCo, per-method counters and red/green logs
are retained. Rustc1.98.1/cargo-llvm-cov0.8.7. Cached Kotlin PSI1.9.20 scores are
<=9 for every examined function; native Sonar measurement remains T1019. Manual
Rust source estimates are<=7, including tests. Probe code keeps its established
benchmark coverage exemption; its correctness regressions remain permanent.

One immutable validated key grants logical cutoff only. The Rust key borrows
its original credential within the awaiting caller; getters retain no state.
The receiver's compareAndSet cannot withdraw a different current key. Android
camera/codec owners retain original cleanup after withdrawal; processed receipt
is not native death. The held-worker test requires replacement to remain pending
until original native retirement, rejects stale status, and then permits current
replacement. Old permission callbacks cannot grant a new pending request. Test
fixtures cut admission before releasing held gates, then cancel/join their owning
scope and restore the prior global endpoint. Commands use existing bounded input/
output/admission/reaper ownership; only validated hex/typed ports enter the shell.
The finite model is sequential evidence around an atomic library operation, not
an unrestricted concurrent/native proof. Cleanup assumes original exit/reaper/
scheduler progress. No physical ADB/tablet/service, Windows/EVDI or network action
occurred. Native mapping-removal safety remains blocked under T1090; this endpoint
cutoff does not resolve that separate ownership contract or physical acceptance.
