# T1343: observer recovery

A failed X11 observer now requests source replacement after publishing loss.
The portable follower stops and retires that source, then waits for its exact
native lifetime certificate before admitting another attempt. Replacement epochs
must increase; each source starts with its own capabilities and observations.

One attempt admission exists per follower. Its lifetime retains all original
creator/task/child certificates; cancellation cannot recycle that admission.
The follower lifetime retains each attempt certificate through caller loss.
Completed certificates are pruned when the next certificate is registered.
Unknown native obligations remain owned; recovery waits for their actual
settlement. Eventual recovery assumes native cleanup and the replacement factory
can eventually complete. The configured observation deadline sets the delay
between settled attempts; it does not establish retirement. Initial constructor
failure retains its existing terminal behavior. Invalid replacement identities
fail closed. Unsupported capability policy is unchanged.

Permanent regressions failed before the fix for missing native replacement
notification and missing recovery after original retirement. The same tests now
pass. Coverage execution passed 61 host and 37 common activity tests. All 13
changed/new production functions measured at least 93.9% executable-line
coverage individually. The project tree-sitter cyclomatic checker measured the
changed Rust scope with no function above nine. A 256-attempt certificate test
also verifies bounded retention without dropping live debt.

Evidence is in [the artifact directory](artifacts/2026-11-12-observer-recovery/).
These are automated adapter and orchestration results. Visible tablet frame and
reconnect acceptance remains T1340.
