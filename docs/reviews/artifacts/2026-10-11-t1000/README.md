# T1000: automated joined duplex isolation

One Android original connection joins microphone, speaker and display with
independent real private services/Binder, TCP, mux and media workers. API 27/34
permanent tests qualify both PCM directions together, either independent Stop,
pending microphone denial, native open failure/death, held noncancellable read,
responsive display revocation and clean generation replacement with fresh Start.
Exact sibling authority and native ownership are retained. Existing single-direction
regressions and bounded PCM/clock properties remain unchanged.

One private Linux mutual-TLS connection likewise joins both original registrations,
real loopback endpoints and independent synthetic native helpers. Both Stop orders
keep sibling PCM progressing through held remote retirement. Remote revocation and
actual registration-owner destruction remain separate; retained native completion
is required after the original owners are dropped. The shared peer retains all
T992/T999 single-direction assertions.

Final full Android suite: 1,949 tests, zero failures/errors/skips; all 2,741 production
methods pass individual native 80% executable-line coverage. Fresh Rust accessory:
46 host + 49 shared tests; default: 41 host + 49 shared. All 47 production audio
functions pass the same individual native gate. Changed authored fixture complexity:
174 functions, maximum 6. No production behavior changed in this item.

The first instrumented full Android run had one duplex speaker admission timeout.
The diagnostic full run passes with the same timeout and unchanged behavioral
assertions. Exact admission diagnostics are retained permanently; unexplained
intermittency remains Open T1004. Successful retry is not a claimed root-cause fix.
The first fixture trial also incorrectly expected registration-owner destruction
while still retaining that owner; corrected assertions independently require actual
remote revocation followed by original-owner destruction. These are test fixture
qualification steps, not production bug fixes.

Cached offline builds only. No downloads, real USB/ADB, installations, real host
services or tablet disruption. Native audio devices, USB authority and Android
process-death evidence are synthetic. Physical/acoustic duplex remains deferred
T859/T846; display/touch physical qualification T857 is not established here.

Sources were frozen before snapshots/builds/native checks; native inventories,
object attestations, LCOV, JaCoCo, function counters and hashes are compressed here.
All collections include the pre-existing uncommitted TLS fixture. This commit does
not modify or stage those unrelated edits.
