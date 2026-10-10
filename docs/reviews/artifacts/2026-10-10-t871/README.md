# T871 local registration and endpoint ownership

Implemented portable bounded BLRG registration/replies and the Linux same-user
SOCK_SEQPACKET adapter. Native listener capabilities travel through SCM_RIGHTS;
ports come from exclusive IPv4 localhost TCP listeners. RouteClaims retain the
listeners through caller death and release only after explicit valid retirement.
Registration is local reservation, not remote media admission. Production owner
election/private directory/socket cleanup remain T853; link dispatch remains T872.

Offline validation: 329 common tests, eight native broker tests, two cached-SDK
raw-socket tests and the retained malformed-descriptor capture regression pass.
The descriptor helper also has a passing ownership compile-fail doctest. All 57
scoped production functions pass the per-function 80% LLVM gate. Changed capture
wrappers owned/send_bytes/receive each have 100% native SDK counters; unchanged
capture functions were not all exercised by this narrow run. Ninety-one scoped
production/test functions have cyclomatic complexity at most 9. These are scoped
checks, not a whole-project coverage or complexity claim.

Permanent tests cover bounded invalid packet fields, exact handle counts,
truncation/unknown ancillary records and descriptor cleanup, peer UID rejection,
independent clients, kernel-derived endpoints, duplicate/shared/foreign listeners,
stale retirement and ownership retention. The packet send path uses fixed stack
ancillary storage and the original borrowed descriptor without duplication or a
per-send heap allocation; no throughput improvement is claimed without a benchmark.

Three initial-draft defects have retained fail-before-fix evidence: failed
retirement dropping its pinned listener; accepting SO_REUSEPORT listeners; and a
safe raw-descriptor adoption API permitting duplicate ownership. Their permanent
regressions pass after the fixes. Kernel-generated ancillary headers are parsed
only after recvmsg; every delivered descriptor is adopted before rejecting unknown
control records or truncation.

Native SDK tests used the installed Freedesktop SDK with network unshared and
local Cargo caches. No downloads, installations, service restarts or live USB mode
changes occurred. Display/touch remains the next user-visible milestone, followed
by webcam; this commit does not establish production ADB-free media support.
