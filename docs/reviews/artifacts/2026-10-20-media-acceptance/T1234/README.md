# T1234: bounded media approval budget

Media approval defaults to the shared 120-second admission contract, with startup and cleanup budgets kept separate. Boundary tests and a physical approval after 35 seconds passed; full media acceptance remained separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Actual requests disappeared after30s while the independent broker/resource
contract defaults to120s. The native registration client now derives its default
from AdmissionTimeout and validates its1–600000ms whole-millisecond domain.
Startup remains10s, cleanup5s; both retain their independent60s maximum. The
first selected client/broker/tablet deadline still wins. Broker configuration
alone does not extend a client's chosen admission budget. No timer is removed.

Two permanent regressions fail before the fix: literal120s default and admitted
resource-domain settings (the original rejects61000ms). Both pass afterward.
The existing invalid61s assertion remains for startup/cleanup; admission now has
explicit valid61s and invalid zero/submillisecond/above600s/overflow checks.
T1228's original timeout/Unregister/Retired regression is unchanged. The normal
native cohort passes23 cases, including rejection, blocked send, foreign proof,
unknown remote ownership, native starter stalls and broker loss. A new virtual
clock check accepts a31s wait with default settings and retains the old30s
budget as a deliberately faulty control. Raw native LLVM counters measure both
changed production functions at100% (new9/9, validate11/11). Exact executable,
source identities and red/green logs are retained. Rustc1.98.1/cargo-llvm-cov0.8.7;
complexity numbers are source estimates, not unavailable native Sonar evidence.

The rebuilt production CLI/GUI use the new default. On the actual API36 tablet,
the owner waited for an instruction issued more than35s after the camera offer,
then pressed Android Start. Native USB admission and capture succeeded:12 actual
640×360 V4L2 frames have11 distinct hashes. Image content is not retained.
Earlier native XML independently shows the same pending Front request after the
old30s window. Debugging-off and full media acceptance remain separate parents.

Settings owns bounded values and launch paths; validation performs no native
admission. The unchanged16-slot factory funds original threads, listener copies,
models and broker lifetime observers. Longer permitted approval never authorizes
replacement or release. Timeout still revokes local authority and uses the
original IPC cleanup deadline; actual original Unregister/Retired and native
thread exit remain necessary. Missing/late proof stays owned and funded.
Conditional eventual cleanup requires original proof or original broker exit
and actual native return. These bounded fixtures are not unrestricted leak or
platform qualification proofs.

</details>
