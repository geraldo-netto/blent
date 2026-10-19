# T1091: wake the original Android local resource actor

The production AndroidConnectionWorker supplies its original conflated wake
channel through tabletConnectionRun into local endpoint construction. Both the
retained bind/accept/close workers and StreamConnectionIo read/write/flush workers
receive that callback. Custom test/runtime factories retain their explicit
construction boundary. The runner composition requires a wake argument, preventing
silent omission there. Existing timer frequency, absolute deadlines, protocol
policy and all native retirement predicates are unchanged. T1092/T918 retain
polling reduction and matched idle performance acceptance; no CPU/battery gain
or completed polling optimization is claimed here.

The permanent loopback regression advances live local resources only after a
native hint, exercising listener preparation, acceptance and duplex stream work.
Before wiring, the original implementation fails with "original native completion
did not wake its resource owner"; afterward the same case passes. Independent
production-runner composition and late-hint/retired-actor tests remain in the normal
suite. The exact native loans, flush-credit barriers and stuck-owner tests from
T854/T905/T906 are retained; a wake never substitutes for their completion proofs.
The loopback fixture polls only during unconditional cleanup, so a failing wake
assertion cannot strand its original sockets or threads. These hosted fixtures
use real JVM loopback workers and synthetic authenticated setup/transport; they
establish neither physical Android/USB behavior nor a production media acceptance.

Ownership stays with the existing connection actor and each original local
endpoint/I/O owner. One conflated channel admits at most one wake hint; the new
closures allocate no native resource or replacement authority. They remain
reachable while their original native owners retain them. Late sends after actor
shutdown safely fail on that same closed channel and cannot enter a replacement
actor. Stop requests and hints release no byte loans, worker admissions or native
handles; existing exact retirement checks remain mandatory. Conditional cleanup
still requires eventual native return/closure and normal GC after external owners
release references; an unknown/stuck original remains owned under existing
quarantine/process supervision. No unrestricted native leak-freedom claim follows
from hosted success or managed references.

A fresh clean offline native JaCoCo run passes all 2,237 cases with no failures,
errors or skips. The strict current coverage checker measures all six changed
production functions/callbacks independently at 100%, including both constructors
and new factory callbacks. Raw exec/XML, compiled-class hashes, frozen source
hashes and logs are retained. The source manifest was rewritten with identical
contents while extending the source-estimate scope after measurement; report
regeneration uses the same unchanged native classes/exec and source hashes.
All 12 architecture checks pass. Kotlin PSI 1.9.20 audits 133 functions/callbacks
across changed production/tests, maximum source estimate 6. These are repository
Sonar-style estimates; native SonarQube remains unavailable under T1019.

No network transfer, download, installation, host restart, tablet action or push.
