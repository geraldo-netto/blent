# T1438 — transient session identity failure becomes unsupported lock observation

An existing private logind owner returns a valid session path from
GetSessionByPID. Its first User-property query fails once. PowerFactory/Following
publish power=true, lock=false and display demand=true; after five configured
retry intervals, the identity getter has still been called only once. A direct
query to the same owner then returns UID 1000, matching the probe's actual UID,
and LockedHint=true. Blent's publication remains unchanged and permits display.
Exact output: `initial-session-failure.log`; public-API probe:
`initial_session_failure.rs`.

`native.rs::session_path` discards the User error as None, and Actor::logind then
declares lock capability absent. That actor has no session path whose lock signals
it could follow and no retry scheduled. Source establishes the missing recovery;
a finite observation delay alone does not prove permanent loss.

Severity medium, effort small. Retain conservative unknown observation and owned
retry for transient session-identity discovery failures, while preserving actual
wrong-UID/no-session and explicitly unsupported controls. Permanent regressions
must cover initial failure, held original native debt, fresh validated locked
state, later unlock, repeated failure and stop. No foreign UID may become an
admitted session merely because a query failed.

Protocol reference: systemd's [login1 documentation](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html)
defines the session lookup interface; its [public error names](https://github.com/systemd/systemd/blob/main/src/libsystemd/sd-bus/bus-common-errors.h)
distinguish NoSessionForPID and NoSuchSession from generic query failure. These
identifiers were checked to prepare explicit absence controls; no upstream
implementation is incorporated into the probe or project changes.

Run under a new `dbus-run-session`, using the same external rustc dependency
fingerprints/command pattern as `review.md`, with LLVM_PROFILE_FILE under
`/tmp/blent-lifecycle-round3-linux`. The probe redirects its system-bus address to
that private bus and never touches the real logind/session, tablet or desktop.
Following retires before the private server closes; no probe remains running.
`initial-session-complexity.json` records source/library hashes and all 11 authored
function scores, maximum 2. A permanent current-build regression is required
before the fix.

The same probe's `timeout` argument delays the first User reply by 200 ms under
its 100 ms acquisition deadline. The outer discovery timeout discards the known
logind actor, so power=false and lock=false remain published with display demand
true. The next same-owner User read succeeds and LockedHint=true; no replacement
was attempted. Exact output: `initial-session-timeout.log`. The same finite
acquisition deadline must therefore retain known failed identity observation,
rather than erasing it. Probe complexity after this additional fault mode: all 11
functions measured, maximum 3. The actor's original source completion and private
server are both settled before the process returns.
